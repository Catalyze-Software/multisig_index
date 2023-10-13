export const idlFactory = ({ IDL }) => {
  const Tokens = IDL.Record({ 'e8s' : IDL.Nat64 });
  const Result = IDL.Variant({ 'Ok' : Tokens, 'Err' : IDL.Text });
  const TransactionStatus = IDL.Variant({
    'CmcTransactionFailed' : IDL.Null,
    'IcpTransactionFailed' : IDL.Null,
    'Success' : IDL.Null,
    'CycleTopupFailed' : IDL.Null,
    'IcpToCmcFailed' : IDL.Null,
    'IcpToIndexFailed' : IDL.Null,
  });
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
  const Result_1 = IDL.Variant({ 'Ok' : IDL.Text, 'Err' : IDL.Text });
  return IDL.Service({
    'get_cmc_icp_balance' : IDL.Func([], [Result], []),
    'get_cycles' : IDL.Func([], [IDL.Nat64], ['query']),
    'get_transactions' : IDL.Func(
        [IDL.Opt(TransactionStatus)],
        [IDL.Vec(TransactionData)],
        ['query'],
      ),
    'top_up_self' : IDL.Func([IDL.Nat64], [Result_1], []),
  });
};
export const init = ({ IDL }) => { return []; };