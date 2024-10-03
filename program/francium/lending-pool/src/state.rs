use super::*;
use crate::{
    error::LendingError,
    math::{Decimal, Rate, TryAdd, TryDiv, TryMul, TrySub, WAD},
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use enum_dispatch::enum_dispatch;
use solana_program::{
    program_error::ProgramError,
    clock::Slot,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::{Pubkey, PUBKEY_BYTES},
    clock::{DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY},
    msg,
    program_option::COption,
};

use borsh::{BorshSerialize, BorshDeserialize};
use num_enum::TryFromPrimitive;
use std::{
    cmp::Ordering,
    convert::{TryFrom, TryInto},
    io::SeekFrom,
};

use solana_program::entrypoint::ProgramResult;
use crate::instruction::MAX_SIGNERS;
use std::fs::create_dir_all;

/// Collateral tokens are initially valued at a ratio of 5:1 (collateral:liquidity)
// @FIXME: restore to 5
pub const INITIAL_COLLATERAL_RATIO: u64 = 1;
const INITIAL_COLLATERAL_RATE: u64 = INITIAL_COLLATERAL_RATIO * WAD;

/// Current version of the program and all new accounts created
pub const PROGRAM_VERSION: u8 = 1;

/// Accounts are created with data zeroed out, so uninitialized state instances
/// will have the version set to 0.
pub const UNINITIALIZED_VERSION: u8 = 0;

/// Number of slots per year
// pub const SLOTS_PER_YEAR: u64 =
//     DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;
// SENCONDS_PER_SLOT is 0.5 s
pub const SLOTS_PER_YEAR: u64 = 2 * SECONDS_PER_DAY * 365;

/// Lending market state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LendingMarket {

    /// Version of Lending market
    pub version: u8,

    /// Bump seed for derived authority address
    pub bump_seed: u8,
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
    /// Token program id
    pub token_program_id: Pubkey,
}

/// Initialize a Lending market
pub struct InitLendingMarketParams {
    /// Bump seed for derived authority address
    pub bump_seed: u8,
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
    /// Token program id
    pub token_program_id: Pubkey,
}

impl LendingMarket {
    /// Create a new Lending market
    pub fn new(params: InitLendingMarketParams) -> Self {
        let mut lending_market = Self::default();
        Self::init(&mut lending_market, params);
        lending_market
    }

    /// Initialize a Lending market
    pub fn init(&mut self, params: InitLendingMarketParams) {
        self.version = PROGRAM_VERSION;
        self.bump_seed = params.bump_seed;
        self.token_program_id = params.token_program_id;
        self.owner = params.owner;
    }

    pub fn transfer_owner(&mut self, new_owner: Pubkey) {
        self.owner = new_owner;
    }

}

impl Sealed for LendingMarket {}
impl IsInitialized for LendingMarket {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const LENDING_MARKET_LEN: usize = 194; // [1, 1, 32, 32, 128]
impl Pack for LendingMarket {
    const LEN: usize = LENDING_MARKET_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, LENDING_MARKET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
            let (version, bump_seed, owner, token_program_id, _padding) =
            mut_array_refs![output,
                1, 1, 32, 32, 128
            ];

        *version = self.version.to_le_bytes();
        *bump_seed = self.bump_seed.to_le_bytes();
        owner.copy_from_slice(self.owner.as_ref());
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
    }

    /// Unpacks a byte buffer into a [LendingMarketInfo](struct.LendingMarketInfo.html)
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, LENDING_MARKET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
            let (version, bump_seed, owner, token_program_id, _padding) =
            array_refs![input,
                1, 1, 32, 32, 128
            ];

        let version = u8::from_le_bytes(*version);
        if version > PROGRAM_VERSION {
            msg!("Lending market version does not match Lending program version");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self {
            version,
            bump_seed: u8::from_le_bytes(*bump_seed),
            owner: Pubkey::new_from_array(*owner),
            token_program_id: Pubkey::new_from_array(*token_program_id),
        })
    }
}

