use anchor_lang::prelude::*;

pub const COMPLIANCE_STATUS_PENDING: u8 = 0;
pub const COMPLIANCE_STATUS_APPROVED: u8 = 1;
pub const COMPLIANCE_STATUS_REJECTED: u8 = 2;
pub const COMPLIANCE_STATUS_EXPIRED: u8 = 3;
pub const COMPLIANCE_STATUS_REVOKED: u8 = 4;

/// Compact on-chain Shariah compliance record for a tradable asset mint.
///
/// Designed to minimize Solana account rent and compute overhead while providing
/// an un-bypassable on-chain barrier:
/// - Full financial statements, legal contracts, and screening dossiers remain off-chain.
/// - Only the asset mint, compliance status enum, 32-byte policy hash, 32-byte evidence hash,
///   and audit expiration timestamp are stored on-chain.
#[account]
#[derive(InitSpace)]
pub struct AssetCompliance {
    /// Underlying SPL token mint of the tradable asset
    pub asset_mint: Pubkey,
    /// Compliance status: 0 = Pending, 1 = Approved, 2 = Rejected, 3 = Expired, 4 = Revoked
    pub status: u8,
    /// 32-byte cryptographic hash or identifier of the screening policy version
    pub policy_version: [u8; 32],
    /// 32-byte cryptographic evidence hash of the board review / statutory certificates
    pub evidence_hash: [u8; 32],
    /// Unix timestamp until which this Shariah audit review remains valid
    pub valid_until: i64,
    /// PDA bump seed
    pub bump: u8,
}
