use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

pub type BoostMint = Pubkey;
pub type StakerBalances = HashMap<Pubkey, u64>;
pub type Stakers = HashMap<BoostMint, StakerBalances>;
