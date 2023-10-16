use candid::Principal;
use ic_cdk::{caller, id, query, update};

use crate::{
    logic::store::Store,
    rust_declarations::types::{TransactionData, TransactionStatus},
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
fn get_transactions(status: Option<TransactionStatus>) -> Vec<TransactionData> {
    Store::get_transactions(status)
}

#[update]
async fn spawn_multisig(blockheight: u64) -> Result<Principal, String> {
    Store::spawn_multisig(caller(), blockheight).await
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
