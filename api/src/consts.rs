use solana_program::{pubkey, pubkey::Pubkey};

/// The seed of the member account PDA.
pub const MEMBER: &[u8] = b"member";

/// The seed of the pool account PDA.
pub const POOL: &[u8] = b"pool";

/// The seed of the share account PDA.
pub const SHARE: &[u8] = b"share";

/// The legacy boost program ID.
pub const LEGACY_BOOST_PROGRAM_ID: Pubkey = pubkey!("boostmPwypNUQu8qZ8RoWt5DXyYSVYxnBXqbbrGjecc");