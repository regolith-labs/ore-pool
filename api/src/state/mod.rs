mod member;
mod batch;
mod pool;

pub use member::*;
pub use batch::*
pub use pool::*;

use num_enum::{IntoPrimitive, TryFromPrimitive};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum AccountDiscriminator {
    Batch = 100,
    Member = 101,
    Pool = 102,
}
