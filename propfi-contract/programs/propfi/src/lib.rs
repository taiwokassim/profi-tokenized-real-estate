use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo};

/// I will replace with my actual program ID after deployment.
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkgVh9r7v6v7P");

#[program]
pub mod propfi {
    use super::*;

    // -------------------------------------------------------------------------
    // INITIALIZE (existing)
    // -------------------------------------------------------------------------
    /// Initializes a new property for fractional ownership.
    /// - Creates a `Property` PDA to store metadata.
    /// - Creates a new SPL token mint to represent shares.
    /// - Sets the initial share supply and ownership details.
    pub fn initialize_property(
        ctx: Context<InitializeProperty>,
        total_shares: u64,
    ) -> Result<()> {
        require!(total_shares > 0, CustomError::ZeroAmount);

        let property = &mut ctx.accounts.property;

        // Persist property metadata
        property.owner = *ctx.accounts.owner.key;
        property.total_shares = total_shares;
        property.available_shares = total_shares;
        property.rent_pool = 0;
        property.bump = *ctx.bumps.get("property").unwrap();
        property.share_mint = ctx.accounts.share_mint.key();
        property.is_listed = false;
        property.share_price_lamports = 0; // set when listed
        property.created_at = Clock::get()?.unix_timestamp;

        emit!(PropertyInitialized {
            property: property.key(),
            owner: property.owner,
            total_shares,
        });
        Ok(())
    }

    // -------------------------------------------------------------------------
    // LIST / UPDATE
    // -------------------------------------------------------------------------
    /// Owner lists the property shares for sale by setting a price per share.
    pub fn list_property(ctx: Context<ListProperty>, price_per_share_lamports: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;
        require!(ctx.accounts.owner.key() == property.owner, CustomError::NotOwner);
        require!(price_per_share_lamports > 0, CustomError::PriceTooLow);

        property.is_listed = true;
        property.share_price_lamports = price_per_share_lamports;

        emit!(PropertyListed {
            property: property.key(),
            owner: property.owner,
            price_per_share_lamports,
        });
        Ok(())
    }

    /// Owner can update the listing price and optionally add new shares
    /// (increases `total_shares` and `available_shares`).
    pub fn update_property(
        ctx: Context<UpdateProperty>,
        new_price_per_share_lamports: Option<u64>,
        add_shares: Option<u64>,
    ) -> Result<()> {
        let property = &mut ctx.accounts.property;
        require!(ctx.accounts.owner.key() == property.owner, CustomError::NotOwner);

        if let Some(p) = new_price_per_share_lamports {
            require!(p > 0, CustomError::PriceTooLow);
            property.share_price_lamports = p;
        }

        if let Some(extra) = add_shares {
            if extra > 0 {
                property.total_shares = property
                    .total_shares
                    .checked_add(extra)
                    .ok_or(CustomError::MathError)?;
                property.available_shares = property
                    .available_shares
                    .checked_add(extra)
                    .ok_or(CustomError::MathError)?;
            }
        }

        emit!(PropertyUpdated {
            property: property.key(),
            owner: property.owner,
            share_price_lamports: property.share_price_lamports,
            total_shares: property.total_shares,
            available_shares: property.available_shares,
        });
        Ok(())
    }

    // -------------------------------------------------------------------------
    // BUY FLOW (payment + mint shares)
    // -------------------------------------------------------------------------
    /// Buyer purchases `amount` shares at the current listed price.
    /// - Transfers SOL from buyer to owner (no escrow in v1).
    /// - Mints `amount` share tokens to buyer ATA.
    pub fn buy_property(ctx: Context<BuyProperty>, amount: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;
        require!(amount > 0, CustomError::ZeroAmount);
        require!(property.is_listed, CustomError::NotListed);
        require!(amount <= property.available_shares, CustomError::NotEnoughShares);
        require!(property.share_price_lamports > 0, CustomError::PriceTooLow);

        // Compute total cost: amount * price
        let total_cost = amount
            .checked_mul(property.share_price_lamports)
            .ok_or(CustomError::MathError)?;

        // Transfer lamports from buyer to owner
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.owner.to_account_info(),
                },
            ),
            total_cost,
        )?;

        // Reduce available shares
        property.available_shares = property
            .available_shares
            .checked_sub(amount)
            .ok_or(CustomError::MathError)?;

        // Mint shares to buyer's ATA using property PDA as mint authority
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

        emit!(PropertyPurchased {
            property: property.key(),
            buyer: ctx.accounts.buyer.key(),
            amount,
            total_cost,
        });
        Ok(())
    }

    // -------------------------------------------------------------------------
    // EXISTING BUY_SHARES (kept for compatibility)
    // -------------------------------------------------------------------------
    /// Legacy/low-level share mint without payment logic (kept for compatibility).
    pub fn buy_shares(ctx: Context<BuyShares>, amount: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;
        require!(amount > 0, CustomError::ZeroAmount);
        require!(amount <= property.available_shares, CustomError::NotEnoughShares);

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

    // -------------------------------------------------------------------------
    // RENT (existing)
    // -------------------------------------------------------------------------
    /// Deposit rent units (accounting only in v1).
    pub fn deposit_rent(ctx: Context<DepositRent>, amount: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;
        require!(amount > 0, CustomError::ZeroAmount);
        property.rent_pool = property
            .rent_pool
            .checked_add(amount)
            .ok_or(CustomError::MathError)?;
        Ok(())
    }

    /// Reset rent pool (placeholder for future proportional distribution).
    pub fn distribute_rent(ctx: Context<DistributeRent>) -> Result<()> {
        let property = &mut ctx.accounts.property;
        property.rent_pool = 0;
        Ok(())
    }
}

