use candid::Principal;
use ic_cdk::{caller, id, query, update};

use crate::{
    logic::store::Store,
    rust_declarations::types::{
        InitializeStatus, MultisigData, TransactionData, TransactionStatus,
    },
};

#[query]
fn get_cycles() -> u64 {
    Store::get_cycles()
}

#[update]
async fn get_cmc_icp_balance() -> Result<u64, String> {
    Store::get_icp_balance(id()).await
}

#[query]
async fn get_caller_local_balance() -> u64 {
    Store::get_caller_local_icp_balance(caller())
}

#[query]
async fn get_principal_local_balance(principal: Principal) -> u64 {
    Store::get_caller_local_icp_balance(principal)
}

#[query]
fn get_transactions(status: Option<TransactionStatus>) -> Vec<TransactionData> {
    Store::get_transactions(status)
}

#[query]
fn get_multisig_by_group_identifier(identifier: Principal) -> Option<MultisigData> {
    Store::get_multisig_by_group_identifier(identifier)
}

#[query]
fn get_multisigs() -> Vec<MultisigData> {
    Store::get_multisigs()
}

#[update]
async fn spawn_multisig(
    blockheight: u64,
    group_identifier: Principal, // can be an identifier
) -> Result<Principal, String> {
    Store::spawn_multisig(caller(), blockheight, group_identifier).await
}

#[query]
fn get_initialization_status(group_identifier: Principal) -> Option<InitializeStatus> {
    Store::get_initialization_status(group_identifier)
}

#[update]
async fn withdraw_balance() -> Result<(), String> {
    Store::withdraw_balance(caller()).await
}

// Method used to save the candid interface to a file
#[test]
pub fn candid() {
    use candid::export_service;
    use std::env;
    use std::fs::write;
    use std::path::PathBuf;
    export_service!();
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let dir = dir.parent().unwrap().join("candid");
    write(dir.join(format!("multisig_index.did")), __export_service()).expect("Write failed.");
}
