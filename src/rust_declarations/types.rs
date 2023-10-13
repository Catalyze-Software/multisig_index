use candid::{CandidType, Principal};
use serde::Deserialize;
use std::borrow::Cow;

use candid::{Decode, Encode};
use ic_stable_structures::{storable::Bound, Storable};

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct MultisigData {
    pub canister_id: Principal,
    pub group_identifier: Principal,
    pub created_by: Principal,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Storable for MultisigData {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}
