use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, burn_checked, BurnChecked, MintTo, TransferChecked};

declare_id!("Avj3EdWetSP4wZwMG5xCn9zWKCb9cq7EQVd5xVotyJDj");

pub mod common;
pub mod instructions;

pub use common::*;
pub use instructions::*;

#[program]
pub mod amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.data_account.bump = ctx.bumps.data_account;
        ctx.accounts.data_account.token_1_mint = ctx.accounts.mint_token1.key();
        ctx.accounts.data_account.token_2_mint = ctx.accounts.mint_token2.key();
        ctx.accounts.data_account.token_1_balance = 0;
        ctx.accounts.data_account.token_2_balance = 0;
        Ok(())
    }

    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        token1_amount: u64,
        token2_amount: u64,
    ) -> Result<()> {
        require!(
            token1_amount > 0 && token2_amount > 0,
            GeneralErrors::InsufficientAmount
        );
        let token1_amount_to_add_to_pool = token1_amount;
        let token2_amount_to_add_to_pool: u64;
        let amount_to_mint: u64;
        if ctx.accounts.data_account.token_1_balance == 0 {
            token2_amount_to_add_to_pool = token2_amount;
            amount_to_mint = calc_first_lp_mint(token1_amount, token2_amount);
        } else {
            let required_first_token = ctx.accounts.data_account.token_1_balance;
            let prev_amount_2 = ctx.accounts.data_account.token_2_balance;
            let required_token2_for_all_token1 = token1_amount
                .checked_mul(prev_amount_2)
                .unwrap()
                .checked_div(required_first_token)
                .unwrap();
            token2_amount_to_add_to_pool = required_token2_for_all_token1;
            require!(
                token2_amount_to_add_to_pool <= token2_amount,
                GeneralErrors::InsufficientAmount
            );
            amount_to_mint = calc_subsequent_lp_mint(
                token1_amount_to_add_to_pool,
                token2_amount_to_add_to_pool,
                ctx.accounts.data_account.token_1_balance,
                ctx.accounts.data_account.token_2_balance,
                ctx.accounts.lp_mint.supply,
            )
        }
        transfer_tokens_general_from_user_to_pool(
            ctx.accounts.mint_token1.to_account_info(),
            ctx.accounts.token_1_account_of_user.to_account_info(),
            ctx.accounts.token_1_account.to_account_info(),
            ctx.accounts.signer.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            token1_amount_to_add_to_pool,
            ctx.accounts.mint_token1.decimals,
        )?;
        transfer_tokens_general_from_user_to_pool(
            ctx.accounts.mint_token2.to_account_info(),
            ctx.accounts.token_2_account_of_user.to_account_info(),
            ctx.accounts.token_2_account.to_account_info(),
            ctx.accounts.signer.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            token2_amount_to_add_to_pool,
            ctx.accounts.mint_token2.decimals,
        )?;
        let signer_seeds: &[&[&[u8]]] = &[&[b"mint", &[ctx.bumps.lp_mint]]];
        mint_lp_tokens(
            ctx.accounts.lp_mint.to_account_info(),
            ctx.accounts.user_lp_ata.to_account_info(),
            ctx.accounts.lp_mint.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            amount_to_mint,
            signer_seeds,
        )?;
        ctx.accounts.data_account.token_1_balance += token1_amount_to_add_to_pool;
        ctx.accounts.data_account.token_2_balance += token2_amount_to_add_to_pool;
        Ok(())
    }

    // token means token you are giving to the pool
    // amount of tokens to send to amm
    pub fn quote(ctx: Context<QuoteAmm>, token: Pubkey, amount: u64) -> Result<u64> {
        let token1_mint = ctx.accounts.data_account.token_1_mint;
        let token1_balance = ctx.accounts.data_account.token_1_balance;
        let token2_balance = ctx.accounts.data_account.token_2_balance;
        get_quote(token1_balance, token2_balance, token1_mint, amount, token)
    }

    // amount you want to put into the pool
    pub fn swap(
        ctx: Context<SwapToken>,
        amount_adding_to_pool: u64,
        token_putting_to_pool: Pubkey,
    ) -> Result<()> {
        let amount_after_fee = after_fee(amount_adding_to_pool)?;
        let token1_balance = ctx.accounts.data_account.token_1_balance;
        let token2_balance = ctx.accounts.data_account.token_2_balance;
        let token1_mint = ctx.accounts.data_account.token_1_mint;
        let amount_to_send_to_user = get_swap_quote(
            token1_balance,
            token2_balance,
            token1_mint,
            amount_after_fee,
            token_putting_to_pool,
        )?;
        let seeds: &[&[&[u8]]] = &[&[b"pool_authority", &[ctx.bumps.pool_authority]]];
        if token_putting_to_pool == ctx.accounts.data_account.token_1_mint.key() {
            transfer_tokens_general_from_user_to_pool(
                ctx.accounts.mint_token1.to_account_info(),
                ctx.accounts.token_1_account_of_user.to_account_info(),
                ctx.accounts.token_1_account.to_account_info(),
                ctx.accounts.signer.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                amount_adding_to_pool,
                ctx.accounts.mint_token1.decimals,
            )?;
            transfer_tokens_general_from_pool_to_user(
                ctx.accounts.mint_token2.to_account_info(),
                ctx.accounts.token_2_account.to_account_info(),
                ctx.accounts.token_2_account_of_user.to_account_info(),
                ctx.accounts.pool_authority.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                amount_to_send_to_user,
                ctx.accounts.mint_token2.decimals,
                seeds,
            )?;
            ctx.accounts.data_account.token_1_balance += amount_adding_to_pool;
            ctx.accounts.data_account.token_2_balance -= amount_to_send_to_user;
        } else {
            transfer_tokens_general_from_user_to_pool(
                ctx.accounts.mint_token2.to_account_info(),
                ctx.accounts.token_2_account_of_user.to_account_info(),
                ctx.accounts.token_2_account.to_account_info(),
                ctx.accounts.signer.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                amount_adding_to_pool,
                ctx.accounts.mint_token2.decimals,
            )?;
            transfer_tokens_general_from_pool_to_user(
                ctx.accounts.mint_token1.to_account_info(),
                ctx.accounts.token_1_account.to_account_info(),
                ctx.accounts.token_1_account_of_user.to_account_info(),
                ctx.accounts.pool_authority.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                amount_to_send_to_user,
                ctx.accounts.mint_token1.decimals,
                seeds,
            )?;
            ctx.accounts.data_account.token_2_balance += amount_adding_to_pool;
            ctx.accounts.data_account.token_1_balance -= amount_to_send_to_user;
        }
        Ok(())
    }

    pub fn remove_liquidity(ctx: Context<RemoveLiquidity>, amount_of_lp: u64) -> Result<()> {
        let (token1_to_return, token2_to_return) = tokens_to_return_while_remove_liquidity(
            amount_of_lp,
            ctx.accounts.lp_mint.supply,
            ctx.accounts.token_1_account.amount,
            ctx.accounts.token_2_account.amount,
        )?;
        burn_lp_tokens_from_user(
            amount_of_lp,
            ctx.accounts.user_lp_ata.to_account_info(),
            ctx.accounts.signer.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.lp_mint.to_account_info(),
            ctx.accounts.lp_mint.decimals,
        )?;
        let seeds: &[&[&[u8]]] = &[&[b"pool_authority", &[ctx.bumps.pool_authority]]];
        transfer_tokens_general_from_pool_to_user(
            ctx.accounts.mint_token1.to_account_info(),
            ctx.accounts.token_1_account.to_account_info(),
            ctx.accounts.token_1_account_of_user.to_account_info(),
            ctx.accounts.pool_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            token1_to_return,
            ctx.accounts.mint_token1.decimals,
            seeds,
        )?;
        transfer_tokens_general_from_pool_to_user(
            ctx.accounts.mint_token2.to_account_info(),
            ctx.accounts.token_2_account.to_account_info(),
            ctx.accounts.token_2_account_of_user.to_account_info(),
            ctx.accounts.pool_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            token2_to_return,
            ctx.accounts.mint_token2.decimals,
            seeds,
        )?;
        ctx.accounts.data_account.token_1_balance -= token1_to_return;
        ctx.accounts.data_account.token_2_balance -= token2_to_return;
        Ok(())
    }
}

