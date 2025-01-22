// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::rpc_module::RpcModule;
use crate::Reader;
use crate::{filter, query};
use diesel::{prelude::*, sql_query};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use move_core_types::language_storage::TypeTag;
use sui_indexer_alt_schema::objects::StoredObject;
use sui_indexer_alt_schema::schema::coin_balance_buckets;
use sui_json_rpc_types::{Coin, CoinPage};
use sui_open_rpc::Module;
use sui_open_rpc_macros::open_rpc;
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    gas_coin::GAS,
    object::Object,
};

#[open_rpc(namespace = "suix", tag = "Coin API")]
#[rpc(server, namespace = "suix")]
trait CoinApi {
    #[method(name = "getCoins")]
    async fn get_coins(
        &self,
        owner: SuiAddress,
        coin_type: Option<String>,
        cursor: Option<ObjectID>,
        limit: Option<usize>,
    ) -> RpcResult<CoinPage>;

    /// Return all Coin objects owned by an address.
    #[method(name = "getAllCoins")]
    async fn get_all_coins(
        &self,
        /// the owner's Sui address
        owner: SuiAddress,
        /// optional paging cursor
        cursor: Option<ObjectID>,
        /// maximum number of items per page
        limit: Option<usize>,
    ) -> RpcResult<CoinPage>;
}

pub(crate) struct CoinServer(pub Reader);

#[derive(QueryableByName, Selectable, Debug)]
#[diesel(table_name = coin_balance_buckets)]
struct IdCpBalance {
    object_id: Vec<u8>,
    #[allow(unused)]
    cp_sequence_number: i64,
    coin_balance_bucket: Option<i16>,
}

// Need this wrapper to avoid having to implement TryInto<Coin> in indexer alt schema crate.
struct StoredObjectWrapper(StoredObject);

#[async_trait::async_trait]
impl CoinApiServer for CoinServer {
    async fn get_coins(
        &self,
        owner: SuiAddress,
        coin_type: Option<String>,
        cursor: Option<ObjectID>,
        limit: Option<usize>,
    ) -> RpcResult<CoinPage> {
        let coin_struct_tag = if let Some(coin_type) = coin_type {
            sui_types::parse_sui_type_tag(&coin_type).map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(
                    1,
                    format!("Failed to parse coin type: {}", e),
                    None::<()>,
                )
            })?
        } else {
            GAS::type_tag()
        };
        self.get_coins_impl(owner, Some(coin_struct_tag), cursor, limit)
            .await
    }

    async fn get_all_coins(
        &self,
        owner: SuiAddress,
        cursor: Option<ObjectID>,
        limit: Option<usize>,
    ) -> RpcResult<CoinPage> {
        self.get_coins_impl(owner, None, cursor, limit).await
    }
}

