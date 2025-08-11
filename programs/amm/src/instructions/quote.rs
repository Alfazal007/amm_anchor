use anchor_lang::prelude::*;

use crate::DataAccount;

#[derive(Accounts)]
pub struct QuoteAmm<'info> {
    pub data_account: Account<'info, DataAccount>,
}
