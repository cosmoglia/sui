// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use diesel::dsl::Limit;
use diesel::pg::Pg;
use diesel::query_builder::QueryFragment;
use diesel::query_dsl::methods::LimitDsl;
use diesel::result::Error as DieselError;
use diesel::QueryableByName;
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::methods::LoadQuery;
use diesel_async::RunQueryDsl;
use jsonrpsee::types::{error::INTERNAL_ERROR_CODE, ErrorObject};
use prometheus::Registry;
use sui_indexer_alt_metrics::db::DbConnectionStatsCollector;
use sui_indexer_alt_schema::objects::StoredObjVersion;
use sui_indexer_alt_schema::objects::StoredObject;
use sui_indexer_alt_schema::schema::obj_versions;
use sui_pg_db as db;
use sui_types::base_types::ObjectID;
use tracing::debug;

use crate::metrics::RpcMetrics;
use crate::query;
use crate::raw_query::RawQuery;

pub(crate) struct Connection<'p> {
    pub conn: db::Connection<'p>,
    metrics: Arc<RpcMetrics>,
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum DbError {
    #[error(transparent)]
    Create(anyhow::Error),

    #[error(transparent)]
    Connect(anyhow::Error),

    #[error(transparent)]
    RunQuery(#[from] DieselError),
}

/// This wrapper type exists to perform error conversion between the data fetching layer and the
/// RPC layer, metrics collection, and debug logging of database queries.
#[derive(Clone)]
pub(crate) struct Reader {
    db: db::Db,
    max_page_limit: usize,
    metrics: Arc<RpcMetrics>,
}

impl Reader {
    pub(crate) async fn new(
        db_args: db::DbArgs,
        metrics: Arc<RpcMetrics>,
        registry: &Registry,
        max_page_limit: usize,
    ) -> Result<Self, DbError> {
        let db = db::Db::for_read(db_args).await.map_err(DbError::Create)?;

        registry
            .register(Box::new(DbConnectionStatsCollector::new(
                Some("rpc_db"),
                db.clone(),
            )))
            .map_err(|e| DbError::Create(e.into()))?;

        Ok(Self {
            db,
            max_page_limit,
            metrics,
        })
    }

    /// Cap the given page limit to the maximum page limit.
    pub(crate) fn cap_page_limit(&self, limit: Option<usize>) -> usize {
        std::cmp::min(limit.unwrap_or(self.max_page_limit), self.max_page_limit)
    }

    pub(crate) async fn connect(&self) -> Result<Connection<'_>, DbError> {
        Ok(Connection::new(
            self.db.connect().await.map_err(DbError::Connect)?,
            self.metrics.clone(),
        ))
    }
}

impl From<DbError> for ErrorObject<'static> {
    fn from(err: DbError) -> Self {
        match err {
            DbError::Create(err) => {
                ErrorObject::owned(INTERNAL_ERROR_CODE, err.to_string(), None::<()>)
            }

            DbError::Connect(err) => {
                ErrorObject::owned(INTERNAL_ERROR_CODE, err.to_string(), None::<()>)
            }

            DbError::RunQuery(err) => {
                ErrorObject::owned(INTERNAL_ERROR_CODE, err.to_string(), None::<()>)
            }
        }
    }
}

impl<'p> Connection<'p> {
    pub(crate) fn new(conn: db::Connection<'p>, metrics: Arc<RpcMetrics>) -> Self {
        Self { conn, metrics }
    }

    pub(crate) async fn first<Q, U>(&mut self, query: Q) -> Result<U, DbError>
    where
        U: Send,
        Q: RunQueryDsl<db::ManagedConnection> + 'static,
        Q: LimitDsl,
        Limit<Q>: LoadQuery<'static, db::ManagedConnection, U> + QueryFragment<Pg> + Send,
    {
        let query = query.limit(1);
        debug!("{}", diesel::debug_query(&query));

        let _guard = self.metrics.db_latency.start_timer();
        let res = query.get_result(&mut self.conn).await;

        if res.is_ok() {
            self.metrics.db_requests_succeeded.inc();
        } else {
            self.metrics.db_requests_failed.inc();
        }

        Ok(res?)
    }

    pub(crate) async fn results<'q, Q, U>(&mut self, query: Q) -> Result<Vec<U>, DbError>
    where
        U: Send + 'q,
        Q: RunQueryDsl<db::ManagedConnection> + 'q,
        Q: LoadQuery<'q, db::ManagedConnection, U> + QueryFragment<Pg> + Send,
    {
        debug!("{}", diesel::debug_query(&query));

        let _guard = self.metrics.db_latency.start_timer();
        let res = query.get_results(&mut self.conn).await;

        if res.is_ok() {
            self.metrics.db_requests_succeeded.inc();
        } else {
            self.metrics.db_requests_failed.inc();
        }

        Ok(res?)
    }

    pub(crate) async fn results_with_raw_query<U>(
        &mut self,
        query: RawQuery,
    ) -> Result<Vec<U>, DbError>
    where
        U: Send + QueryableByName<Pg> + 'static,
    {
        let query = query.into_boxed();

        println!("{}", diesel::debug_query(&query));

        let _guard = self.metrics.db_latency.start_timer();
        let res = query.get_results(&mut self.conn).await;

        if res.is_ok() {
            self.metrics.db_requests_succeeded.inc();
        } else {
            self.metrics.db_requests_failed.inc();
        }

        Ok(res?)
    }

    pub(crate) async fn load_latest_objects<'a>(
        &mut self,
        object_ids: Vec<ObjectID>,
    ) -> Result<Vec<StoredObject>, DbError> {
        let stored_objects_versions = self.load_latest_objects_versions(object_ids).await?;
        let objects_versions = stored_objects_versions
            .into_iter()
            .map(|v| (v.object_id, v.object_version))
            .collect();
        let objects = self.load_objects_at_versions(objects_versions).await?;
        Ok(objects)
    }

    pub(crate) async fn load_latest_objects_versions<'a>(
        &mut self,
        object_ids: Vec<ObjectID>,
    ) -> Result<Vec<StoredObjVersion>, DbError> {
        use obj_versions as o;
        if object_ids.is_empty() {
            return Ok(vec![]);
        }
        let object_ids = object_ids.iter().map(|o| o.to_vec()).collect::<Vec<_>>();

        let query = o::table
            .select(StoredObjVersion::as_select())
            .distinct_on(o::object_id)
            .order_by((o::object_id, o::object_version.desc()))
            .filter(o::object_id.eq_any(object_ids));

        let object_versions: Vec<StoredObjVersion> = self.results(query).await?;
        Ok(object_versions)
    }

    pub(crate) async fn load_objects_at_versions<'a>(
        &mut self,
        object_versions: Vec<(Vec<u8>, i64)>,
    ) -> Result<Vec<StoredObject>, DbError> {
        if object_versions.is_empty() {
            return Ok(vec![]);
        }
        let mut query = query!("SELECT * FROM kv_objects");
        for (object_id, object_version) in object_versions {
            query = query.or_filter(format!(
                "object_id = '\\x{}'::bytea AND object_version = {}",
                hex::encode(object_id),
                object_version
            ));
        }

        let objects: Vec<StoredObject> = self.results_with_raw_query(query).await?;
        Ok(objects)
    }
}
