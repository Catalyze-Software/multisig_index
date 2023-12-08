import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';

export type InitializeStatus = { 'Error' : null } |
  { 'Done' : null } |
  { 'Initializing' : null };
export interface MultisigData {
  'updated_at' : bigint,
  'group_identifier' : [] | [Principal],
  'canister_id' : Principal,
  'created_at' : bigint,
  'created_by' : Principal,
}
export type Result = { 'Ok' : bigint } |
  { 'Err' : string };
export type Result_1 = { 'Ok' : Principal } |
  { 'Err' : string };
export interface Tokens { 'e8s' : bigint }
export interface TransactionData {
  'status' : TransactionStatus,
  'cmc_transfer_block_index' : [] | [bigint],
  'cycles_amount' : [] | [bigint],
  'error_message' : [] | [string],
  'initialized_by' : Principal,
  'created_at' : bigint,
  'icp_transfer_block_index' : bigint,
  'icp_amount' : [] | [Tokens],
}
export type TransactionStatus = { 'InsufficientIcp' : null } |
  { 'CyclesToIndexFailed' : null } |
  { 'Success' : null } |
  { 'IcpToCmcFailed' : null } |
  { 'IcpToIndexFailed' : null } |
  { 'Pending' : null };
export interface _SERVICE {
  'get_caller_local_balance' : ActorMethod<[], bigint>,
  'get_cmc_icp_balance' : ActorMethod<[], Result>,
  'get_cycles' : ActorMethod<[], bigint>,
  'get_initialization_status' : ActorMethod<
    [Principal],
    [] | [InitializeStatus]
  >,
  'get_multisig_by_group_identifier' : ActorMethod<
    [Principal],
    [] | [MultisigData]
  >,
  'get_multisigs' : ActorMethod<[], Array<MultisigData>>,
  'get_principal_local_balance' : ActorMethod<[Principal], bigint>,
  'get_transactions' : ActorMethod<
    [[] | [TransactionStatus]],
    Array<TransactionData>
  >,
  'spawn_multisig' : ActorMethod<[bigint, Principal], Result_1>,
}
