use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::constants::COMPLIANCE_SEED;
use crate::state::AssetCompliance;

#[derive(Accounts)]
pub struct SetAssetCompliance<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub asset_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + AssetCompliance::INIT_SPACE,
        seeds = [COMPLIANCE_SEED, asset_mint.key().as_ref()],
        bump
    )]
    pub compliance: Account<'info, AssetCompliance>,

    pub system_program: Program<'info, System>,
}

pub fn set_asset_compliance(
    ctx: Context<SetAssetCompliance>,
    status: u8,
    policy_version: [u8; 32],
    evidence_hash: [u8; 32],
    valid_until: i64,
) -> Result<()> {
    let compliance = &mut ctx.accounts.compliance;
    compliance.asset_mint = ctx.accounts.asset_mint.key();
    compliance.status = status;
    compliance.policy_version = policy_version;
    compliance.evidence_hash = evidence_hash;
    compliance.valid_until = valid_until;
    compliance.bump = ctx.bumps.compliance;

    Ok(())
}