/// Lending market reserve state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LendingPool {
    /// Version of the struct, len = 1
    pub version: u8,
    /// Last slot when supply and rates updated, len = 9
    pub last_update: LastUpdate,
    /// Lending market address, len = 32
    pub lending_market: Pubkey,
    /// Reserve liquidity, len = 193
    pub liquidity: ReserveLiquidity,
    /// Liquidity shares, len = 72
    pub shares: LiquidityShares,
    /// Credit token, len = 72
    pub credit: CreditToken,
    /// InterestRate len = 11
    pub interest_rate_model: InterestRateModel,
    /// interest reverse rate
    pub interest_reverse_rate: u8,
    /// accumulated_interest_reverse: u64
    pub accumulated_interest_reverse: u64,
    // _padding 108
}

impl Sealed for LendingPool {}
impl IsInitialized for LendingPool {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const LENDING_POOL_LEN: usize = 495;
/// [1, 8, 1, 32, 32, 1, 32, 32, 36, 8, 16, 16, 8, 32, 8, 32,  32, 8, 32, 11,1,8,108]
// @TODO: break this up by reserve / liquidity / collateral / config https://git.io/JOCca
impl Pack for LendingPool {
    const LEN: usize = LENDING_POOL_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, LENDING_POOL_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
            let (
            version,
            last_update_slot,
            last_update_stale,
            lending_market,
            liquidity_mint_pubkey,
            liquidity_mint_decimals,
            liquidity_supply_pubkey,
            liquidity_fee_receiver,
            liquidity_oracle_pubkey,
            liquidity_available_amount,
            liquidity_borrowed_amount_wads,
            liquidity_cumulative_borrow_rate_wads,
            liquidity_market_price,
            share_mint_pubkey,
            share_mint_total_supply,
            share_supply_pubkey,
            credit_mint_pubkey,
            credit_mint_total_supply,
            credit_supply_pubkey,
            interest_model,
            interest_reverse_rate,
            accumulated_interest_reverse,
            _padding,
        ) = mut_array_refs![output,
            1, 8, 1, 32, 32, 1, 32, 32, 36, 8, 16, 16, 8, 32, 8, 32,  32, 8, 32, 11,1,8,108
        ];

        // reserve
        *version = self.version.to_le_bytes();
        *last_update_slot = self.last_update.slot.to_le_bytes();
        pack_bool(self.last_update.stale, last_update_stale);
        lending_market.copy_from_slice(self.lending_market.as_ref());

        // liquidity
        liquidity_mint_pubkey.copy_from_slice(self.liquidity.mint_pubkey.as_ref());
        *liquidity_mint_decimals = self.liquidity.mint_decimals.to_le_bytes();
        liquidity_supply_pubkey.copy_from_slice(self.liquidity.supply_pubkey.as_ref());
        liquidity_fee_receiver.copy_from_slice(self.liquidity.fee_receiver.as_ref());
        pack_coption_key(&self.liquidity.oracle_pubkey, liquidity_oracle_pubkey);
        *liquidity_available_amount = self.liquidity.available_amount.to_le_bytes();
        pack_decimal(
            self.liquidity.borrowed_amount_wads,
            liquidity_borrowed_amount_wads,
        );
        pack_decimal(
            self.liquidity.cumulative_borrow_rate_wads,
            liquidity_cumulative_borrow_rate_wads,
        );
        *liquidity_market_price = self.liquidity.market_price.to_le_bytes();

        // share
        share_mint_pubkey.copy_from_slice(self.shares.mint_pubkey.as_ref());
        *share_mint_total_supply = self.shares.mint_total_supply.to_le_bytes();
        share_supply_pubkey.copy_from_slice(self.shares.supply_pubkey.as_ref());

        // credit
        credit_mint_pubkey.copy_from_slice(self.credit.mint_pubkey.as_ref());
        *credit_mint_total_supply = self.credit.mint_total_supply.to_le_bytes();
        credit_supply_pubkey.copy_from_slice(self.credit.supply_pubkey.as_ref());

        // interest model
        self.interest_rate_model.pack(interest_model);

