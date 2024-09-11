use bytemuck::{Pod, Zeroable};
use num_enum::TryFromPrimitive;
use ore_utils::instruction;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
#[rustfmt::skip]
pub enum PoolInstruction {
    // User
    Join = 0,
    Claim = 1,

    // Operator
    Attribute = 100,
    Launch = 101,
    Submit = 102,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Attribute {
    pub total_balance: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Claim {
    pub amount: [u8; 8],
    pub pool_bump: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Launch {
    pub pool_bump: u8,
    pub proof_bump: u8,
    pub url: [u8; 128],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Join {
    pub member_bump: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Submit {
    pub attestation: [u8; 32],
    pub digest: [u8; 16],
    pub nonce: [u8; 8],
}

instruction!(PoolInstruction, Launch);
instruction!(PoolInstruction, Claim);
instruction!(PoolInstruction, Attribute);
instruction!(PoolInstruction, Join);
instruction!(PoolInstruction, Submit);
