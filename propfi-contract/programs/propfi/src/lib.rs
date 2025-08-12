use anchor_lang::prelude::*;

// This is the Program ID that Solana uses to identify your deployed program.
// IMPORTANT: Replace this with your actual program ID after running `anchor deploy`.
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkgVh9r7v6v7P");

#[program]
pub mod propfi {
    use super::*;

    /// Initialize a new property on-chain.
    /// This will create a PDA (Program Derived Address) to store property data.
    ///
    /// Arguments:
    /// - `total_shares`: Total number of fractional shares available for the property.
    pub fn initialize_property(ctx: Context<InitializeProperty>, total_shares: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;

        // Set the property owner to whoever called this instruction
        property.owner = *ctx.accounts.owner.key;

        // Set total and available shares
        property.total_shares = total_shares;
        property.available_shares = total_shares;

        // Start with an empty rent pool
        property.rent_pool = 0;

        // Store the bump (used for PDA)
        property.bump = *ctx.bumps.get("property").unwrap();

        Ok(())
    }

    /// Buy shares from a property.
    /// This reduces available shares but (currently) doesn't mint actual tokens.
    ///
    /// Arguments:
    /// - `amount`: Number of shares to buy.
    pub fn buy_shares(ctx: Context<BuyShares>, amount: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;

        // Ensure there are enough shares left
        require!(
            amount <= property.available_shares,
            CustomError::NotEnoughShares
        );

        // Deduct shares
        property.available_shares = property
            .available_shares
            .checked_sub(amount)
            .ok_or(CustomError::MathError)?;

        Ok(())
    }

    /// Deposit rent into the property’s rent pool.
    /// Later, this will be distributed proportionally to shareholders.
    ///
    /// Arguments:
    /// - `amount`: Amount of rent to deposit (in lamports or a simulated token unit).
    pub fn deposit_rent(ctx: Context<DepositRent>, amount: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;

        // Add to the rent pool
        property.rent_pool = property
            .rent_pool
            .checked_add(amount)
            .ok_or(CustomError::MathError)?;

        Ok(())
    }

    /// Distribute rent to shareholders.
    /// Currently just resets the rent pool (placeholder).
    pub fn distribute_rent(ctx: Context<DistributeRent>) -> Result<()> {
        let property = &mut ctx.accounts.property;

        // TODO: Implement proportional payouts based on share ownership
        property.rent_pool = 0;

        Ok(())
    }
}

//
// ---------------------- ACCOUNT STRUCTS ----------------------
//

// Context for initializing a property
#[derive(Accounts)]
pub struct InitializeProperty<'info> {
    // Create a PDA to store property details
    #[account(
        init,
        payer = owner,
        space = 8 + Property::LEN, // 8 bytes for account discriminator + struct size
        seeds = [b"property", owner.key().as_ref()], // Unique seed for PDA
        bump
    )]
    pub property: Account<'info, Property>,

    // The signer (property creator)
    #[account(mut)]
    pub owner: Signer<'info>,

    // System program (needed for creating accounts)
    pub system_program: Program<'info, System>,
}

// Context for buying shares
#[derive(Accounts)]
pub struct BuyShares<'info> {
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump
    )]
    pub property: Account<'info, Property>,

    #[account(mut)]
    pub buyer: Signer<'info>,
}

// Context for depositing rent
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

// Context for distributing rent
#[derive(Accounts)]
pub struct DistributeRent<'info> {
    #[account(
        mut,
        seeds = [b"property", property.owner.as_ref()],
        bump = property.bump,
        has_one = owner // Only the property owner can distribute rent
    )]
    pub property: Account<'info, Property>,

    pub owner: Signer<'info>,
}

//
// ---------------------- DATA STRUCTURES ----------------------
//

// Stores all details about a property
#[account]
pub struct Property {
    pub owner: Pubkey,         // (32 bytes) Wallet address of property owner
    pub total_shares: u64,     // (8 bytes) Total shares created for this property
    pub available_shares: u64, // (8 bytes) Shares still available for sale
    pub rent_pool: u64,        // (8 bytes) Rent available for distribution
    pub bump: u8,              // (1 byte) PDA bump seed
}

// Length of the Property struct (needed for account allocation)
impl Property {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 1; // Total size in bytes
}

//
// ---------------------- CUSTOM ERRORS ----------------------
//

// Custom error messages for better debugging
#[error_code]
pub enum CustomError {
    #[msg("Not enough shares available")]
    NotEnoughShares,

    #[msg("Math error occurred")]
    MathError,
}