        // interest reverse
        interest_reverse_rate[0] = self.interest_reverse_rate;
        accumulated_interest_reverse.copy_from_slice(&self.accumulated_interest_reverse.to_le_bytes());
    }

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, LENDING_POOL_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
            let (
            version,
            last_update_slot,
            last_update_stale,
            lending_market,
            liquidity_mint_pubkey,
            liquidity_mint_decimals,
            liquidity_supply_pubkey,
            liquidity_fee_receiver,
            liquidity_oracle_pubkey,
            liquidity_available_amount,
            liquidity_borrowed_amount_wads,
            liquidity_cumulative_borrow_rate_wads,
            liquidity_market_price,
            share_mint_pubkey,
            share_mint_total_supply,
            share_supply_pubkey,
            credit_mint_pubkey,
            credit_mint_total_supply,
            credit_supply_pubkey,
            interest_model,
            interest_reverse_rate,
            accumulated_interest_reverse,
            _padding,
        ) = array_refs![input,
            1, 8, 1, 32, 32, 1, 32, 32, 36, 8, 16, 16, 8, 32, 8, 32,  32, 8, 32, 11,1,8,108
            ];

        let version = u8::from_le_bytes(*version);
        if version > PROGRAM_VERSION {
            msg!("Reserve version does not match Lending program version");
            return Err(ProgramError::InvalidAccountData);
        }

        let interest_reverse_rate = if interest_reverse_rate[0] == 0 || interest_reverse_rate[0] > 50 {
            10
        }else {
            interest_reverse_rate[0]
        };

        let ret = Self {
            version,
            last_update: LastUpdate {
                slot: u64::from_le_bytes(*last_update_slot),
                stale: unpack_bool(last_update_stale)?,
            },
            lending_market: Pubkey::new_from_array(*lending_market),
            liquidity: ReserveLiquidity {
                mint_pubkey: Pubkey::new_from_array(*liquidity_mint_pubkey),
                mint_decimals: u8::from_le_bytes(*liquidity_mint_decimals),
                supply_pubkey: Pubkey::new_from_array(*liquidity_supply_pubkey),
                fee_receiver: Pubkey::new_from_array(*liquidity_fee_receiver),
                oracle_pubkey: unpack_coption_key(liquidity_oracle_pubkey)?,
                available_amount: u64::from_le_bytes(*liquidity_available_amount),
                borrowed_amount_wads: unpack_decimal(liquidity_borrowed_amount_wads),
                cumulative_borrow_rate_wads: unpack_decimal(liquidity_cumulative_borrow_rate_wads),
                market_price: u64::from_le_bytes(*liquidity_market_price),
            },
            shares: LiquidityShares {
                mint_pubkey: Pubkey::new_from_array(*share_mint_pubkey),
                mint_total_supply: u64::from_le_bytes(*share_mint_total_supply),
                supply_pubkey: Pubkey::new_from_array(*share_supply_pubkey),
            },
            credit: CreditToken{
                mint_pubkey: Pubkey::new_from_array(*credit_mint_pubkey),
                mint_total_supply: u64::from_le_bytes(*credit_mint_total_supply),
                supply_pubkey: Pubkey::new_from_array(*credit_supply_pubkey),
            },
            interest_rate_model: InterestRateModel::unpack(interest_model),
            interest_reverse_rate,
            accumulated_interest_reverse: u64::from_le_bytes(*accumulated_interest_reverse),
        };

        Ok(ret)
    }
}

impl LendingPool {
    /// Create a new reserve
    pub fn new(params: InitLendingPoolParams) -> Self {
        let mut lending_pool = Self::default();
        Self::init(&mut lending_pool, params);
        lending_pool
    }

    /// Initialize a reserve
    pub fn init(&mut self, params: InitLendingPoolParams) {
        self.version = PROGRAM_VERSION;
        self.last_update = LastUpdate::new(params.current_slot);
        self.lending_market = params.lending_market;
        self.liquidity = params.liquidity;
        self.shares = params.shares;
        self.credit = params.credit;
    }

    /// Unpacks cumulated_borrow_rate
    pub fn unpack_cumulated_borrow_rate(input: &[u8]) -> Result<Decimal, ProgramError> {
        let input = array_ref![input, 199, 16];

        Ok(unpack_decimal(input))
    }

    /// Record deposited liquidity and return amount of collateral tokens to mint
    pub fn deposit_liquidity(&mut self, liquidity_amount: u64) -> Result<u64, ProgramError> {
        let share_amount = self
            .share_exchange_rate()?
            .liquidity_to_share(liquidity_amount)?;

        self.liquidity.deposit(liquidity_amount)?;
        self.shares.mint(share_amount)?;

        Ok(share_amount)
    }

