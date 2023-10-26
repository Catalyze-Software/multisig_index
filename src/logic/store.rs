use std::{cell::RefCell, convert::TryFrom};

use candid::{Encode, Nat, Principal};
use ic_cdk::{
    api::{
        management_canister::{
            main::{
                create_canister, install_code, CanisterInstallMode, CreateCanisterArgument,
                InstallCodeArgument,
            },
            provisional::CanisterSettings,
        },
        time,
    },
    id,
};
use ic_ledger_types::{
    account_balance, AccountBalanceArgs, AccountIdentifier, Memo, Subaccount, Tokens, TransferArgs,
    DEFAULT_SUBACCOUNT, MAINNET_CYCLES_MINTING_CANISTER_ID, MAINNET_LEDGER_CANISTER_ID,
};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    {DefaultMemoryImpl, StableBTreeMap},
};

use crate::rust_declarations::types::{
    MultisigData, TransactionData, TransactionStatus, UpdateIcpBalanceArgs,
};

use super::{cmc::CMC, ledger::Ledger};

type Memory = VirtualMemory<DefaultMemoryImpl>;

type GroupIdentifier = String;

pub static MEMO_TOP_UP_CANISTER: Memo = Memo(1347768404_u64);
pub static MEMO_CREATE_CANISTER: Memo = Memo(1095062083_u64);
pub static ICP_TRANSACTION_FEE: Tokens = Tokens::from_e8s(10000);
pub static MIN_E8S_FOR_SPINUP: Tokens = Tokens::from_e8s(110000000);
pub static CATALYZE_E8S_FEE: Tokens = Tokens::from_e8s(10000000);
pub static CATALYZE_MULTI_SIG: &str = "fcygz-gqaaa-aaaap-abpaa-cai";

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));


    pub static ENTRIES: RefCell<StableBTreeMap<GroupIdentifier, MultisigData, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
        )
    );

    pub static TRANSACTIONS: RefCell<StableBTreeMap<u64, TransactionData, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))),
        )
    );

    pub static CALLER_ICP_BALANCE: RefCell<StableBTreeMap<String, u64, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))),
        )
    );
}

pub struct Store;

impl Store {
    pub fn get_cycles() -> u64 {
        ic_cdk::api::canister_balance()
    }

    pub fn get_multisig_by_group_identifier(group_identifier: Principal) -> Option<MultisigData> {
        ENTRIES.with(|e| {
            e.borrow()
                .iter()
                .find(|(_, v)| {
                    if let Some(_group_identifier) = v.group_identifier.clone() {
                        _group_identifier == group_identifier
                    } else {
                        false
                    }
                })
                .map(|(_, v)| v.clone())
        })
    }

    pub async fn get_icp_balance(caller: Principal) -> Result<u64, String> {
        let result = account_balance(
            MAINNET_LEDGER_CANISTER_ID,
            AccountBalanceArgs {
                account: AccountIdentifier::new(&caller, &DEFAULT_SUBACCOUNT),
            },
        )
        .await;

        match result {
            Ok(balance) => Ok(balance.e8s()),
            Err((_, err)) => Err(err),
        }
    }

    pub fn get_caller_local_icp_balance(caller: Principal) -> u64 {
        CALLER_ICP_BALANCE.with(|c| {
            let balance = c.borrow();
            balance.get(&caller.to_string()).unwrap_or(0)
        })
    }

