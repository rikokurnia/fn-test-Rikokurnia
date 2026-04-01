use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("4g55JHQDi9diLma9XsAwhdBNSkuEVBK9vNExEMPmcUTK");

#[program]
pub mod solana_crowdfunding {
    use super::*;

    pub fn create_campaign(ctx: Context<CreateCampaign>, goal: u64, deadline: i64) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        // Validasi: Deadline harus di masa depan
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

    pub fn contribute(ctx: Context<Contribute>, amount: u64) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let receipt = &mut ctx.accounts.receipt;
        let clock = Clock::get()?;

        // Validasi: Tidak bisa donasi jika waktu sudah habis
        require!(clock.unix_timestamp < campaign.deadline, CrowdError::CampaignEnded);

        // Inisialisasi data kwitansi jika ini donasi pertama dari donor ini
        if receipt.amount == 0 {
            receipt.donor = ctx.accounts.donor.key();
            receipt.campaign = campaign.key();
            receipt.bump = ctx.bumps.receipt;
        }

        // Catat donasi
        receipt.amount += amount;
        campaign.raised += amount;

        // Transfer SOL dari Donor ke PDA Campaign (Vault)
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.donor.to_account_info(),
                to: campaign.to_account_info(),
            }
        );
        system_program::transfer(cpi_ctx, amount)?;

        msg!("Contributed: {} lamports, total={}", amount, campaign.raised);
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        // Syarat Withdraw Mutlak
        require!(campaign.raised >= campaign.goal, CrowdError::GoalNotMet);
        require!(clock.unix_timestamp >= campaign.deadline, CrowdError::CampaignActive);
        require!(!campaign.claimed, CrowdError::AlreadyClaimed);

        campaign.claimed = true;
        let amount = campaign.raised;

        // Transfer SOL dari PDA Vault kembali ke Creator
        **campaign.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.creator.to_account_info().try_borrow_mut_lamports()? += amount;

        msg!("Withdrawn: {} lamports", amount);
        Ok(())
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        let campaign = &ctx.accounts.campaign;
        let receipt = &mut ctx.accounts.receipt;
        let clock = Clock::get()?;

        // Syarat Refund Mutlak
        require!(campaign.raised < campaign.goal, CrowdError::GoalMet);
        require!(clock.unix_timestamp >= campaign.deadline, CrowdError::CampaignActive);
        require!(receipt.amount > 0, CrowdError::NoContribution);

        let amount = receipt.amount;
        receipt.amount = 0; // Cegah double refund

        // Transfer SOL dari PDA Vault kembali ke Donor
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

#[account]
pub struct Campaign {
    pub creator: Pubkey,
    pub goal: u64,
    pub raised: u64,
    pub deadline: i64,
    pub claimed: bool,
    pub bump: u8,
}

#[account]
pub struct Receipt {
    pub campaign: Pubkey,
    pub donor: Pubkey,
    pub amount: u64,
    pub bump: u8,
}

#[error_code]
pub enum CrowdError {
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