    /// Withdraw liquidity according to user's liquidity shares
    pub fn withdraw_liquidity(&mut self, share_amount: u64) -> Result<u64, ProgramError> {
        let share_exchange_rate = self.share_exchange_rate()?;
        let liquidity_amount =
            share_exchange_rate.share_to_liquidity(share_amount)?;

        self.shares.burn(share_amount)?;
        self.liquidity.withdraw(liquidity_amount)?;

        Ok(liquidity_amount)
    }

    /// Calculate the current borrow rate
    pub fn current_borrow_rate(&self) -> Result<Rate, ProgramError> {
        let utilization_rate = self.liquidity.utilization_rate().unwrap();
        self.interest_rate_model.current_borrow_rate(
            utilization_rate
        )
    }

    /// Share exchange rate
    /// exchange_rate = total_shares.div(total_liquidity)
    pub fn share_exchange_rate(&self) -> Result<ShareExchangeRate, ProgramError> {
        let total_liquidity = self.liquidity.total_supply()?;
        self.shares.exchange_rate(total_liquidity)
    }

    /// Update borrow rate and accrue interest
    pub fn accrue_interest(&mut self, current_slot: Slot) -> ProgramResult {
        let slots_elapsed = self.last_update.slots_elapsed(current_slot)?;
        if slots_elapsed > 0 {
            let current_borrow_rate = self.current_borrow_rate()?;
            let interest = self.liquidity
                .compound_interest(current_borrow_rate, slots_elapsed)?;

            let interest_reverse = interest.try_mul(
                Rate::from_percent(self.interest_reverse_rate)
            ).unwrap().try_floor_u64().unwrap();

            if interest_reverse < self.liquidity.available_amount {
                self.liquidity.charge_interest_reverse(interest_reverse)?;

                self.accumulated_interest_reverse =
                    self.accumulated_interest_reverse
                        .checked_add(interest_reverse)
                        .ok_or(LendingError::MathOverflow)?;
            }
        }
        Ok(())
    }

    pub fn update_interest_reverse_rate(&mut self, reverse_rate: u8) {
        if reverse_rate > 50 {
            return
        }

        self.interest_reverse_rate = reverse_rate;
    }

    pub fn update_interest_model(&mut self, interest_model: InterestRateModel) {
        self.interest_rate_model = interest_model
    }
}

/// Initialize a liquidity pool
pub struct InitLendingPoolParams {
    /// Last slot when supply and rates updated
    pub current_slot: Slot,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Reserve liquidity
    pub liquidity: ReserveLiquidity,
    /// Reserve collateral
    pub shares: LiquidityShares,
    /// Credit token
    pub credit: CreditToken,
}

/// Reserve liquidity, len 181 [32, 1, 32, 32, 36, 8, 16, 16, 8]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReserveLiquidity {
    /// Reserve liquidity mint address, len 32
    pub mint_pubkey: Pubkey,
    /// Reserve liquidity mint decimals, len 1
    pub mint_decimals: u8,
    /// Reserve liquidity supply address, len 32
    pub supply_pubkey: Pubkey,
    /// Reserve liquidity fee receiver address, len 32
    pub fee_receiver: Pubkey,
    /// Optional reserve liquidity oracle state account, len 4+32
    pub oracle_pubkey: COption<Pubkey>,
    /// Reserve liquidity available, len 8
    pub available_amount: u64,
    /// Reserve liquidity borrowed, len 16
    pub borrowed_amount_wads: Decimal,
    /// Reserve liquidity cumulative borrow rate, len 16
    pub cumulative_borrow_rate_wads: Decimal,
    /// Reserve liquidity market price in quote currency, len 8
    pub market_price: u64,
}

impl ReserveLiquidity {
    /// Create a new reserve liquidity
    pub fn new(params: NewReserveLiquidityParams) -> Self {
        Self {
            mint_pubkey: params.mint_pubkey,
            mint_decimals: params.mint_decimals,
            supply_pubkey: params.supply_pubkey,
            fee_receiver: params.fee_receiver,
            oracle_pubkey: params.oracle_pubkey,
            cumulative_borrow_rate_wads: Decimal::one(),
            market_price: params.market_price,
            available_amount: 0,
            borrowed_amount_wads: Decimal::zero(),
        }
    }

