use steel::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum PoolInstruction {
    // User
    Claim = 0,
    Join = 1,
    OpenShare = 2,
    Stake = 3,
    Unstake = 4,

    // Operator
    Attribute = 100,
    Commit = 101,
    Launch = 102,
    OpenStake = 103,
    Submit = 104,
    OpenTotalRewards = 105,
    OpenShareRewards = 106,
    IncrementTotalRewards = 107,
    IncrementShareRewards = 108,
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
pub struct Commit {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Launch {
    pub pool_bump: u8,
    pub proof_bump: u8,
    pub url: [u8; 128],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct OpenShare {
    pub share_bump: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct OpenStake {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Join {
    pub member_bump: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Stake {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Submit {
    pub attestation: [u8; 32],
    pub digest: [u8; 16],
    pub nonce: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Unstake {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct OpenTotalRewards {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct OpenShareRewards {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct IncrementTotalRewards {
    pub miner_rewards: [u8; 8],
    pub staker_rewards: [u8; 8],
    pub operator_rewards: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct IncrementShareRewards {
    pub rewards: [u8; 8],
}

instruction!(PoolInstruction, Attribute);
instruction!(PoolInstruction, Claim);
instruction!(PoolInstruction, Commit);
instruction!(PoolInstruction, Launch);
instruction!(PoolInstruction, OpenShare);
instruction!(PoolInstruction, OpenStake);
instruction!(PoolInstruction, Join);
instruction!(PoolInstruction, Stake);
instruction!(PoolInstruction, Submit);
instruction!(PoolInstruction, Unstake);
instruction!(PoolInstruction, OpenTotalRewards);
instruction!(PoolInstruction, OpenShareRewards);
instruction!(PoolInstruction, IncrementTotalRewards);
instruction!(PoolInstruction, IncrementShareRewards);
