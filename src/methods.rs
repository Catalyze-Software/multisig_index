use ic_cdk::{caller, query, update};

use crate::logic::store::Store;

#[query]
fn get_cycles() -> u64 {
    Store::get_cycles()
}

#[update]
async fn top_up_self(blockheight: u64) -> Result<String, String> {
    Store::top_up_self(caller(), blockheight).await
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
