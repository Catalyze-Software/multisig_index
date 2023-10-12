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
    write(dir.join(format!("boost.did")), __export_service()).expect("Write failed.");
}
