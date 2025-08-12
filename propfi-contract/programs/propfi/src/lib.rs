use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkgVh9r7v6v7P"); // I will replace with my actual program ID after deployment

#[program]
pub mod propfi {
    use super::*;

    /// Initializes a new property with a set number of shares
    pub fn initialize_property(ctx: Context<InitializeProperty>, total_shares: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;
        property.owner = *ctx.accounts.owner.key;
        property.total_shares = total_shares;
        property.available_shares = total_shares;
        property.rent_pool = 0;
        property.bump = *ctx.bumps.get("property").unwrap();
        Ok(())
    }

    /// Allows a user to buy shares from the property
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
        Ok(())
    }

    /// Owner deposits rent into the property’s rent pool
    pub fn deposit_rent(ctx: Context<DepositRent>, amount: u64) -> Result<()> {
        let property = &mut ctx.accounts.property;
        property.rent_pool = property
            .rent_pool
            .checked_add(amount)
            .ok_or(CustomError::MathError)?;
        Ok(())
    }

    /// Distributes rent (placeholder: clears rent pool for now)
    pub fn distribute_rent(_ctx: Context<DistributeRent>) -> Result<()> {
        let property = &mut _ctx.accounts.property;
        // Later: Implement proportional distribution to shareholders
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
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
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
    pub buyer: Signer<'info>,
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
    pub owner: Pubkey,         // 32 bytes
    pub total_shares: u64,     // 8 bytes
    pub available_shares: u64, // 8 bytes
    pub rent_pool: u64,        // 8 bytes
    pub bump: u8,              // 1 byte
}

impl Property {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 1;
}

#[error_code]
pub enum CustomError {
    #[msg("Not enough shares available")]
    NotEnoughShares,
    #[msg("Math error occurred")]
    MathError,
}

