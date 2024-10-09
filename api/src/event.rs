use steel::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct UnstakeEvent {
    /// the authority of the share account
    pub authority: Pubkey,
    /// the share account
    pub share: Pubkey,
    /// the mint (target of the staking)
    pub mint: Pubkey,
    /// latest balance
    pub balance: u64,
}

event!(UnstakeEvent);
