export const idlFactory = ({ IDL }) => {
  const Result = IDL.Variant({ 'Ok' : IDL.Nat64, 'Err' : IDL.Text });
  const MultisigData = IDL.Record({
    'updated_at' : IDL.Nat64,
    'group_identifier' : IDL.Opt(IDL.Principal),
    'canister_id' : IDL.Principal,
    'created_at' : IDL.Nat64,
    'created_by' : IDL.Principal,
  });
  const TransactionStatus = IDL.Variant({
    'InsufficientIcp' : IDL.Null,
    'CyclesToIndexFailed' : IDL.Null,
    'Success' : IDL.Null,
    'IcpToCmcFailed' : IDL.Null,
    'IcpToIndexFailed' : IDL.Null,
    'Pending' : IDL.Null,
  });
  const Tokens = IDL.Record({ 'e8s' : IDL.Nat64 });
  const TransactionData = IDL.Record({
    'status' : TransactionStatus,
    'cmc_transfer_block_index' : IDL.Opt(IDL.Nat64),
    'cycles_amount' : IDL.Opt(IDL.Nat),
    'error_message' : IDL.Opt(IDL.Text),
    'initialized_by' : IDL.Principal,
    'created_at' : IDL.Nat64,
    'icp_transfer_block_index' : IDL.Nat64,
    'icp_amount' : IDL.Opt(Tokens),
  });
  const Result_1 = IDL.Variant({ 'Ok' : IDL.Principal, 'Err' : IDL.Text });
  return IDL.Service({
    'get_caller_local_balance' : IDL.Func([], [IDL.Nat64], ['query']),
    'get_cmc_icp_balance' : IDL.Func([], [Result], []),
    'get_cycles' : IDL.Func([], [IDL.Nat64], ['query']),
    'get_multisig_by_group_identifier' : IDL.Func(
        [IDL.Principal],
        [IDL.Opt(MultisigData)],
        ['query'],
      ),
    'get_transactions' : IDL.Func(
        [IDL.Opt(TransactionStatus)],
        [IDL.Vec(TransactionData)],
        ['query'],
      ),
    'spawn_multisig' : IDL.Func(
        [IDL.Nat64, IDL.Opt(IDL.Principal)],
        [Result_1],
        [],
      ),
  });
};
export const init = ({ IDL }) => { return []; };
