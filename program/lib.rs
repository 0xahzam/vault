use {
    anchor_lang::prelude::*,
    anchor_spl::token::{transfer, Token, TokenAccount, Transfer},
    std::collections::BTreeMap,
};

declare_id!("ELfpHVqpAKgJnnQg6mC8SBcntVYwR8UmjBKEyC4n2cDN");

#[account]
pub struct Vault {
    pub manager: Pubkey,
    pub total_balance: u64,
    pub user_balances: BTreeMap<Pubkey, u64>,
}

#[program]
pub mod token_vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.manager = *ctx.accounts.user.key;
        vault.total_balance = 0;
        vault.user_balances = BTreeMap::new();
        Ok(())
    }

    pub fn deposit_in_vault(ctx: Context<DepositInVault>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;

        vault.total_balance = vault
            .total_balance
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;

        let user_key = ctx.accounts.owner.key();

        let user_balance = vault.user_balances.entry(user_key).or_insert(0);

        *user_balance = user_balance
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;

        let transfer_instruction = Transfer {
            from: ctx.accounts.from_ata.to_account_info(),
            to: ctx.accounts.to_ata.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        transfer(CpiContext::new(cpi_program, transfer_instruction), amount)?;

        Ok(())
    }

    pub fn withdraw_from_vault(ctx: Context<WithdrawFromVault>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;

        let user_key = *ctx.accounts.owner.key;

        let user_balance = vault
            .user_balances
            .get_mut(&user_key)
            .ok_or(ErrorCode::NoDepositRecord)?;

        if amount > *user_balance {
            return Err(ErrorCode::InsufficientUserBalance.into());
        }

        let new_user_balance = user_balance
            .checked_sub(amount)
            .ok_or(ErrorCode::Underflow)?;

        *user_balance = new_user_balance;

        if *user_balance == 0 {
            vault.user_balances.remove(&user_key);
        }

        vault.total_balance = vault
            .total_balance
            .checked_sub(amount)
            .ok_or(ErrorCode::Underflow)?;

        let transfer_instruction = Transfer {
            from: ctx.accounts.from_ata.to_account_info(),
            to: ctx.accounts.to_ata.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        transfer(CpiContext::new(cpi_program, transfer_instruction), amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(init, payer = user, space = 8 + 32 + 8 + (32 + 8) * 100)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositInVault<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub from_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to_ata: Account<'info, TokenAccount>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawFromVault<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub from_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to_ata: Account<'info, TokenAccount>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum ErrorCode {
    Overflow,
    Underflow,
    InsufficientUserBalance,
    NoDepositRecord,
}
