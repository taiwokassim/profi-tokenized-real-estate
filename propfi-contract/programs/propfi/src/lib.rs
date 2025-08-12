use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkgVh9r7v6v7P");

#[program]
pub mod propfi {
    use super::*;

    pub fn initialize_property(
        ctx: Context<InitializeProperty>,
        total_shares: u64
    ) -> Result<()> {
        let property = &mut ctx.accounts.property;
        property.owner = *ctx.accounts.owner.key;
        property.total_shares = total_shares;
        property.available_shares = total_shares;
        property.rent_pool = 0;
        property.bump = *ctx.bumps.get("property").unwrap();
        property.share_mint = ctx.accounts.share_mint.key();
        Ok(())
    }

    pub fn buy_shares(ctx: Context<BuyShares>, amount: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;
        require!(
            amount <= property.available_shares,
            CustomError::NotEnoughShares
        );
        property.available_shares = property
            .available_shares
            .checked_sub(amount)
            .ok_or(CustomError::MathError)?;

        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.share_mint.to_account_info(),
                    to: ctx.accounts.buyer_token_account.to_account_info(),
                    authority: ctx.accounts.property.to_account_info(),
                },
                &[&[b"property", property.owner.as_ref(), &[property.bump]]],
            ),
            amount,
        )?;

        Ok(())
    }

    pub fn deposit_rent(ctx: Context<DepositRent>, amount: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;
        property.rent_pool = property
            .rent_pool
            .checked_add(amount)
            .ok_or(CustomError::MathError)?;
        Ok(())
    }

    pub fn distribute_rent(ctx: Context<DistributeRent>) -> Result<()> {
        let property = &mut ctx.accounts.property;
        property.rent_pool = 0;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeProperty<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + Property::LEN,
        seeds = [b"property", owner.key().as_ref()],
        bump
    )]
    pub property: Account<'info, Property>,

    #[account(
        init,
        payer = owner,
        mint::decimals = 0,
        mint::authority = property
    )]
    pub share_mint: Account<'info, Mint>,

    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct BuyShares<'info> {
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump
    )]
    pub property: Account<'info, Property>,

    #[account(mut)]
    pub share_mint: Account<'info, Mint>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = share_mint,
        associated_token::authority = buyer
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DepositRent<'info> {
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump
    )]
    pub property: Account<'info, Property>,
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct DistributeRent<'info> {
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump,
        has_one = owner
    )]
    pub property: Account<'info, Property>,
    pub owner: Signer<'info>,
}

#[account]
pub struct Property {
    pub owner: Pubkey,
    pub total_shares: u64,
    pub available_shares: u64,
    pub rent_pool: u64,
    pub share_mint: Pubkey,
    pub bump: u8,
}
impl Property {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 32 + 1;
}

#[error_code]
pub enum CustomError {
    #[msg("Not enough shares available")]
    NotEnoughShares,
    #[msg("Math error occurred")]
    MathError,
}