impl CoinServer {
    async fn get_coins_impl(
        &self,
        owner: SuiAddress,
        coin_type_tag: Option<TypeTag>,
        cursor: Option<ObjectID>,
        limit: Option<usize>,
    ) -> RpcResult<CoinPage> {
        let mut conn = self.0.connect().await?;
        let limit = self.0.cap_page_limit(limit);

        // First load the coins' object ids.
        // TODO: optimize this query according to the indexes in coin_balance_buckets table.
        let mut filtered_rows = query!(
            "
            SELECT object_id, cp_sequence_number, coin_balance_bucket FROM coin_balance_buckets
        "
        );

        filtered_rows = filter!(
            filtered_rows,
            format!("owner_id = '\\x{}'::bytea", hex::encode(owner))
        );

        if let Some(coin_type_tag) = coin_type_tag {
            let serialized_coin_type = bcs::to_bytes(&coin_type_tag).map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(
                    1,
                    format!("Failed to serialize coin type: {}", e),
                    None::<()>,
                )
            })?;
            filtered_rows = filter!(
                filtered_rows,
                format!(
                    "coin_type = '\\x{}'::bytea",
                    hex::encode(serialized_coin_type)
                )
            );
        }

        if let Some(cursor) = cursor {
            // We need to find the balance of the cursor coin so that we know
            //how to filter the results according to their balances.
            let cursor_coin_query = query!(format!(
                "SELECT object_id, cp_sequence_number, coin_balance_bucket
                 FROM coin_balance_buckets
                 WHERE object_id = '\\x{}'::bytea
                 ORDER BY cp_sequence_number DESC
                 LIMIT 1;",
                hex::encode(cursor)
            ));

            let mut cursor_coins: Vec<IdCpBalance> =
                conn.results_with_raw_query(cursor_coin_query).await?;
            if cursor_coins.len() != 1 {
                return Err(jsonrpsee::types::ErrorObject::owned(
                    1,
                    "Cursor coin not found or found more than one",
                    None::<()>,
                ));
            }
            let cursor_coin = cursor_coins.remove(0);
            let Some(cursor_balance_bucket) = cursor_coin.coin_balance_bucket else {
                return Err(jsonrpsee::types::ErrorObject::owned(
                    1,
                    "Cursor coin was deleted",
                    None::<()>,
                ));
            };
            let cursor_coin_id = cursor_coin.object_id;
            filtered_rows = filter!(
                filtered_rows,
                format!(
                    "(coin_balance_bucket < {} OR (coin_balance_bucket = {} AND object_id > '\\x{}'::bytea))",
                    cursor_balance_bucket, cursor_balance_bucket, hex::encode(cursor_coin_id)
                )
            );
        }

        let (filtered_rows, _) = filtered_rows.finish();

        // Construct the final query to get the rows with the latest cp sequence number.
        let query = format!(
            "
            WITH filtered_rows AS (
                {filtered_rows}
            )
            SELECT
                f.cp_sequence_number,
                f.object_id,
                f.coin_balance_bucket
            FROM
                filtered_rows f
            LEFT JOIN
                coin_balance_buckets c
            ON
                c.object_id = f.object_id AND c.cp_sequence_number > f.cp_sequence_number
            WHERE
                c.object_id IS NULL
            ORDER BY
                f.coin_balance_bucket DESC,
                f.object_id ASC
            LIMIT {limit}
        ",
            limit = limit + 1 // Query one more result to determine if there is a next page.
        );

        let buckets: Vec<IdCpBalance> = conn.results(sql_query(query)).await?;

        let coin_ids: Vec<ObjectID> = buckets
            .into_iter()
            .filter_map(|bucket| ObjectID::from_bytes(&bucket.object_id).ok())
            .collect();

        // Then we load the actual objects from kv, to render the coins.
        let objects = conn.load_latest_objects(coin_ids.clone()).await?;

        // The resulting objects are no longer sorted according to original order so we sort them to match coin_ids order.
        let mut sorted_objects = Vec::with_capacity(objects.len());
        for id in coin_ids {
            if let Some(obj) = objects
                .iter()
                .find(|o| ObjectID::from_bytes(&o.object_id).map_or(false, |oid| oid == id))
            {
                sorted_objects.push(obj.clone());
            }
        }

        // Finally convert objects to coins.
        let mut coins: Vec<Coin> = sorted_objects
            .into_iter()
            .map(|o| StoredObjectWrapper(o).try_into())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                jsonrpsee::types::ErrorObject::owned(
                    1,
                    "Failed to convert object to coin",
                    None::<()>,
                )
            })?;

        // Deal with pagination if needed.
        let (next_cursor, has_next_page) = if coins.len() > limit {
            let last_coin = coins.remove(coins.len() - 1);
            (Some(last_coin.coin_object_id), true)
        } else {
            (None, false)
        };

        Ok(CoinPage {
            data: coins,
            next_cursor,
            has_next_page,
        })
    }
}

impl RpcModule for CoinServer {
    fn schema(&self) -> Module {
        CoinApiOpenRpc::module_doc()
    }

    fn into_impl(self) -> jsonrpsee::RpcModule<Self> {
        self.into_rpc()
    }
}

impl TryInto<Coin> for StoredObjectWrapper {
    type Error = ();
    fn try_into(self) -> Result<Coin, Self::Error> {
        let object: Object = bcs::from_bytes(&self.0.serialized_object.unwrap()).unwrap();
        let coin_object_id = object.id();
        let digest = object.digest();
        let version = object.version();
        let previous_transaction = object.as_inner().previous_transaction;
        let coin = object.as_coin_maybe().unwrap();
        let coin_type = object
            .coin_type_maybe()
            .unwrap()
            .to_canonical_string(/* with_prefix */ true);
        Ok(Coin {
            coin_type,
            coin_object_id,
            version,
            digest,
            balance: coin.balance.value(),
            previous_transaction,
        })
    }
}
