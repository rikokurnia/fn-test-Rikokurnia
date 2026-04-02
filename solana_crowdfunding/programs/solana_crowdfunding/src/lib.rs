use anchor_lang::prelude::*;
use anchor_lang::solana_program::{system_instruction, program::invoke_signed};

declare_id!("4g55JHQDi9diLma9XsAwhdBNSkuEVBK9vNExEMPmcUTK");

#[program]
pub mod solana_crowdfunding {
    use super::*;

    /// 1. Create Campaign
    /// What it does: Creator sets up a new fundraising campaign
    pub fn create_campaign(ctx: Context<CreateCampaign>, goal: u64, deadline: i64) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        // Validate deadline is in the future
        require!(deadline > clock.unix_timestamp, CrowdError::InvalidDeadline);
        // Ensure goal is positive
        require!(goal > 0, CrowdError::InvalidGoal);

        // Store campaign data
        campaign.creator = ctx.accounts.creator.key();
        campaign.goal = goal;
        campaign.raised = 0;
        campaign.deadline = deadline;
        campaign.claimed = false;

        msg!("Campaign created: goal={}, deadline={}", goal, deadline);
        Ok(())
    }

    /// 2. Contribute
    /// What it does: Donor sends SOL to the campaign vault (PDA)
    pub fn contribute(ctx: Context<Contribute>, amount: u64) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        // Validate contribution only while campaign is active
        require!(clock.unix_timestamp < campaign.deadline, CrowdError::CampaignEnded);

        // Update campaign raised amount
        campaign.raised += amount;

        // Track individual donor's contribution for refunds
        let receipt = &mut ctx.accounts.receipt;
        if receipt.amount == 0 {
            receipt.donor = ctx.accounts.donor.key();
            receipt.campaign = campaign.key();
        }
        receipt.amount += amount;

        // Transfer SOL from donor to campaign vault (PDA)
        let ix = system_instruction::transfer(
            &ctx.accounts.donor.key(),
            &ctx.accounts.vault.key(),
            amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.donor.to_account_info(),
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        msg!("Contributed: {} lamports, total={}", amount, campaign.raised);
        Ok(())
    }

    /// 3. Withdraw
    /// What it does: Creator claims funds if campaign succeeded
    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        // Conditions: Goal met, Deadline passed, Caller is creator, Not already claimed
        require!(campaign.raised >= campaign.goal, CrowdError::GoalNotMet);
        require!(clock.unix_timestamp >= campaign.deadline, CrowdError::CampaignActive);
        require!(!campaign.claimed, CrowdError::AlreadyClaimed);

        campaign.claimed = true;
        let amount = campaign.raised;

        // Transfer all SOL from vault to creator using invoke_signed
        let campaign_key = campaign.key();
        let seeds = &[b"vault", campaign_key.as_ref(), &[ctx.bumps.vault]];
        let signer_seeds = &[&seeds[..]];

        let ix = system_instruction::transfer(
            &ctx.accounts.vault.key(),
            &ctx.accounts.creator.key(),
            amount,
        );

        invoke_signed(
            &ix,
            &[
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.creator.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer_seeds,
        )?;

        msg!("Withdrawn: {} lamports", amount);
        Ok(())
    }

    /// 4. Refund
    /// What it does: Donor gets money back if campaign failed
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        let campaign = &ctx.accounts.campaign;
        let receipt = &mut ctx.accounts.receipt;
        let clock = Clock::get()?;

        // Conditions: Goal NOT met, Deadline passed
        require!(campaign.raised < campaign.goal, CrowdError::GoalMet);
        require!(clock.unix_timestamp >= campaign.deadline, CrowdError::CampaignActive);
        require!(receipt.amount > 0, CrowdError::NoContribution);

        let amount = receipt.amount;
        receipt.amount = 0; // Prevent double refund

        // Transfer donor's contribution back from vault using invoke_signed
        let campaign_key = campaign.key();
        let seeds = &[b"vault", campaign_key.as_ref(), &[ctx.bumps.vault]];
        let signer_seeds = &[&seeds[..]];

        let ix = system_instruction::transfer(
            &ctx.accounts.vault.key(),
            &ctx.accounts.donor.key(),
            amount,
        );

        invoke_signed(
            &ix,
            &[
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.donor.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer_seeds,
        )?;

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
        space = 8 + 32 + 8 + 8 + 8 + 1, // Corrected Campaign space
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
    /// CHECK: This is a PDA used as a SOL vault. It's a simple system account.
    #[account(
        mut,
        seeds = [b"vault", campaign.key().as_ref()],
        bump
    )]
    pub vault: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = donor,
        space = 8 + 32 + 32 + 8, // Corrected Receipt space
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
        bump
    )]
    pub campaign: Account<'info, Campaign>,
    /// CHECK: This is the SOL vault for the campaign.
    #[account(
        mut,
        seeds = [b"vault", campaign.key().as_ref()],
        bump
    )]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub donor: Signer<'info>,
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    /// CHECK: This is the SOL vault for the campaign.
    #[account(
        mut,
        seeds = [b"vault", campaign.key().as_ref()],
        bump
    )]
    pub vault: UncheckedAccount<'info>,
    #[account(
        mut,
        has_one = donor,
        has_one = campaign,
        seeds = [b"receipt", campaign.key().as_ref(), donor.key().as_ref()],
        bump
    )]
    pub receipt: Account<'info, Receipt>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Campaign {
    pub creator: Pubkey,    // Who created this
    pub goal: u64,          // Target amount
    pub raised: u64,        // Current amount
    pub deadline: i64,      // When it expires
    pub claimed: bool,      // Already withdrawn?
}

#[account]
pub struct Receipt {
    pub campaign: Pubkey,
    pub donor: Pubkey,
    pub amount: u64,
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