pub fn transfer_tokens_general_from_pool_to_user<'info>(
    mint_account: AccountInfo<'info>,
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    cpi_program: AccountInfo<'info>,
    amount: u64,
    decimals: u8,
    seeds: &[&[&[u8]]],
) -> Result<()> {
    let cpi_accounts = TransferChecked {
        mint: mint_account,
        from,
        to,
        authority,
    };
    let cpi_context = CpiContext::new(cpi_program, cpi_accounts).with_signer(seeds);
    token_interface::transfer_checked(cpi_context, amount, decimals)?;
    Ok(())
}

pub fn transfer_tokens_general_from_user_to_pool<'info>(
    mint_account: AccountInfo<'info>,
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    cpi_program: AccountInfo<'info>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let cpi_accounts = TransferChecked {
        mint: mint_account,
        from,
        to,
        authority,
    };
    let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
    token_interface::transfer_checked(cpi_context, amount, decimals)?;
    Ok(())
}

pub fn mint_lp_tokens<'info>(
    lp_token_mint: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    cpi_program: AccountInfo<'info>,
    amount: u64,
    seeds: &[&[&[u8]]],
) -> Result<()> {
    let cpi_accounts = MintTo {
        mint: lp_token_mint,
        to,
        authority,
    };
    let cpi_context = CpiContext::new(cpi_program, cpi_accounts).with_signer(seeds);
    token_interface::mint_to(cpi_context, amount)?;
    Ok(())
}

