pub mod consts;
pub mod error;
pub mod instruction;
pub mod loaders;
pub mod state;

pub(crate) use ore_utils as utils;

use solana_program::declare_id;

declare_id!("CnyzTc43LBkJsFP2XNs5RCHjHFX1ZtQ8mSHp6AW6N5TJ");
