use anchor_lang::prelude::*;

#[error_code]
pub enum GeneralErrors {
    #[msg("Insufficient amount")]
    InsufficientAmount,
    #[msg("Insufficient funds in the pool")]
    PoolInsufficient,
    #[msg("value too high")]
    MathOverflow,
    #[msg("value too low")]
    MathUnderflow,
    #[msg("divide by 0")]
    MathDivisionByZero,
}