pub fn calc_first_lp_mint(token1_amount: u64, token2_amount: u64) -> u64 {
    let product = token1_amount
        .checked_mul(token2_amount)
        .expect("Overflow in multiplication");
    integer_sqrt(product)
}

pub fn integer_sqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x0 = n;
    let mut x1 = (x0 + n / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + n / x0) / 2;
    }
    x0
}

pub fn calc_subsequent_lp_mint(
    new_token1_amount: u64,
    new_token2_amount: u64,
    old_token1_reserve: u64,
    old_token2_reserve: u64,
    old_total_lp_supply: u64,
) -> u64 {
    assert!(old_token1_reserve > 0 && old_token2_reserve > 0 && old_total_lp_supply > 0);
    let lp_from_token1 = new_token1_amount
        .checked_mul(old_total_lp_supply)
        .expect("Overflow in token1 LP calc")
        / old_token1_reserve;
    // Calculate LP from token2 side
    let lp_from_token2 = new_token2_amount
        .checked_mul(old_total_lp_supply)
        .expect("Overflow in token2 LP calc")
        / old_token2_reserve;
    lp_from_token1.min(lp_from_token2)
}

pub fn after_fee(amount: u64) -> Result<u64> {
    let fee_numerator: u64 = 3;
    let fee_denominator: u64 = 1000;
    let fee_amount = amount
        .checked_mul(fee_numerator)
        .ok_or(GeneralErrors::MathOverflow)?
        .checked_div(fee_denominator)
        .ok_or(GeneralErrors::MathDivisionByZero)?;
    let amount_after_fee = amount
        .checked_sub(fee_amount)
        .ok_or(GeneralErrors::MathUnderflow)?;
    Ok(amount_after_fee)
}

