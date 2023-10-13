use std::cell::RefCell;

use candid::Principal;
use ic_cdk::id;
use ic_ledger_types::{transfer, AccountIdentifier, Memo, Subaccount, Tokens, TransferArgs};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    {DefaultMemoryImpl, StableBTreeMap},
};

use crate::rust_declarations::{
    cmc_service::{CmcService, NotifyTopUpArg, NotifyTopUpResult},
    types::MultisigData,
};

use super::ledger::Ledger;

type Memory = VirtualMemory<DefaultMemoryImpl>;

type GroupIdentifier = String;

pub static LEDGER_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
pub static CMC_CANISTER_ID: &str = "rkp4c-7iaaa-aaaaa-aaaca-cai";
pub static MEMO_TOP_UP_CANISTER: Memo = Memo(1347768404_u64);
pub static MEMO_CREATE_CANISTER: Memo = Memo(1095062083_u64);
pub static ICP_TRANSACTION_FEE: Tokens = Tokens::from_e8s(10000);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));


    pub static ENTRIES: RefCell<StableBTreeMap<GroupIdentifier, MultisigData, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
        )
    );

}

pub struct Store;

impl Store {
    pub fn get_cycles() -> u64 {
        ic_cdk::api::canister_balance()
    }

    pub async fn top_up_self(caller: Principal, blockheight: u64) -> Result<String, String> {
        // create principal from the canister ids
        let ledger_principal =
            Principal::from_text(LEDGER_CANISTER_ID).expect("invalid ledger principal");
        let cmc_principal = Principal::from_text(CMC_CANISTER_ID).expect("invalid cmc principal");

        // validate the transaction done to this canister and return the amount
        match Ledger::validate_transaction(caller, blockheight).await {
            Ok(amount) => {
                let ledger_args = TransferArgs {
                    memo: MEMO_TOP_UP_CANISTER,
                    amount,
                    fee: ICP_TRANSACTION_FEE,
                    from_subaccount: None,
                    to: AccountIdentifier::new(&cmc_principal, &Subaccount::from(id())),
                    created_at_time: None,
                };

                // transfer the received amount to the cmc canister
                let transfer_result = transfer(ledger_principal, ledger_args).await;
                match transfer_result {
                    Ok(transaction_result) => match transaction_result {
                        Ok(block_index) => {
                            let topup_args = NotifyTopUpArg {
                                block_index,
                                canister_id: id(),
                            };
                            let topup_result =
                                CmcService(cmc_principal).notify_top_up(topup_args).await;
                            match topup_result {
                                Ok((topup_resonse,)) => match topup_resonse {
                                    NotifyTopUpResult::Ok(cycles) => {
                                        Ok(format!("topped up with '{}' cycles", cycles))
                                    }
                                    NotifyTopUpResult::Err(err) => Err(format!("{:?}", err)),
                                },
                                Err((_, err)) => Err(err),
                            }
                        }
                        Err(err) => Err(err.to_string()),
                    },
                    Err((_, err)) => Err(err),
                }
            }
            Err(err) => Err(err),
        }
    }
}
