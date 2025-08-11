use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::DataAccount;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer=signer,
        space=8+1+32+32+8+8,
        seeds=[b"dataAccount",  crate::ID.as_ref()],
        bump
    )]
    pub data_account: Account<'info, DataAccount>,
    pub system_program: Program<'info, System>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 6,
        mint::authority = lp_mint.key(),
        mint::freeze_authority = lp_mint.key(),
        seeds = [b"mint"],
        bump
    )]
    pub lp_mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint_token1,
        associated_token::authority = pool_authority,
        associated_token::token_program = token_program,
    )]
    pub token_1_account: InterfaceAccount<'info, TokenAccount>,
    pub mint_token1: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint_token2,
        associated_token::authority = pool_authority,
        associated_token::token_program = token_program,
    )]
    pub token_2_account: InterfaceAccount<'info, TokenAccount>,
    pub mint_token2: InterfaceAccount<'info, Mint>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(
        seeds = [b"pool_authority"],
        bump
    )]
    pub pool_authority: SystemAccount<'info>,
}