pub fn get_quote(
    token1_balance: u64,
    token2_balance: u64,
    token1_mint: Pubkey,
    amount_to_put_into_the_pool: u64,
    token_to_put_into_the_pool: Pubkey,
) -> Result<u64> {
    let amount_after_fees = after_fee(amount_to_put_into_the_pool)?;
    let k = token1_balance
        .checked_mul(token2_balance)
        .ok_or(GeneralErrors::MathOverflow)?;
    let res: u64;
    let tokens_to_remove_from_pool: u64;
    if token_to_put_into_the_pool == token1_mint {
        let new_t1_balance = token1_balance
            .checked_add(amount_after_fees)
            .ok_or(GeneralErrors::MathOverflow)?;
        res = k
            .checked_div(new_t1_balance)
            .ok_or(GeneralErrors::MathDivisionByZero)?;
        require!(token2_balance >= res, GeneralErrors::PoolInsufficient);
        tokens_to_remove_from_pool = token2_balance - res;
    } else {
        let denom = token2_balance
            .checked_add(amount_after_fees)
            .ok_or(GeneralErrors::MathUnderflow)?;
        res = k
            .checked_div(denom)
            .ok_or(GeneralErrors::MathDivisionByZero)?;
        require!(res <= token1_balance, GeneralErrors::PoolInsufficient);
        tokens_to_remove_from_pool = token1_balance - res;
    }
    Ok(tokens_to_remove_from_pool)
}

pub fn get_swap_quote(
    token1_balance: u64,
    token2_balance: u64,
    token1_mint: Pubkey,
    amount_to_put_into_the_pool: u64,
    token_to_put_into_the_pool: Pubkey,
) -> Result<u64> {
    let k = token1_balance
        .checked_mul(token2_balance)
        .ok_or(GeneralErrors::MathOverflow)?;
    let res: u64;
    let tokens_to_remove_from_pool: u64;
    if token_to_put_into_the_pool == token1_mint {
        let new_t1_balance = token1_balance
            .checked_add(amount_to_put_into_the_pool)
            .ok_or(GeneralErrors::MathOverflow)?;
        res = k
            .checked_div(new_t1_balance)
            .ok_or(GeneralErrors::MathDivisionByZero)?;
        require!(token2_balance >= res, GeneralErrors::PoolInsufficient);
        tokens_to_remove_from_pool = token2_balance - res;
    } else {
        let denom = token2_balance
            .checked_add(amount_to_put_into_the_pool)
            .ok_or(GeneralErrors::MathUnderflow)?;
        res = k
            .checked_div(denom)
            .ok_or(GeneralErrors::MathDivisionByZero)?;
        require!(res <= token1_balance, GeneralErrors::PoolInsufficient);
        tokens_to_remove_from_pool = token1_balance - res;
    }
    Ok(tokens_to_remove_from_pool)
}

pub fn tokens_to_return_while_remove_liquidity(
    lp_token_to_burn: u64,
    total_lp_tokens: u64,
    token_1_balance_in_pool: u64,
    token_2_balance_in_pool: u64,
) -> Result<(u64, u64)> {
    let token1_return = lp_token_to_burn
        .checked_mul(token_1_balance_in_pool)
        .ok_or(GeneralErrors::MathOverflow)?
        .checked_div(total_lp_tokens)
        .ok_or(GeneralErrors::MathDivisionByZero)?;
    let token2_return = lp_token_to_burn
        .checked_mul(token_2_balance_in_pool)
        .ok_or(GeneralErrors::MathOverflow)?
        .checked_div(total_lp_tokens)
        .ok_or(GeneralErrors::MathDivisionByZero)?;
    Ok((token1_return, token2_return))
}

pub fn burn_lp_tokens_from_user<'info>(
    amount: u64,
    from: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    decimals: u8,
) -> Result<()> {
    burn_checked(
        CpiContext::new(
            token_program,
            BurnChecked {
                mint,
                from,
                authority,
            },
        ),
        amount,
        decimals,
    )?;
    Ok(())
}
