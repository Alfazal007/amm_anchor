use anchor_lang::prelude::*;

use crate::DataAccount;

#[derive(Accounts)]
pub struct GetToken1<'info> {
    #[account(mut)]
    pub data_account: Account<'info, DataAccount>,
}
