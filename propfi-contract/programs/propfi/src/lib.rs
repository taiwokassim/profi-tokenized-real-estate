use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo};

/// Unique program ID — must match your deployed program's address.
/// I will replace with my actual program ID after deployment.
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkgVh9r7v6v7P");

#[program]
pub mod propfi {
    use super::*;

    /// Initializes a new property for fractional ownership.
    /// - Creates a `Property` account to store metadata.
    /// - Mints a new SPL token to represent property shares.
    /// - Sets the initial share supply and ownership details.
    pub fn initialize_property(
        ctx: Context<InitializeProperty>,
        total_shares: u64
    ) -> Result<()> {
        let property = &mut ctx.accounts.property;

        // Save property details
        property.owner = *ctx.accounts.owner.key;
        property.total_shares = total_shares;
        property.available_shares = total_shares;
        property.rent_pool = 0;
        property.bump = *ctx.bumps.get("property").unwrap();
        property.share_mint = ctx.accounts.share_mint.key();

        Ok(())
    }

    /// Allows a user to purchase a specified number of shares in a property.
    /// - Verifies that enough shares are available.
    /// - Reduces available share count.
    /// - Mints the purchased shares to the buyer's wallet.
    pub fn buy_shares(ctx: Context<BuyShares>, amount: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;

        // Ensure enough shares remain
        require!(
            amount <= property.available_shares,
            CustomError::NotEnoughShares
        );

        // Reduce available shares
        property.available_shares = property
            .available_shares
            .checked_sub(amount)
            .ok_or(CustomError::MathError)?;

        // Mint shares to buyer's token account
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.share_mint.to_account_info(),
                    to: ctx.accounts.buyer_token_account.to_account_info(),
                    authority: ctx.accounts.property.to_account_info(),
                },
                &[&[b"property", property.owner.as_ref(), &[property.bump]]], // PDA signer
            ),
            amount,
        )?;

        Ok(())
    }

    /// Allows the property owner (or authorized payer) to deposit rent into the rent pool.
    /// - Rent is accumulated for future distribution to shareholders.
    pub fn deposit_rent(ctx: Context<DepositRent>, amount: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;

        property.rent_pool = property
            .rent_pool
            .checked_add(amount)
            .ok_or(CustomError::MathError)?;

        Ok(())
    }

    /// Distributes accumulated rent to shareholders.
    /// - Currently just resets the rent pool to 0.
    /// - Future upgrade: send proportional rent to each token holder.
    pub fn distribute_rent(ctx: Context<DistributeRent>) -> Result<()> {
        let property = &mut ctx.accounts.property;
        property.rent_pool = 0;
        Ok(())
    }
}

//////////////////////////////////////
// Account Contexts
//////////////////////////////////////

/// Context for initializing a property and its share mint.
#[derive(Accounts)]
pub struct InitializeProperty<'info> {
    /// Property account storing metadata. PDA derived from owner key.
    #[account(
        init,
        payer = owner,
        space = 8 + Property::LEN,
        seeds = [b"property", owner.key().as_ref()],
        bump
    )]
    pub property: Account<'info, Property>,

    /// SPL token mint for property shares.
    #[account(
        init,
        payer = owner,
        mint::decimals = 0, // No fractional shares
        mint::authority = property
    )]
    pub share_mint: Account<'info, Mint>,

    #[account(mut)]
    pub owner: Signer<'info>,                 // Wallet creating the property
    pub system_program: Program<'info, System>, // Required by Anchor
    pub token_program: Program<'info, Token>,   // SPL Token Program
    pub rent: Sysvar<'info, Rent>,              // Rent sysvar for account creation
}

/// Context for buying shares.
#[derive(Accounts)]
pub struct BuyShares<'info> {
    /// Property account to track shares.
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump
    )]
    pub property: Account<'info, Property>,

    #[account(mut)]
    pub share_mint: Account<'info, Mint>, // SPL token mint for shares

    #[account(mut)]
    pub buyer: Signer<'info>, // User buying the shares

    /// Buyer’s associated token account for receiving shares.
    #[account(
        mut,
        associated_token::mint = share_mint,
        associated_token::authority = buyer
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>, // SPL Token Program
}

/// Context for depositing rent into a property.
#[derive(Accounts)]
pub struct DepositRent<'info> {
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump
    )]
    pub property: Account<'info, Property>,

    pub payer: Signer<'info>, // The account paying the rent
}

/// Context for distributing rent to shareholders.
#[derive(Accounts)]
pub struct DistributeRent<'info> {
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump,
        has_one = owner
    )]
    pub property: Account<'info, Property>,

    pub owner: Signer<'info>, // Property owner
}

//////////////////////////////////////
// State Accounts
//////////////////////////////////////

/// On-chain storage for a property.
#[account]
pub struct Property {
    pub owner: Pubkey,         // Property owner
    pub total_shares: u64,     // Total number of shares
    pub available_shares: u64, // Remaining unsold shares
    pub rent_pool: u64,        // Accumulated rent
    pub share_mint: Pubkey,    // Mint address for share tokens
    pub bump: u8,              // PDA bump
}
impl Property {
    /// Size of Property account (in bytes).
    pub const LEN: usize = 32 + 8 + 8 + 8 + 32 + 1;
}

//////////////////////////////////////
// Custom Errors
//////////////////////////////////////

#[error_code]
pub enum CustomError {
    #[msg("Not enough shares available")]
    NotEnoughShares,
    #[msg("Math error occurred")]
    MathError,
}
