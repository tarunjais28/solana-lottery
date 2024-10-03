mod seed_component;
mod small_vec;
#[macro_use]
mod macros;

use std::ops::Deref;

use solana_program::pubkey::Pubkey;

// SeedComponent and SmallVec allows us to remove heap allocation from the PDA code.
// SmallVec is a fixed size array of size SMALL_VEC_MAX_LEN
// SeedComponent helps us to avoid storing Vec<u8> by getting &[u8] from it's members.
pub use seed_component::SeedComponent;
pub use small_vec::SmallVec;

/// Make PDA usage ergonomic.
///
/// ```rust
/// # use nezha_utils::pda::PDA;
/// # use nezha_utils::seeds;
/// # use solana_program::pubkey::Pubkey;
///
/// #[derive(Clone)]
/// enum AccountType { MyAccount }
/// enum MyError { InvalidAccount(AccountType) }
///
/// let program_id = Pubkey::new_unique();
/// let seeds = seeds!["MY_PROGRAM", "MY_ACCOUNT", "ID", 123u8];
/// let pda = PDA::new(&program_id, seeds, AccountType::MyAccount);
///
/// # struct AccountInfo { key: Pubkey }
/// # let account_info = AccountInfo { key: Pubkey::default() };
///
/// // Return MyError::InvalidAccount(AccountType::MyAccount)
/// // if the PDA doesn't match the account info.
/// let res: Result<(), MyError> = pda.verify_or(&account_info.key, &MyError::InvalidAccount);
///
/// # fn invoke_signed(_: (), _: (), _: &[&[&[u8]]]) {}
/// # let ix = ();
/// # let account_infos = ();
///
/// // Get seeds for signing.
/// invoke_signed(ix, account_infos, &[&pda.seeds()]);
/// ```
///
/// ```rust,no_run
/// // Get the pubkey
/// # use nezha_utils::pda::PDA;
/// # let pda: PDA<()> = todo!();
/// # use solana_program::pubkey::Pubkey;
/// let pubkey: &Pubkey = &pda.pubkey;
/// let pubkey: &Pubkey = &pda; // (via Deref)
/// ```
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PDA<AcTy> {
    seeds: SmallVec<SeedComponent>,
    pub pubkey: Pubkey,
    pub bump_seed: u8,
    pub account_type: AcTy,
}

impl<AcTy: Clone> PDA<AcTy> {
    pub fn new(program_id: &Pubkey, seeds: &[SeedComponent], account_type: AcTy) -> PDA<AcTy> {
        let mut seeds_: SmallVec<SeedComponent> = SmallVec::new();
        seeds_.extend_from_iter(seeds.iter().copied());

        let mut seeds_bytes: SmallVec<&[u8]> = SmallVec::new_with_sentinel(&[0u8]);
        seeds_bytes.extend_from_iter(seeds.iter().map(AsRef::as_ref));
        let (pubkey, bump_seed) = Pubkey::find_program_address(seeds_bytes.as_slice(), program_id);

        seeds_.push(bump_seed.into());
        PDA {
            seeds: seeds_,
            pubkey,
            bump_seed,
            account_type,
        }
    }

    pub fn verify_or<E>(&self, pubkey: &Pubkey, e: &dyn Fn(AcTy) -> E) -> Result<(), E> {
        if pubkey == &self.pubkey {
            Ok(())
        } else {
            Err(e(self.account_type.clone()))
        }
    }

    pub fn seeds(&self) -> SmallVec<&[u8]> {
        let mut seeds: SmallVec<&[u8]> = SmallVec::new_with_sentinel(&[0u8]);
        seeds.extend_from_iter(self.seeds.as_slice().iter().map(AsRef::as_ref));
        seeds
    }

    pub fn with_account_type<AcTy2>(self, account_type: AcTy2) -> PDA<AcTy2> {
        PDA {
            seeds: self.seeds,
            pubkey: self.pubkey,
            bump_seed: self.bump_seed,
            account_type,
        }
    }
}

impl<AcTy> Deref for PDA<AcTy> {
    type Target = Pubkey;

    fn deref(&self) -> &Self::Target {
        &self.pubkey
    }
}

#[test]
fn test_pda_seeds() {
    let program_id = Pubkey::new_unique();
    let pda = PDA::new(&program_id, seeds!("FOO", "BAR", 123u8, 567u64), "MyAccount");
    let seeds: &[&[u8]] = &pda.seeds();

    let seeds_new = &[
        "FOO".as_bytes(),
        "BAR".as_bytes(),
        &[123u8],
        &567u64.to_le_bytes(),
        &[pda.bump_seed],
    ];

    assert_eq!(seeds, seeds_new);
}

#[test]
fn test_deref() {
    let program_id = Pubkey::new_unique();
    let pda = PDA::new(&program_id, seeds!("FOO"), "MyAccount");

    let _: &Pubkey = &pda;
}

#[test]
fn test_verify() {
    let program_id = Pubkey::new_unique();
    let pda = PDA::new(&program_id, seeds!("FOO"), "MyAccount");

    let e = pda.verify_or(&Pubkey::default(), &String::from);
    assert_eq!(e, Err(String::from("MyAccount")));
}
