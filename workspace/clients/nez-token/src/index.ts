import { Connection } from '@solana/web3.js';

export * from './generated';
export * from './consts';
export * from './utils';
export * from './types';

export interface Context {
  connection: Connection;
}