    pub fn get_transactions(status: Option<TransactionStatus>) -> Vec<TransactionData> {
        TRANSACTIONS.with(|t| {
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

    pub fn is_valid_block(block_index: u64) -> bool {
        TRANSACTIONS.with(|t| {
            if let Some(transaction) = t.borrow().get(&block_index) {
                return match transaction.status {
                    TransactionStatus::IcpToCmcFailed => true,
                    TransactionStatus::InsufficientIcp => true,
                    _ => false,
                };
            } else {
                return true;
            }
        })
    }

    pub async fn top_up_self(caller: Principal, icp_block_index: u64) -> Result<Nat, String> {
        // check if the block is already used
        if !Self::is_valid_block(icp_block_index) {
            return Err("Transaction already processed".to_string());
        }

        // initialize a base transaction data object where the field are set per case
        let mut transaction_data = TransactionData {
            icp_transfer_block_index: icp_block_index,
            cmc_transfer_block_index: None,
            icp_amount: None,
            cycles_amount: None,
            initialized_by: caller,
            created_at: time(),
            status: TransactionStatus::Pending,
            error_message: None,
        };

        // validate the transaction done from the user to this canister and return the amount
        match Ledger::validate_transaction(caller, icp_block_index).await {
            // If the transaction from the user to this canister is valid, return the amount
            Ok(amount) => {
                // add the amount to the callers balance
                Self::update_caller_icp_balance(&caller, UpdateIcpBalanceArgs::Add(amount));

                // Check if the transfer amount is lower as the minimum amount needed to spin up a canister
                if amount < MIN_E8S_FOR_SPINUP {
                    // In case the transfer amount is to low, check if the caller has enough previous balance to spin up a canister
                    let prev_amount = Tokens::from_e8s(Self::get_caller_local_icp_balance(caller));

                    // if the transfered amount + the previous balance is still to low, return an error
                    if (amount + prev_amount) < MIN_E8S_FOR_SPINUP {
                        transaction_data.icp_amount = Some(amount);
                        transaction_data.status = TransactionStatus::InsufficientIcp;
                        transaction_data.error_message =
                            Some("Amount too low to spin up a canister".to_string());
                        Self::insert_transaction_data(icp_block_index, transaction_data);
                        return Err("Amount too low to spin up a canister".to_string());
                    }
                }

                let catalyze_amount = CATALYZE_E8S_FEE - ICP_TRANSACTION_FEE;
                let multisig_amount = MIN_E8S_FOR_SPINUP - ICP_TRANSACTION_FEE - catalyze_amount;

                // Create the ledger arguments needed for the transfer call to the ledger canister
                let multig_spinup_ledger_args = TransferArgs {
                    memo: MEMO_TOP_UP_CANISTER,
                    amount: multisig_amount,
                    fee: ICP_TRANSACTION_FEE,
                    from_subaccount: None,
                    to: AccountIdentifier::new(
                        &MAINNET_CYCLES_MINTING_CANISTER_ID,
                        &Subaccount::from(id()),
                    ),
                    created_at_time: None,
                };

                // Pass the amount received from the user, from this canister to the cycles management canister (minus the fee)
                match Ledger::transfer_icp(multig_spinup_ledger_args).await {
                    // If the transaction is successfull, return the block index of the transaction
                    Ok(cmc_block_index) => {
                        // subtract the amount to the callers balance
                        Self::update_caller_icp_balance(
                            &caller,
                            UpdateIcpBalanceArgs::Subtract(amount),
                        );
                        // Trigger the call to send the cycles to this canister
                        match CMC::top_up_self(cmc_block_index).await {
                            Ok(cycles) => {
                                transaction_data.cmc_transfer_block_index = Some(cmc_block_index);
                                transaction_data.icp_amount = Some(amount);
                                transaction_data.cycles_amount = Some(cycles.clone());
                                transaction_data.status = TransactionStatus::Success;

                                Self::insert_transaction_data(icp_block_index, transaction_data);
                                Ok(cycles)
                            }
                            Err(err) => {
                                // if this step fails, the topup needs to be triggered manually with the cmc_block_index
                                transaction_data.cmc_transfer_block_index = Some(cmc_block_index);
                                transaction_data.icp_amount = Some(amount);
                                transaction_data.status = TransactionStatus::CyclesToIndexFailed;
                                transaction_data.error_message = Some(err.clone());
                                Self::insert_transaction_data(icp_block_index, transaction_data);
                                Err(err)
                            }
                        }
                    }
                    Err(err) => {
                        transaction_data.icp_amount = Some(amount);
                        transaction_data.status = TransactionStatus::IcpToCmcFailed;
                        transaction_data.error_message = Some(err.clone());
                        Self::insert_transaction_data(icp_block_index, transaction_data);

                        Err(err.to_string())
                    }
                }
            }
            Err(err) => Err(err),
        }
    }

    pub async fn spawn_multisig(
        caller: Principal,
        icp_block_index: u64,
        group_identifier: Option<Principal>,
    ) -> Result<Principal, String> {
        let spin_up_result = Self::top_up_self(caller, icp_block_index).await;
        match spin_up_result {
            Ok(cycles) => {
                let canister_id = Self::spawn_canister(cycles).await;
                match canister_id {
                    Ok(canister_id) => {
                        let install_result = Self::install_canister(caller, canister_id).await;
                        match install_result {
                            Ok(_) => {
                                ENTRIES.with(|e| {
                                    e.borrow_mut().insert(
                                        canister_id.to_string(),
                                        MultisigData {
                                            canister_id,
                                            group_identifier,
                                            created_by: caller,
                                            created_at: time(),
                                            updated_at: time(),
                                        },
                                    )
                                });

                                let catalyze_amount = CATALYZE_E8S_FEE - ICP_TRANSACTION_FEE;

                                let catalyze_fee_ledger_args = TransferArgs {
                                    memo: Memo(0),
                                    amount: catalyze_amount,
                                    fee: ICP_TRANSACTION_FEE,
                                    from_subaccount: None,
                                    to: AccountIdentifier::new(
                                        &Principal::from_text(CATALYZE_MULTI_SIG).unwrap(),
                                        &DEFAULT_SUBACCOUNT,
                                    ),
                                    created_at_time: None,
                                };

                                let _ = Ledger::transfer_icp(catalyze_fee_ledger_args).await;
                                Ok(canister_id)
                            }
                            Err(err) => Err(err),
                        }
                    }
                    Err(err) => Err(err),
                }
            }
            Err(err) => Err(err),
        }
    }

    pub async fn spawn_canister(cycles: Nat) -> Result<Principal, String> {
        let args = CreateCanisterArgument {
            settings: Some(CanisterSettings {
                controllers: Some(vec![id()]),
                compute_allocation: None,
                memory_allocation: None,
                freezing_threshold: None,
            }),
        };

        let result = create_canister(args, Self::nat_to_u128(cycles.clone())).await;
        match result {
            Ok((canister_record,)) => Ok(canister_record.canister_id),
            Err((_, err)) => Err(err),
        }
    }

    pub async fn install_canister(
        owner: Principal,
        canister_id: Principal,
    ) -> Result<Principal, String> {
        let multisig_wasm = include_bytes!("../../wasm/multisig.wasm.gz");

        let args = InstallCodeArgument {
            mode: CanisterInstallMode::Install,
            canister_id,
            wasm_module: multisig_wasm.to_vec(),
            arg: Encode!((&owner)).unwrap(),
        };
        let result = install_code(args).await;

        match result {
            Ok(()) => Ok(canister_id),
            Err((_, err)) => Err(err),
        }
    }

    fn insert_transaction_data(icp_block_index: u64, transaction_data: TransactionData) {
        TRANSACTIONS.with(|t| {
            t.borrow_mut()
                .insert(icp_block_index, transaction_data.clone())
        });
    }

    fn update_caller_icp_balance(caller: &Principal, args: UpdateIcpBalanceArgs) {
        CALLER_ICP_BALANCE.with(|c| {
            let mut balance = c.borrow_mut();
            let current_balance = balance.get(&caller.to_string()).unwrap_or(0);
            match args {
                UpdateIcpBalanceArgs::Add(amount) => {
                    balance.insert(caller.to_string(), current_balance + amount.e8s());
                }
                UpdateIcpBalanceArgs::Subtract(amount) => {
                    balance.insert(caller.to_string(), current_balance - amount.e8s());
                }
            }
        });
    }

    fn nat_to_u128(value: Nat) -> u128 {
        TryFrom::try_from(value.0).unwrap()
    }
}
