use bytemuck::{Pod, Zeroable};

use crate::utils::{impl_account_from_bytes, impl_to_bytes, Discriminator};

use super::AccountDiscriminator;

/// Pool ...
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Pool {}

impl Discriminator for Pool {
    fn discriminator() -> u8 {
        AccountDiscriminator::Pool.into()
    }
}

impl_to_bytes!(Pool);
impl_account_from_bytes!(Pool);