// ============================================================================
// ACCOUNT CONTEXTS
// ============================================================================

#[derive(Accounts)]
pub struct InitializeProperty<'info> {
    /// Property account (PDA) derived from owner
    #[account(
        init,
        payer = owner,
        space = 8 + Property::LEN,
        seeds = [b"property", owner.key().as_ref()],
        bump
    )]
    pub property: Account<'info, Property>,

    /// SPL token mint for property shares; property PDA is the mint authority
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
pub struct ListProperty<'info> {
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump,
        has_one = owner
    )]
    pub property: Account<'info, Property>,
    /// CHECK: we only need lamport recipient; has_one enforces identity
    pub owner: SystemAccount<'info>,
}

#[derive(Accounts)]
pub struct UpdateProperty<'info> {
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump,
        has_one = owner
    )]
    pub property: Account<'info, Property>,
    /// CHECK: used as identity and lamport recipient; protected by has_one
    pub owner: SystemAccount<'info>,
}

#[derive(Accounts)]
pub struct BuyProperty<'info> {
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump,
        has_one = owner
    )]
    pub property: Account<'info, Property>,

    #[account(mut)]
    pub share_mint: Account<'info, Mint>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    /// Buyer's ATA for share mint
    #[account(
        mut,
        associated_token::mint = share_mint,
        associated_token::authority = buyer
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,

    /// Seller (property owner) receives SOL
    /// CHECK: recipient of lamports; identity enforced by `has_one = owner`
    #[account(mut)]
    pub owner: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
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
    pub associated_token_program: Program<'info, AssociatedToken>,
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

// ============================================================================
// STATE
// ============================================================================

#[account]
pub struct Property {
    pub owner: Pubkey,              // 32
    pub total_shares: u64,          // 8
    pub available_shares: u64,      // 8
    pub rent_pool: u64,             // 8 (accounting only in v1)
    pub share_mint: Pubkey,         // 32
    pub bump: u8,                   // 1
    pub is_listed: bool,            // 1
    pub share_price_lamports: u64,  // 8
    pub created_at: i64,            // 8
}
impl Property {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 32 + 1 + 1 + 8 + 8;
}

// ============================================================================
// EVENTS
// ============================================================================

#[event]
pub struct PropertyInitialized {
    pub property: Pubkey,
    pub owner: Pubkey,
    pub total_shares: u64,
}

#[event]
pub struct PropertyListed {
    pub property: Pubkey,
    pub owner: Pubkey,
    pub price_per_share_lamports: u64,
}

#[event]
pub struct PropertyUpdated {
    pub property: Pubkey,
    pub owner: Pubkey,
    pub share_price_lamports: u64,
    pub total_shares: u64,
    pub available_shares: u64,
}

#[event]
pub struct PropertyPurchased {
    pub property: Pubkey,
    pub buyer: Pubkey,
    pub amount: u64,
    pub total_cost: u64,
}

// ============================================================================
// ERRORS
// ============================================================================

#[error_code]
pub enum CustomError {
    #[msg("Not enough shares available")] NotEnoughShares,
    #[msg("Math error occurred")] MathError,
    #[msg("You are not the property owner")] NotOwner,
    #[msg("Property is not listed for sale")] NotListed,
    #[msg("Price must be greater than zero")] PriceTooLow,
    #[msg("Amount must be greater than zero")] ZeroAmount,
}
