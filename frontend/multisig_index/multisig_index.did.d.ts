import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';

export type Result = { 'Ok' : Tokens } |
  { 'Err' : string };
export type Result_1 = { 'Ok' : string } |
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
export type TransactionStatus = { 'CmcTransactionFailed' : null } |
  { 'IcpTransactionFailed' : null } |
  { 'Success' : null } |
  { 'CycleTopupFailed' : null } |
  { 'IcpToCmcFailed' : null } |
  { 'IcpToIndexFailed' : null };
export interface _SERVICE {
  'get_cmc_icp_balance' : ActorMethod<[], Result>,
  'get_cycles' : ActorMethod<[], bigint>,
  'get_transactions' : ActorMethod<
    [[] | [TransactionStatus]],
    Array<TransactionData>
  >,
  'top_up_self' : ActorMethod<[bigint], Result_1>,
}
