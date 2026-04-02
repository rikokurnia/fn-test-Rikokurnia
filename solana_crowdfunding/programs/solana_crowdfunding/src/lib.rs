use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("4g55JHQDi9diLma9XsAwhdBNSkuEVBK9vNExEMPmcUTK");

#[program]
pub mod solana_crowdfunding {
    use super::*;

    /// Creates a new crowdfunding campaign.
    ///
    /// * `goal` - Target amount in lamports.
    /// * `deadline` - Unix timestamp when the campaign ends.
    pub fn create_campaign(ctx: Context<CreateCampaign>, goal: u64, deadline: i64) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        // Ensure the goal is at least some amount
        require!(goal > 0, CrowdError::InvalidGoal);
        // Validate: Deadline must be in the future
        require!(deadline > clock.unix_timestamp, CrowdError::InvalidDeadline);

        campaign.creator = ctx.accounts.creator.key();
        campaign.goal = goal;
        campaign.raised = 0;
        campaign.deadline = deadline;
        campaign.claimed = false;
        campaign.bump = ctx.bumps.campaign;

        msg!("Campaign created: goal={}, deadline={}", goal, deadline);
        Ok(())
    }

    /// Allows a user to contribute SOL to a specific campaign.
    ///
    /// * `amount` - Amount to contribute in lamports.
    pub fn contribute(ctx: Context<Contribute>, amount: u64) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let receipt = &mut ctx.accounts.receipt;
        let clock = Clock::get()?;

        // Validate: Contribution only allowed while campaign is active
        require!(clock.unix_timestamp < campaign.deadline, CrowdError::CampaignEnded);

        // Initialize receipt data if this is the first contribution from this donor
        if receipt.amount == 0 {
            receipt.donor = ctx.accounts.donor.key();
            receipt.campaign = campaign.key();
            receipt.bump = ctx.bumps.receipt;
        }

        // Record contribution
        receipt.amount += amount;
        campaign.raised += amount;

        // Transfer SOL from Donor to Campaign PDA (Vault)
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.donor.to_account_info(),
                to: campaign.to_account_info(),
            }
        );
        system_program::transfer(cpi_ctx, amount)?;

        msg!("Contributed: {} lamports, total raised={}", amount, campaign.raised);
        Ok(())
    }

    /// Allows the campaign creator to withdraw funds if the goal was met and the deadline passed.
    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        // Withdrawal requirements
        require!(campaign.raised >= campaign.goal, CrowdError::GoalNotMet);
        require!(clock.unix_timestamp >= campaign.deadline, CrowdError::CampaignActive);
        require!(!campaign.claimed, CrowdError::AlreadyClaimed);

        campaign.claimed = true;
        let amount = campaign.raised;

        // Transfer SOL from Campaign PDA Vault back to Creator
        **campaign.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.creator.to_account_info().try_borrow_mut_lamports()? += amount;

        msg!("Withdrawn: {} lamports", amount);
        Ok(())
    }

    /// Allows donors to claim a refund if the campaign failed to reach its goal by the deadline.
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        let campaign = &ctx.accounts.campaign;
        let receipt = &mut ctx.accounts.receipt;
        let clock = Clock::get()?;

        // Refund requirements
        require!(campaign.raised < campaign.goal, CrowdError::GoalMet);
        require!(clock.unix_timestamp >= campaign.deadline, CrowdError::CampaignActive);
        require!(receipt.amount > 0, CrowdError::NoContribution);

        let amount = receipt.amount;
        receipt.amount = 0; // Guard against re-entrancy / double refund

        // Transfer SOL from Campaign PDA Vault back to Donor
        **campaign.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.donor.to_account_info().try_borrow_mut_lamports()? += amount;

        msg!("Refunded: {} lamports", amount);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateCampaign<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        init,
        payer = creator,
        space = 8 + 32 + 8 + 8 + 8 + 1 + 1, // Space untuk struct Campaign
        seeds = [b"campaign", creator.key().as_ref()],
        bump
    )]
    pub campaign: Account<'info, Campaign>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Contribute<'info> {
    #[account(mut)]
    pub donor: Signer<'info>,
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    #[account(
        init_if_needed,
        payer = donor,
        space = 8 + 32 + 32 + 8 + 1, // Space untuk struct Receipt
        seeds = [b"receipt", campaign.key().as_ref(), donor.key().as_ref()],
        bump
    )]
    pub receipt: Account<'info, Receipt>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        mut,
        has_one = creator,
        seeds = [b"campaign", creator.key().as_ref()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, Campaign>,
}

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub donor: Signer<'info>,
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    #[account(
        mut,
        has_one = donor,
        has_one = campaign,
        seeds = [b"receipt", campaign.key().as_ref(), donor.key().as_ref()],
        bump = receipt.bump
    )]
    pub receipt: Account<'info, Receipt>,
}

/// Metadata and state of a single crowdfunding campaign.
#[account]
pub struct Campaign {
    /// Public key of the campaign creator.
    pub creator: Pubkey,
    /// Targeted amount to raise in lamports.
    pub goal: u64,
    /// Total amount currently raised in lamports.
    pub raised: u64,
    /// Unix timestamp when the campaign expires.
    pub deadline: i64,
    /// Whether the funds have been successfully claimed by the creator.
    pub claimed: bool,
    /// Bump seed for PDA.
    pub bump: u8,
}

/// Record of an individual donor's contribution to a specific campaign.
#[account]
pub struct Receipt {
    /// Public key of the campaign account.
    pub campaign: Pubkey,
    /// Public key of the donor.
    pub donor: Pubkey,
    /// Total amount contributed by this donor in lamports.
    pub amount: u64,
    /// Bump seed for PDA.
    pub bump: u8,
}

#[error_code]
pub enum CrowdError {
    #[msg("Goal must be greater than 0.")]
    InvalidGoal,
    #[msg("Deadline must be in the future.")]
    InvalidDeadline,
    #[msg("Campaign deadline has passed.")]
    CampaignEnded,
    #[msg("Campaign is still active.")]
    CampaignActive,
    #[msg("Goal not met.")]
    GoalNotMet,
    #[msg("Goal was met, cannot refund.")]
    GoalMet,
    #[msg("Funds already claimed.")]
    AlreadyClaimed,
    #[msg("No contribution found to refund.")]
    NoContribution,
}