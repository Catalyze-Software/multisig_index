use std::cell::RefCell;

use candid::Principal;
use ic_cdk::{api::time, id};
use ic_ledger_types::{transfer, AccountIdentifier, Memo, Subaccount, Tokens, TransferArgs};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    {DefaultMemoryImpl, StableBTreeMap},
};

use crate::rust_declarations::{
    cmc_service::{CmcService, NotifyTopUpArg, NotifyTopUpResult},
    types::{MultisigData, TransactionData, TransactionStatus},
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

    pub static TOP_UP_TRANSACTIONS: RefCell<StableBTreeMap<u64, TransactionData, Memory>> = RefCell::new(
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

    pub fn get_transactions(status: Option<TransactionStatus>) -> Vec<TransactionData> {
        TOP_UP_TRANSACTIONS.with(|t| {
            t.borrow()
                .iter()
                .filter(|(_, v)| {
                    if let Some(_status) = status.clone() {
                        v.status == _status
                    } else {
                        true
                    }
                })
                .map(|(_, v)| v.clone())
                .collect()
        })
    }

    pub async fn top_up_self(caller: Principal, icp_block_index: u64) -> Result<String, String> {
        // create principal from the canister ids
        let ledger_principal =
            Principal::from_text(LEDGER_CANISTER_ID).expect("invalid ledger principal");
        let cmc_principal = Principal::from_text(CMC_CANISTER_ID).expect("invalid cmc principal");

        // validate the transaction done to this canister and return the amount
        match Ledger::validate_transaction(caller, icp_block_index).await {
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
                match transfer(ledger_principal, ledger_args).await {
                    Ok(transaction_result) => match transaction_result {
                        Ok(cmc_block_index) => {
                            let topup_args = NotifyTopUpArg {
                                block_index: cmc_block_index,
                                canister_id: id(),
                            };
                            let topup_result =
                                CmcService(cmc_principal).notify_top_up(topup_args).await;
                            match topup_result {
                                Ok((topup_resonse,)) => match topup_resonse {
                                    NotifyTopUpResult::Ok(cycles) => {
                                        TOP_UP_TRANSACTIONS.with(|t| {
                                            t.borrow_mut().insert(
                                                icp_block_index,
                                                TransactionData {
                                                    icp_transfer_block_index: icp_block_index,
                                                    cmc_transfer_block_index: Some(cmc_block_index),
                                                    icp_amount: Some(amount),
                                                    initialized_by: caller,
                                                    cycles_amount: Some(cycles.clone()),
                                                    created_at: time(),
                                                    error_message: None,
                                                    status: TransactionStatus::Success,
                                                },
                                            )
                                        });
                                        Ok(format!("topped up with '{}' cycles", cycles))
                                    }
                                    NotifyTopUpResult::Err(err) => {
                                        TOP_UP_TRANSACTIONS.with(|t| {
                                            t.borrow_mut().insert(
                                                icp_block_index,
                                                TransactionData {
                                                    icp_transfer_block_index: icp_block_index,
                                                    cmc_transfer_block_index: Some(cmc_block_index),
                                                    icp_amount: Some(amount),
                                                    initialized_by: caller,
                                                    cycles_amount: None,
                                                    created_at: time(),
                                                    error_message: Some(format!("{:?}", err)),
                                                    status: TransactionStatus::CycleTopupFailed,
                                                },
                                            )
                                        });
                                        Err(format!("{:?}", err))
                                    }
                                },
                                Err((_, err)) => {
                                    TOP_UP_TRANSACTIONS.with(|t| {
                                        t.borrow_mut().insert(
                                            icp_block_index,
                                            TransactionData {
                                                icp_transfer_block_index: icp_block_index,
                                                cmc_transfer_block_index: Some(cmc_block_index),
                                                icp_amount: Some(amount),
                                                initialized_by: caller,
                                                cycles_amount: None,
                                                created_at: time(),
                                                error_message: Some(err.clone()),
                                                status: TransactionStatus::CmcTransactionFailed,
                                            },
                                        )
                                    });

                                    Err(err)
                                }
                            }
                        }
                        Err(err) => {
                            TOP_UP_TRANSACTIONS.with(|t| {
                                t.borrow_mut().insert(
                                    icp_block_index,
                                    TransactionData {
                                        icp_transfer_block_index: icp_block_index,
                                        cmc_transfer_block_index: None,
                                        icp_amount: Some(amount),
                                        initialized_by: caller,
                                        cycles_amount: None,
                                        created_at: time(),
                                        error_message: Some(err.to_string()),
                                        status: TransactionStatus::CmcTransactionFailed,
                                    },
                                )
                            });
                            Err(err.to_string())
                        }
                    },
                    Err((_, err)) => {
                        TOP_UP_TRANSACTIONS.with(|t| {
                            t.borrow_mut().insert(
                                icp_block_index,
                                TransactionData {
                                    icp_transfer_block_index: icp_block_index,
                                    cmc_transfer_block_index: None,
                                    icp_amount: Some(amount),
                                    initialized_by: caller,
                                    cycles_amount: None,
                                    created_at: time(),
                                    error_message: Some(err.clone()),
                                    status: TransactionStatus::CmcTransactionFailed,
                                },
                            )
                        });
                        Err(err)
                    }
                }
            }
            Err(err) => {
                TOP_UP_TRANSACTIONS.with(|t| {
                    t.borrow_mut().insert(
                        icp_block_index,
                        TransactionData {
                            icp_transfer_block_index: icp_block_index,
                            cmc_transfer_block_index: None,
                            icp_amount: None,
                            initialized_by: caller,
                            cycles_amount: None,
                            created_at: time(),
                            error_message: Some(err.clone()),
                            status: TransactionStatus::IcpTransactionFailed,
                        },
                    )
                });
                Err(err)
            }
        }
    }
}
