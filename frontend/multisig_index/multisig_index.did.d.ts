import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';

export type Result = { 'Ok' : string } |
  { 'Err' : string };
export interface _SERVICE {
  'get_cycles' : ActorMethod<[], bigint>,
  'top_up_self' : ActorMethod<[bigint], Result>,
}