    /// Calculate the total reserve supply including active loans
    pub fn total_supply(&self) -> Result<Decimal, ProgramError> {
        Decimal::from(self.available_amount).try_add(self.borrowed_amount_wads)
    }

    /// Charge Interest Reverse
    pub fn charge_interest_reverse(&mut self, interest_reverse_amount: u64)-> ProgramResult  {
        if interest_reverse_amount > self.available_amount {
            msg!("Charge interest reverse amount cannot exceed available amount");
            return Err(LendingError::InsufficientLiquidity.into());
        }

        self.available_amount = self
            .available_amount
            .checked_sub(interest_reverse_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Add liquidity to available amount
    pub fn deposit(&mut self, liquidity_amount: u64) -> ProgramResult {
        self.available_amount = self
            .available_amount
            .checked_add(liquidity_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Remove liquidity from available amount
    pub fn withdraw(&mut self, liquidity_amount: u64) -> ProgramResult {
        if liquidity_amount > self.available_amount {
            msg!("Withdraw amount cannot exceed available amount");
            return Err(LendingError::InsufficientLiquidity.into());
        }
        self.available_amount = self
            .available_amount
            .checked_sub(liquidity_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Subtract borrow amount from available liquidity and add to borrows
    pub fn borrow(&mut self, borrow_decimal: Decimal) -> ProgramResult {
        let borrow_amount = borrow_decimal.try_floor_u64()?;
        if borrow_amount > self.available_amount {
            msg!("Borrow amount cannot exceed available amount");
            return Err(LendingError::InsufficientLiquidity.into());
        }

        self.available_amount = self
            .available_amount
            .checked_sub(borrow_amount)
            .ok_or(LendingError::MathOverflow)?;
        self.borrowed_amount_wads = self.borrowed_amount_wads.try_add(borrow_decimal)?;

        Ok(())
    }

    /// Add repay amount to available liquidity and subtract settle amount from total borrows
    pub fn repay(&mut self, repay_amount: u64, settle_amount: Decimal) -> ProgramResult {
        self.available_amount = self
            .available_amount
            .checked_add(repay_amount)
            .ok_or(LendingError::MathOverflow)?;

        if self.borrowed_amount_wads.gt(&settle_amount) {
            self.borrowed_amount_wads = self.borrowed_amount_wads.try_sub(settle_amount)?;
        } else {
            self.borrowed_amount_wads = Decimal::zero();
        }

        Ok(())
    }

    /// Calculate the liquidity utilization rate of the reserve
    pub fn utilization_rate(&self) -> Result<Rate, ProgramError> {
        let total_supply = self.total_supply()?;
        if total_supply == Decimal::zero() {
            return Ok(Rate::zero());
        }
        self.borrowed_amount_wads.try_div(total_supply)?.try_into()
    }

    /// Compound current borrow rate over elapsed slots
    fn compound_interest(
        &mut self,
        current_borrow_rate: Rate,
        slots_elapsed: u64,
    ) -> Result<Decimal, ProgramError> {
        let slot_interest_rate = current_borrow_rate.try_div(SLOTS_PER_YEAR)?;
        let compounded_interest_rate = Rate::one()
            .try_add(slot_interest_rate)?
            .try_pow(slots_elapsed)?;
        self.cumulative_borrow_rate_wads = self
            .cumulative_borrow_rate_wads
            .try_mul(compounded_interest_rate)?;

        let interest = self.borrowed_amount_wads.try_mul(
            compounded_interest_rate
                .try_sub(Rate::one()).unwrap()
        ).unwrap();

        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .try_mul(compounded_interest_rate)?;
        msg!(
            "current_borrow_rate: {}\ncumulative_borrow_rate_wads: {}\nborrowed_amount_wads: {}",
            current_borrow_rate.to_string(),
            self.cumulative_borrow_rate_wads.to_string(),
            self.borrowed_amount_wads.to_string()
        );

        Ok((interest))
    }
}

/// Create a new reserve liquidity
pub struct NewReserveLiquidityParams {
    /// Reserve liquidity mint address
    pub mint_pubkey: Pubkey,
    /// Reserve liquidity mint decimals
    pub mint_decimals: u8,
    /// Reserve liquidity supply address
    pub supply_pubkey: Pubkey,
    /// Reserve liquidity fee receiver address
    pub fee_receiver: Pubkey,
    /// Optional reserve liquidity oracle state account
    pub oracle_pubkey: COption<Pubkey>,
    /// Reserve liquidity market price in quote currency
    pub market_price: u64,
}

/// Create a new reserve collateral
pub struct NewCreditParams {
    /// Reserve collateral mint address
    pub mint_pubkey: Pubkey,
    /// Reserve collateral supply address
    pub supply_pubkey: Pubkey,
}

/// Credit Token, len 72 = 32+8+32
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CreditToken {
    /// Reserve collateral mint address
    pub mint_pubkey: Pubkey,
    /// Reserve collateral mint supply, used for exchange rate
    pub mint_total_supply: u64,
    /// Reserve collateral supply address
    pub supply_pubkey: Pubkey,
}

impl CreditToken {
    pub fn new(params: NewCreditParams) -> Self {
        Self {
            mint_pubkey: params.mint_pubkey,
            mint_total_supply: 0,
            supply_pubkey: params.supply_pubkey,
        }
    }

    pub fn mint(&mut self, credit_amount: u64) -> ProgramResult {
        self.mint_total_supply = self
            .mint_total_supply
            .checked_add(credit_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    pub fn burn(&mut self, credit_amount: u64) -> ProgramResult {
        self.mint_total_supply = self
            .mint_total_supply
            .checked_sub(credit_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }
}

/// Liquidity shares, len 72
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LiquidityShares {
    /// Reserve collateral mint address
    pub mint_pubkey: Pubkey,
    /// Reserve collateral mint supply, used for exchange rate
    pub mint_total_supply: u64,
    /// Reserve collateral supply address
    pub supply_pubkey: Pubkey,
}

impl LiquidityShares {
    /// Create a new reserve collateral
    pub fn new(params: NewLiquiditShareParams) -> Self {
        Self {
            mint_pubkey: params.mint_pubkey,
            mint_total_supply: 0,
            supply_pubkey: params.supply_pubkey,
        }
    }

    pub fn mint(&mut self, share_amount: u64) -> ProgramResult {
        self.mint_total_supply = self
            .mint_total_supply
            .checked_add(share_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    pub fn burn(&mut self, share_amount: u64) -> ProgramResult {
        self.mint_total_supply = self
            .mint_total_supply
            .checked_sub(share_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Return the current collateral exchange rate.
    fn exchange_rate(
        &self,
        total_liquidity: Decimal,
    ) -> Result<ShareExchangeRate, ProgramError> {
        let rate = if self.mint_total_supply == 0 || total_liquidity == Decimal::zero() {
            Rate::from_scaled_val(INITIAL_COLLATERAL_RATE)
        } else {
            let mint_total_supply = Decimal::from(self.mint_total_supply);
            Rate::try_from(mint_total_supply.try_div(total_liquidity)?)?
        };

        Ok(ShareExchangeRate(rate))
    }
}

/// Create a new reserve collateral
pub struct NewLiquiditShareParams {
    /// Reserve collateral mint address
    pub mint_pubkey: Pubkey,
    /// Reserve collateral supply address
    pub supply_pubkey: Pubkey,
}

/// Collateral exchange rate
#[derive(Clone, Copy, Debug)]
pub struct ShareExchangeRate(Rate);

impl ShareExchangeRate {
    /// Convert shares to liquidity
    pub fn share_to_liquidity(&self, share_amount: u64) -> Result<u64, ProgramError> {
        self.decimal_share_to_liquidity(share_amount.into())?
            .try_floor_u64()
    }

    /// Convert shares to liquidity
    pub fn decimal_share_to_liquidity(
        &self,
        share_amount: Decimal,
    ) -> Result<Decimal, ProgramError> {
        share_amount.try_div(self.0)
    }

    /// Convert liquidity to shares
    pub fn liquidity_to_share(&self, liquidity_amount: u64) -> Result<u64, ProgramError> {
        self.decimal_liquidity_to_share(liquidity_amount.into())?
            .try_floor_u64()
    }

    /// Convert liquidity to shares
    pub fn decimal_liquidity_to_share(
        &self,
        liquidity_amount: Decimal,
    ) -> Result<Decimal, ProgramError> {
        liquidity_amount.try_mul(self.0)
    }
}

impl From<ShareExchangeRate> for Rate {
    fn from(exchange_rate: ShareExchangeRate) -> Self {
        exchange_rate.0
    }
}

/// Number of slots to consider stale after
pub const STALE_AFTER_SLOTS_ELAPSED: u64 = 1;

/// Last update state
#[derive(Clone, Debug, Default)]
pub struct LastUpdate {
    /// Last slot when updated
    pub slot: Slot,
    /// True when marked stale, false when slot updated
    pub stale: bool,
}

impl LastUpdate {
    /// Create new last update
    pub fn new(slot: Slot) -> Self {
        Self { slot, stale: true }
    }

    /// Return slots elapsed since given slot
    pub fn slots_elapsed(&self, slot: Slot) -> Result<u64, ProgramError> {
        let slots_elapsed = slot
            .checked_sub(self.slot)
            .ok_or(LendingError::MathOverflow)?;
        Ok(slots_elapsed)
    }

    /// Set last update slot
    pub fn update_slot(&mut self, slot: Slot) {
        self.slot = slot;
        self.stale = false;
    }

    /// Set stale to true
    pub fn mark_stale(&mut self) {
        self.stale = true;
    }

    /// Check if marked stale or last update slot is too long ago
    pub fn is_stale(&self, slot: Slot) -> Result<bool, ProgramError> {
        Ok(self.stale || self.slots_elapsed(slot)? >= STALE_AFTER_SLOTS_ELAPSED)
    }
}

impl PartialEq for LastUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot
    }
}

impl PartialOrd for LastUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.slot.partial_cmp(&other.slot)
    }
}

/// InterestRateModel Len=11
#[derive(Clone, Debug, PartialEq)]
pub struct InterestRateModel {
    pub threshold_1: u8,
    pub threshold_2: u8,
    pub base_1: u8,
    pub factor_1: u16,
    pub base_2: u8,
    pub factor_2: u16,
    pub base_3: u8,
    pub factor_3: u16,
}

impl Default for InterestRateModel {
    fn default() -> Self {
        Self{
            threshold_1: 60,
            threshold_2: 90,
            base_1: 0,
            factor_1: 25,
            base_2: 15,
            factor_2: 0,
            base_3: 15,
            factor_3: 1300,
        }
    }
}

impl InterestRateModel {
    pub fn new(
        threshold_1: u8,
        threshold_2: u8,
        base_1: u8,
        factor_1: u16,
        base_2: u8,
        factor_2: u16,
        base_3: u8,
        factor_3: u16,
    ) -> Self {
        Self{
            threshold_1,
            threshold_2,
            base_1,
            factor_1,
            base_2,
            factor_2,
            base_3,
            factor_3,
        }
    }

    /// current_borrow_rate
    /// 0 < utilization_rate < threshold_1:
    ///       borrow_rate = base_1 + factor_1 * utilization_rate
    /// threshold_1 < utilization_rate < threshold_2:
    ///       borrow_rate = base_2 + factor_2 * (utilization_rate - threshold_1)
    /// threshold_2 < utilization_rate < 100:
    ///       borrow_rate = base_3 + factor_3 * (utilization_rate - threshold_2)
    pub fn current_borrow_rate(&self, utilization_rate: Rate) -> Result<Rate, ProgramError> {
        if utilization_rate < Rate::from_percent(self.threshold_1) {
            Rate::from_percent(self.base_1).try_add(
                utilization_rate.try_mul(
                    self.factor_1 as u64
                ).unwrap().try_div(
                    100
                ).unwrap()
            )
        } else if utilization_rate < Rate::from_percent(self.threshold_2) {
            Rate::from_percent(self.base_2).try_add(
                utilization_rate.try_sub( Rate::from_percent(self.threshold_1)).unwrap().
                    try_mul(self.factor_2 as u64 ).unwrap()
                    .try_div(100).unwrap()
            )
        } else {
            Rate::from_percent(self.base_3).try_add(
                utilization_rate.try_sub( Rate::from_percent(self.threshold_2)).unwrap().
                    try_mul(self.factor_3 as u64 ).unwrap().
                    try_div(100).unwrap()
            )
        }
    }

    pub fn pack(&self, dst: &mut [u8; 11]) {
        let (
            threshold_1,
            threshold_2,
            base_1,
            factor_1,
            base_2,
            factor_2,
            base_3,
            factor_3,
        ) = mut_array_refs![dst, 1, 1, 1, 2 ,1, 2, 1 , 2];

        threshold_1[0] = self.threshold_1;
        threshold_2[0] = self.threshold_2;
        base_1[0] = self.base_1;
        factor_1.copy_from_slice(&self.factor_1.to_le_bytes());
        base_2[0] = self.base_2;
        factor_2.copy_from_slice(&self.factor_2.to_le_bytes());
        base_3[0] = self.base_3;
        factor_3.copy_from_slice(&self.factor_3.to_le_bytes());
    }

    pub fn to_bytes(&self) -> [u8;11] {
        let mut dst:[u8; 11] = [0,0,0,0,0,0,0,0,0,0,0];
        self.pack(&mut dst);
        return dst;
    }

    pub fn unpack(src: &[u8;11]) -> Self {
        if src.eq(&[0,0,0,0,0,0,0,0,0,0,0]) {
            return Self::default();
        }

        let (
            threshold_1,
            threshold_2,
            base_1,
            factor_1,
            base_2,
            factor_2,
            base_3,
            factor_3,
        ) = array_refs![src, 1, 1, 1, 2 ,1, 2, 1 , 2];


        Self {
            threshold_1: threshold_1[0],
            threshold_2: threshold_2[0],
            base_1: base_1[0],
            factor_1: u16::from_le_bytes(*factor_1),
            base_2: base_2[0],
            factor_2: u16::from_le_bytes(*factor_2),
            base_3: base_3[0],
            factor_3: u16::from_le_bytes(*factor_3),
        }
    }

    pub fn unpack_from_slice(src: &[u8]) -> Self {
        let data = array_ref![src, 0, 11];

        Self::unpack(data)
    }
}

// Helpers
fn pack_coption_key(src: &COption<Pubkey>, dst: &mut [u8; 36]) {
    let (tag, body) = mut_array_refs![dst, 4, 32];
    match src {
        COption::Some(key) => {
            *tag = [1, 0, 0, 0];
            body.copy_from_slice(key.as_ref());
        }
        COption::None => {
            *tag = [0; 4];
        }
    }
}
fn unpack_coption_key(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 32];
    match *tag {
        [0, 0, 0, 0] => Ok(COption::None),
        [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}
fn pack_coption_u64(src: &COption<u64>, dst: &mut [u8; 12]) {
    let (tag, body) = mut_array_refs![dst, 4, 8];
    match src {
        COption::Some(amount) => {
            *tag = [1, 0, 0, 0];
            *body = amount.to_le_bytes();
        }
        COption::None => {
            *tag = [0; 4];
        }
    }
}
fn unpack_coption_u64(src: &[u8; 12]) -> Result<COption<u64>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 8];
    match *tag {
        [0, 0, 0, 0] => Ok(COption::None),
        [1, 0, 0, 0] => Ok(COption::Some(u64::from_le_bytes(*body))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

/// Collateral tokens are initially valued at a ratio of 5:1 (collateral:liquidity)
// @FIXME: restore to 5
fn pack_decimal(decimal: Decimal, dst: &mut [u8; 16]) {
    *dst = decimal
        .to_scaled_val()
        .expect("Decimal cannot be packed")
        .to_le_bytes();
}

fn unpack_decimal(src: &[u8; 16]) -> Decimal {
    Decimal::from_scaled_val(u128::from_le_bytes(*src))
}

fn pack_bool(boolean: bool, dst: &mut [u8; 1]) {
    *dst = (boolean as u8).to_le_bytes()
}

fn unpack_bool(src: &[u8; 1]) -> Result<bool, ProgramError> {
    match u8::from_le_bytes(*src) {
        0 => Ok(false),
        1 => Ok(true),
        _ => {
            msg!("Boolean cannot be unpacked");
            Err(ProgramError::InvalidAccountData)
        }
    }
}
