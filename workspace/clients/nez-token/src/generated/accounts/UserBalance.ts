/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as web3 from '@solana/web3.js';
import * as beet from '@metaplex-foundation/beet';
import * as beetSolana from '@metaplex-foundation/beet-solana';

/**
 * Arguments used to create {@link UserBalance}
 * @category Accounts
 * @category generated
 */
export type UserBalanceArgs = {
  owner: web3.PublicKey;
  mint: web3.PublicKey;
  amount: beet.bignum;
  lastDepositTs: beet.bignum;
};

export const userBalanceDiscriminator = [187, 237, 208, 146, 86, 132, 29, 191];
/**
 * Holds the data for the {@link UserBalance} Account and provides de/serialization
 * functionality for that data
 *
 * @category Accounts
 * @category generated
 */
export class UserBalance implements UserBalanceArgs {
  private constructor(
    readonly owner: web3.PublicKey,
    readonly mint: web3.PublicKey,
    readonly amount: beet.bignum,
    readonly lastDepositTs: beet.bignum,
  ) {}

  /**
   * Creates a {@link UserBalance} instance from the provided args.
   */
  static fromArgs(args: UserBalanceArgs) {
    return new UserBalance(args.owner, args.mint, args.amount, args.lastDepositTs);
  }

  /**
   * Deserializes the {@link UserBalance} from the data of the provided {@link web3.AccountInfo}.
   * @returns a tuple of the account data and the offset up to which the buffer was read to obtain it.
   */
  static fromAccountInfo(accountInfo: web3.AccountInfo<Buffer>, offset = 0): [UserBalance, number] {
    return UserBalance.deserialize(accountInfo.data, offset);
  }

  /**
   * Retrieves the account info from the provided address and deserializes
   * the {@link UserBalance} from its data.
   *
   * @throws Error if no account info is found at the address or if deserialization fails
   */
  static async fromAccountAddress(
    connection: web3.Connection,
    address: web3.PublicKey,
  ): Promise<UserBalance> {
    const accountInfo = await connection.getAccountInfo(address);
    if (accountInfo == null) {
      throw new Error(`Unable to find UserBalance account at ${address}`);
    }
    return UserBalance.fromAccountInfo(accountInfo, 0)[0];
  }

  /**
   * Provides a {@link web3.Connection.getProgramAccounts} config builder,
   * to fetch accounts matching filters that can be specified via that builder.
   *
   * @param programId - the program that owns the accounts we are filtering
   */
  static gpaBuilder(
    programId: web3.PublicKey = new web3.PublicKey('2beVdAd5fpgyxwspZBfJGaqTLe2sZBm1KkBxiZFc1Mjr'),
  ) {
    return beetSolana.GpaBuilder.fromStruct(programId, userBalanceBeet);
  }

  /**
   * Deserializes the {@link UserBalance} from the provided data Buffer.
   * @returns a tuple of the account data and the offset up to which the buffer was read to obtain it.
   */
  static deserialize(buf: Buffer, offset = 0): [UserBalance, number] {
    return userBalanceBeet.deserialize(buf, offset);
  }

  /**
   * Serializes the {@link UserBalance} into a Buffer.
   * @returns a tuple of the created Buffer and the offset up to which the buffer was written to store it.
   */
  serialize(): [Buffer, number] {
    return userBalanceBeet.serialize({
      accountDiscriminator: userBalanceDiscriminator,
      ...this,
    });
  }

  /**
   * Returns the byteSize of a {@link Buffer} holding the serialized data of
   * {@link UserBalance}
   */
  static get byteSize() {
    return userBalanceBeet.byteSize;
  }

  /**
   * Fetches the minimum balance needed to exempt an account holding
   * {@link UserBalance} data from rent
   *
   * @param connection used to retrieve the rent exemption information
   */
  static async getMinimumBalanceForRentExemption(
    connection: web3.Connection,
    commitment?: web3.Commitment,
  ): Promise<number> {
    return connection.getMinimumBalanceForRentExemption(UserBalance.byteSize, commitment);
  }

  /**
   * Determines if the provided {@link Buffer} has the correct byte size to
   * hold {@link UserBalance} data.
   */
  static hasCorrectByteSize(buf: Buffer, offset = 0) {
    return buf.byteLength - offset === UserBalance.byteSize;
  }

  /**
   * Returns a readable version of {@link UserBalance} properties
   * and can be used to convert to JSON and/or logging
   */
  pretty() {
    return {
      owner: this.owner.toBase58(),
      mint: this.mint.toBase58(),
      amount: (() => {
        const x = <{ toNumber: () => number }>this.amount;
        if (typeof x.toNumber === 'function') {
          try {
            return x.toNumber();
          } catch (_) {
            return x;
          }
        }
        return x;
      })(),
      lastDepositTs: (() => {
        const x = <{ toNumber: () => number }>this.lastDepositTs;
        if (typeof x.toNumber === 'function') {
          try {
            return x.toNumber();
          } catch (_) {
            return x;
          }
        }
        return x;
      })(),
    };
  }
}

/**
 * @category Accounts
 * @category generated
 */
export const userBalanceBeet = new beet.BeetStruct<
  UserBalance,
  UserBalanceArgs & {
    accountDiscriminator: number[] /* size: 8 */;
  }
>(
  [
    ['accountDiscriminator', beet.uniformFixedSizeArray(beet.u8, 8)],
    ['owner', beetSolana.publicKey],
    ['mint', beetSolana.publicKey],
    ['amount', beet.u64],
    ['lastDepositTs', beet.i64],
  ],
  UserBalance.fromArgs,
  'UserBalance',
);
