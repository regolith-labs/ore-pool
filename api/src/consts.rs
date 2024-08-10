use array_const_fn_init::array_const_fn_init;
use const_crypto::ed25519;
use solana_program::{pubkey, pubkey::Pubkey};

/// The authority allowed to operate the pool.
pub const OPERATOR_ADDRESS: Pubkey = pubkey!("HBUh9g46wk2X89CvaNN15UmsznP59rh6od1h8JwYAopk");

/// The seed of the pool account PDA.
pub const POOL: &[u8] = b"pool";

/// Program id for const pda derivations
const PROGRAM_ID: [u8; 32] = unsafe { *(&crate::id() as *const Pubkey as *const [u8; 32]) };

/// The address of the treasury account.
pub const POOL_ADDRESS: Pubkey =
    Pubkey::new_from_array(ed25519::derive_program_address(&[POOL], &PROGRAM_ID).0);

/// The bump of the treasury account, for cpis.
pub const POOL_BUMP: u8 = ed25519::derive_program_address(&[POOL], &PROGRAM_ID).1;
