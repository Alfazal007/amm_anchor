use anchor_lang::prelude::*;

#[account]
pub struct DataAccount {
    pub bump: u8,
    pub token_1_mint: Pubkey,
    pub token_2_mint: Pubkey,
    pub token_1_balance: u64,
    pub token_2_balance: u64,
}
