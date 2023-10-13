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
    MultisigData, TransactionData, TransactionStatus, UpdateCycleBalanceArgs, UpdateIcpBalanceArgs,
};

use super::{cmc::CMC, ledger::Ledger};

type Memory = VirtualMemory<DefaultMemoryImpl>;

type GroupIdentifier = String;

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

    pub static CALLER_CYCLE_BALANCE: RefCell<StableBTreeMap<String, u128, Memory>> = RefCell::new(
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

    pub async fn get_cmc_icp_balance() -> Result<Tokens, String> {
        // let result = check_callers_balance().await;
        let result = account_balance(
            MAINNET_LEDGER_CANISTER_ID,
            AccountBalanceArgs {
                account: AccountIdentifier::new(&id(), &DEFAULT_SUBACCOUNT),
            },
        )
        .await;

        match result {
            Ok(balance) => Ok(balance),
            Err((_, err)) => Err(err),
        }
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
                    _ => false,
                };
            } else {
                return true;
            }
        })
    }

    pub async fn top_up_self(caller: Principal, icp_block_index: u64) -> Result<String, String> {
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
                // Create the ledger arguments needed for the transfer call to the ledger canister
                let ledger_args = TransferArgs {
                    memo: MEMO_TOP_UP_CANISTER,
                    amount: amount - ICP_TRANSACTION_FEE,
                    fee: ICP_TRANSACTION_FEE,
                    from_subaccount: None,
                    to: AccountIdentifier::new(
                        &MAINNET_CYCLES_MINTING_CANISTER_ID,
                        &Subaccount::from(id()),
                    ),
                    created_at_time: None,
                };

                // Pass the amount received from the user, from this canister to the cycles management canister (minus the fee)
                match Ledger::transfer_icp(ledger_args).await {
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
                                //
                                // TODO: Not sure if we want to keep track of the user cycles because the canister itself also burns cycles when it is running
                                //       which can cause a scenario where the user has cycles but the canister is out of cycles or we are in credit with the users
                                //       maybe we convert the ICP to cycles once the call to spin up the canister is done, this way we only need to keep track for
                                //       a short period of time and have a "reserved" amount of cycles in case the canister spinup fails
                                //
                                Self::update_caller_cycle_balance(
                                    &caller,
                                    UpdateCycleBalanceArgs::Add(cycles.clone()),
                                );
                                transaction_data.cmc_transfer_block_index = Some(cmc_block_index);
                                transaction_data.icp_amount = Some(amount);
                                transaction_data.cycles_amount = Some(cycles.clone());
                                transaction_data.status = TransactionStatus::Success;

                                Self::insert_transaction_data(icp_block_index, transaction_data);
                                Ok(format!("topped up with '{}' cycles", cycles))
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
            Err(err) => {
                // Do not log a transaction when the transaction is invalid (spam prevention)
                // transaction_data.status = TransactionStatus::IcpToIndexFailed;
                // transaction_data.error_message = Some(err.clone());
                // Self::insert_transaction_data(icp_block_index, transaction_data);
                Err(err)
            }
        }
    }

    pub async fn spawn_canister(caller: &Principal, cycles: Nat) -> Result<Principal, String> {
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
            Ok((canister_record,)) => {
                Self::update_caller_cycle_balance(caller, UpdateCycleBalanceArgs::Subtract(cycles));
                Ok(canister_record.canister_id)
            }
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

    fn update_caller_cycle_balance(caller: &Principal, args: UpdateCycleBalanceArgs) {
        CALLER_CYCLE_BALANCE.with(|c| {
            let mut balance = c.borrow_mut();
            let current_balance = balance.get(&caller.to_string()).unwrap_or(0);
            match args {
                UpdateCycleBalanceArgs::Add(amount) => {
                    balance.insert(
                        caller.to_string(),
                        current_balance + Self::nat_to_u128(amount),
                    );
                }
                UpdateCycleBalanceArgs::Subtract(amount) => {
                    balance.insert(
                        caller.to_string(),
                        current_balance - Self::nat_to_u128(amount),
                    );
                }
            }
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
