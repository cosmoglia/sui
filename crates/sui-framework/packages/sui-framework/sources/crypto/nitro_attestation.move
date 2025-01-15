// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

module sui::nitro_attestation;

use sui::clock::{Self, Clock};

public struct NitroAttestationDocument has store, copy, drop  {
    module_id: vector<u8>,
    timestamp: u64,
    digest: vector<u8>,
    pcrs: vector<vector<u8>>,
    public_key: Option<vector<u8>>,
    user_data: Option<vector<u8>>,
    nonce: Option<vector<u8>>,
}

/// Internal native function
native fun verify_nitro_attestation_internal(
    attestation: &vector<u8>,
    current_timestamp: u64
): NitroAttestationDocument;

/// @param attestation: attesttaion documents bytes data. 
/// @param clock: the clock object.
///
/// Returns parsed NitroAttestationDocument after verifying the attestation.
public fun verify_nitro_attestation(
    attestation: &vector<u8>,
    clock: &Clock
): NitroAttestationDocument {
    verify_nitro_attestation_internal(attestation, clock::timestamp_ms(clock))
}

public fun get_module_id(attestation: &NitroAttestationDocument): vector<u8> {
    attestation.module_id
}

public fun get_timestamp(attestation: &NitroAttestationDocument): u64 {
    attestation.timestamp
}

public fun get_digest(attestation: &NitroAttestationDocument): vector<u8> {
    attestation.digest
}

public fun get_pcrs(attestation: &NitroAttestationDocument): vector<vector<u8>> {
    attestation.pcrs
}

public fun get_public_key(attestation: &NitroAttestationDocument): Option<vector<u8>> {
    attestation.public_key
}

public fun get_user_data(attestation: &NitroAttestationDocument): Option<vector<u8>> {
    attestation.user_data
}

public fun get_nonce(attestation: &NitroAttestationDocument): Option<vector<u8>> {
    attestation.nonce
}
