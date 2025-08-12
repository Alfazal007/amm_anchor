use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::DataAccount;

#[derive(Accounts)]
pub struct SwapToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        seeds=[b"dataAccount",  crate::ID.as_ref()],
        bump,
        mut
    )]
    pub data_account: Account<'info, DataAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        associated_token::mint = mint_token1,
        associated_token::authority = pool_authority,
        associated_token::token_program = token_program,
        mut
    )]
    pub token_1_account: InterfaceAccount<'info, TokenAccount>,
    //   #[account(mut)]
    pub mint_token1: InterfaceAccount<'info, Mint>,
    #[account(
        associated_token::mint = mint_token2,
        associated_token::authority = pool_authority,
        associated_token::token_program = token_program,
        mut
    )]
    pub token_2_account: InterfaceAccount<'info, TokenAccount>,
    //   #[account(mut)]
    pub mint_token2: InterfaceAccount<'info, Mint>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(
        seeds = [b"pool_authority"],
        bump,
    )]
    pub pool_authority: SystemAccount<'info>,
    #[account(
        associated_token::mint = mint_token1,
        associated_token::authority = signer,
        associated_token::token_program = token_program,
        mut
    )]
    pub token_1_account_of_user: InterfaceAccount<'info, TokenAccount>,
    #[account(
        associated_token::mint = mint_token2,
        associated_token::authority = signer,
        associated_token::token_program = token_program,
        mut
    )]
    pub token_2_account_of_user: InterfaceAccount<'info, TokenAccount>,
}
