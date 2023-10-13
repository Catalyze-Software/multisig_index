export const idlFactory = ({ IDL }) => {
  const Result = IDL.Variant({ 'Ok' : IDL.Text, 'Err' : IDL.Text });
  return IDL.Service({
    'get_cycles' : IDL.Func([], [IDL.Nat64], ['query']),
    'top_up_self' : IDL.Func([IDL.Nat64], [Result], []),
  });
};
export const init = ({ IDL }) => { return []; };
