use crate::{accounts::*, rpc::SolanaRpcClient, SolanaError};
use borsh::BorshSerialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    system_program,
    transaction::Transaction,
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tracing::{info, instrument, warn};

// Instruction discriminators matching Anchor IDL and sha256("global:<name>")[..8]
pub const INITIALIZE_VAULT_DISCRIMINATOR: [u8; 8] = [48, 191, 163, 44, 71, 129, 63, 164];
pub const DEPOSIT_DISCRIMINATOR: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
pub const WITHDRAW_DISCRIMINATOR: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];
pub const UPDATE_POLICY_DISCRIMINATOR: [u8; 8] = [212, 245, 246, 7, 163, 151, 18, 57];
pub const EMERGENCY_EXIT_DISCRIMINATOR: [u8; 8] = [164, 174, 48, 163, 191, 65, 91, 245];
pub const SET_ASSET_COMPLIANCE_DISCRIMINATOR: [u8; 8] = [41, 131, 99, 238, 155, 69, 159, 226];
pub const EXECUTE_ACTION_DISCRIMINATOR: [u8; 8] = [246, 137, 105, 113, 247, 6, 223, 174];

pub struct AnchorClient {
    rpc: Arc<SolanaRpcClient>,
    program_id: Pubkey,
    allow_transactions: bool,
}

impl AnchorClient {
    pub fn new(rpc: Arc<SolanaRpcClient>) -> Self {
        Self {
            rpc,
            program_id: program_id(),
            allow_transactions: false, // Default is READ-ONLY mode
        }
    }

    pub fn with_program_id(mut self, program_id: Pubkey) -> Self {
        self.program_id = program_id;
        self
    }

    pub fn enable_transaction_submission(&mut self) {
        self.allow_transactions = true;
    }

    pub fn is_transaction_submission_enabled(&self) -> bool {
        self.allow_transactions
    }

    pub fn rpc(&self) -> &SolanaRpcClient {
        &self.rpc
    }

    pub fn program_id(&self) -> &Pubkey {
        &self.program_id
    }

    // ==========================================
    // READ-ONLY INTEGRATION METHODS
    // ==========================================

    #[instrument(skip(self), fields(vault_pda = %vault_pda))]
    pub async fn fetch_vault(&self, vault_pda: &Pubkey) -> Result<VaultAccount, SolanaError> {
        let info = self
            .rpc
            .get_account_info(vault_pda)
            .await?
            .ok_or_else(|| SolanaError::AccountNotFound(vault_pda.to_string()))?;
        parse_vault(&info.data)
    }

    #[instrument(skip(self), fields(policy_pda = %policy_pda))]
    pub async fn fetch_policy(&self, policy_pda: &Pubkey) -> Result<PolicyAccount, SolanaError> {
        let info = self
            .rpc
            .get_account_info(policy_pda)
            .await?
            .ok_or_else(|| SolanaError::AccountNotFound(policy_pda.to_string()))?;
        parse_policy(&info.data)
    }

    #[instrument(skip(self), fields(shares_pda = %shares_pda))]
    pub async fn fetch_user_shares(
        &self,
        shares_pda: &Pubkey,
    ) -> Result<UserSharesAccount, SolanaError> {
        let info = self
            .rpc
            .get_account_info(shares_pda)
            .await?
            .ok_or_else(|| SolanaError::AccountNotFound(shares_pda.to_string()))?;
        parse_user_shares(&info.data)
    }

    #[instrument(skip(self), fields(position_pda = %position_pda))]
    pub async fn fetch_position(
        &self,
        position_pda: &Pubkey,
    ) -> Result<PositionAccount, SolanaError> {
        let info = self
            .rpc
            .get_account_info(position_pda)
            .await?
            .ok_or_else(|| SolanaError::AccountNotFound(position_pda.to_string()))?;
        parse_position(&info.data)
    }

    #[instrument(skip(self), fields(execution_pda = %execution_pda))]
    pub async fn fetch_execution(
        &self,
        execution_pda: &Pubkey,
    ) -> Result<ExecutionAccount, SolanaError> {
        let info = self
            .rpc
            .get_account_info(execution_pda)
            .await?
            .ok_or_else(|| SolanaError::AccountNotFound(execution_pda.to_string()))?;
        parse_execution(&info.data)
    }

    #[instrument(skip(self), fields(token_account = %token_account))]
    pub async fn fetch_token_account(
        &self,
        token_account: &Pubkey,
    ) -> Result<SplTokenAccount, SolanaError> {
        let info = self
            .rpc
            .get_account_info(token_account)
            .await?
            .ok_or_else(|| SolanaError::AccountNotFound(token_account.to_string()))?;
        parse_spl_token(&info.data)
    }

    #[instrument(skip(self), fields(compliance_pda = %compliance_pda))]
    pub async fn fetch_asset_compliance(
        &self,
        compliance_pda: &Pubkey,
    ) -> Result<AssetComplianceAccount, SolanaError> {
        let info = self
            .rpc
            .get_account_info(compliance_pda)
            .await?
            .ok_or_else(|| SolanaError::AccountNotFound(compliance_pda.to_string()))?;
        parse_asset_compliance(&info.data)
    }

    // ==========================================
    // INSTRUCTION BUILDERS
    // ==========================================

    /// 1. Initialize Vault & linked Policy
    pub fn build_initialize_vault_ix(
        &self,
        authority: &Pubkey,
        asset_mint: &Pubkey,
        name: &str,
        symbol: &str,
        min_cash_bps: u16,
        max_position_bps: u16,
    ) -> Result<(Instruction, Pubkey, Pubkey), SolanaError> {
        let (vault_pda, _) = find_vault_pda(authority, name, &self.program_id);
        let (policy_pda, _) = find_policy_pda(&vault_pda, &self.program_id);

        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(&INITIALIZE_VAULT_DISCRIMINATOR);
        name.to_string()
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        symbol
            .to_string()
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        min_cash_bps
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        max_position_bps
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(policy_pda, false),
            AccountMeta::new_readonly(*asset_mint, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let ix = Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        Ok((ix, vault_pda, policy_pda))
    }

    /// 2. Update Policy (or initial policy configuration)
    #[allow(clippy::too_many_arguments)]
    pub fn build_update_policy_ix(
        &self,
        authority: &Pubkey,
        vault_pda: &Pubkey,
        min_cash_bps: u16,
        max_position_bps: u16,
        stop_loss_bps: u16,
        take_profit_bps: u16,
        rebalance_threshold_bps: u16,
        is_active: bool,
    ) -> Result<(Instruction, Pubkey), SolanaError> {
        let (policy_pda, _) = find_policy_pda(vault_pda, &self.program_id);

        let mut data = Vec::with_capacity(32);
        data.extend_from_slice(&UPDATE_POLICY_DISCRIMINATOR);
        min_cash_bps
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        max_position_bps
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        stop_loss_bps
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        take_profit_bps
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        rebalance_threshold_bps
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        is_active
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(*vault_pda, false),
            AccountMeta::new(policy_pda, false),
        ];

        let ix = Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        Ok((ix, policy_pda))
    }

    /// 3. Deposit into Vault
    pub fn build_deposit_ix(
        &self,
        user: &Pubkey,
        vault_pda: &Pubkey,
        user_asset_account: &Pubkey,
        vault_asset_account: &Pubkey,
        asset_mint: &Pubkey,
        amount: u64,
    ) -> Result<(Instruction, Pubkey), SolanaError> {
        let (user_shares_pda, _) = find_user_shares_pda(vault_pda, user, &self.program_id);
        let spl_token = Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).unwrap();

        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(&DEPOSIT_DISCRIMINATOR);
        amount
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new(user_shares_pda, false),
            AccountMeta::new(*user_asset_account, false),
            AccountMeta::new(*vault_asset_account, false),
            AccountMeta::new_readonly(*asset_mint, false),
            AccountMeta::new_readonly(spl_token, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let ix = Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        Ok((ix, user_shares_pda))
    }

    /// 4. Withdraw from Vault
    pub fn build_withdraw_ix(
        &self,
        user: &Pubkey,
        vault_pda: &Pubkey,
        user_asset_account: &Pubkey,
        vault_asset_account: &Pubkey,
        asset_mint: &Pubkey,
        shares_to_burn: u64,
    ) -> Result<(Instruction, Pubkey), SolanaError> {
        let (user_shares_pda, _) = find_user_shares_pda(vault_pda, user, &self.program_id);
        let spl_token = Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).unwrap();

        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(&WITHDRAW_DISCRIMINATOR);
        shares_to_burn
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new(user_shares_pda, false),
            AccountMeta::new(*user_asset_account, false),
            AccountMeta::new(*vault_asset_account, false),
            AccountMeta::new_readonly(*asset_mint, false),
            AccountMeta::new_readonly(spl_token, false),
        ];

        let ix = Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        Ok((ix, user_shares_pda))
    }

    /// 5. Emergency Exit (Pause / Unpause)
    pub fn build_emergency_exit_ix(
        &self,
        authority: &Pubkey,
        vault_pda: &Pubkey,
        is_paused: bool,
    ) -> Result<Instruction, SolanaError> {
        let mut data = Vec::with_capacity(9);
        data.extend_from_slice(&EMERGENCY_EXIT_DISCRIMINATOR);
        is_paused
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(*vault_pda, false),
        ];

        Ok(Instruction {
            program_id: self.program_id,
            accounts,
            data,
        })
    }

    /// 6. Set Asset Shariah Compliance (on-chain gate configuration)
    #[allow(clippy::too_many_arguments)]
    pub fn build_set_asset_compliance_ix(
        &self,
        authority: &Pubkey,
        vault_pda: &Pubkey,
        asset_mint: &Pubkey,
        status: u8,
        policy_version: [u8; 32],
        evidence_hash: [u8; 32],
        valid_until: i64,
    ) -> Result<(Instruction, Pubkey), SolanaError> {
        let (compliance_pda, _) = find_compliance_pda(asset_mint, &self.program_id);

        let mut data = Vec::with_capacity(8 + 1 + 32 + 32 + 8);
        data.extend_from_slice(&SET_ASSET_COMPLIANCE_DISCRIMINATOR);
        status
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        policy_version
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        evidence_hash
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        valid_until
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new_readonly(*vault_pda, false),
            AccountMeta::new_readonly(*asset_mint, false),
            AccountMeta::new(compliance_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let ix = Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        Ok((ix, compliance_pda))
    }

    /// 7. Execute Action (rebalance, swap, keeper execution guarded by on-chain Shariah compliance)
    #[allow(clippy::too_many_arguments)]
    pub fn build_execute_action_ix(
        &self,
        keeper: &Pubkey,
        vault_pda: &Pubkey,
        execution_id: u64,
        action_type: u8,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        compliance_pda: &Pubkey,
        input_amount: u64,
        min_output_amount: u64,
    ) -> Result<(Instruction, Pubkey), SolanaError> {
        let (execution_pda, _) = find_execution_pda(vault_pda, execution_id, &self.program_id);

        let mut data = Vec::with_capacity(41);
        data.extend_from_slice(&EXECUTE_ACTION_DISCRIMINATOR);
        execution_id
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        action_type
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        input_amount
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;
        min_output_amount
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new(*keeper, true),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new(execution_pda, false),
            AccountMeta::new_readonly(*input_mint, false),
            AccountMeta::new_readonly(*output_mint, false),
            AccountMeta::new_readonly(*compliance_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let ix = Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        Ok((ix, execution_pda))
    }

    /// 7. Borrow
    pub fn build_borrow_ix(
        &self,
        borrower: &Pubkey,
        vault_pda: &Pubkey,
        amount: u64,
    ) -> Result<(Instruction, Pubkey), SolanaError> {
        let (loan_pda, _) = find_loan_pda(vault_pda, borrower, &self.program_id);
        let (policy_pda, _) = find_policy_pda(vault_pda, &self.program_id);
        let disc = compute_instruction_discriminator("borrow");

        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(&disc);
        amount
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new(*borrower, true),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new(loan_pda, false),
            AccountMeta::new_readonly(policy_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let ix = Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        Ok((ix, loan_pda))
    }

    /// 8. Repay
    pub fn build_repay_ix(
        &self,
        borrower: &Pubkey,
        vault_pda: &Pubkey,
        amount: u64,
    ) -> Result<(Instruction, Pubkey), SolanaError> {
        let (loan_pda, _) = find_loan_pda(vault_pda, borrower, &self.program_id);
        let disc = compute_instruction_discriminator("repay");

        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(&disc);
        amount
            .serialize(&mut data)
            .map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new(*borrower, true),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new(loan_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let ix = Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        Ok((ix, loan_pda))
    }

    // ==========================================
    // TRANSACTION DISPATCH & CONFIRMATION
    // (Guarded by allow_transactions flag)
    // ==========================================

    #[instrument(skip(self, signers))]
    pub async fn send_and_confirm_transaction(
        &self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Signature, SolanaError> {
        if !self.allow_transactions {
            warn!("Rejected transaction dispatch: client is currently in READ-ONLY mode");
            return Err(SolanaError::TransactionsDisabled);
        }

        let (recent_blockhash, _) = self.rpc.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(payer),
            signers,
            recent_blockhash,
        );

        let tx_bytes =
            bincode::serialize(&tx).map_err(|e| SolanaError::SerializationFailed(e.to_string()))?;

        let sig = self.rpc.send_transaction(&tx_bytes).await?;
        info!(sig = %sig, "Transaction sent to Solana cluster, awaiting confirmation");

        self.rpc
            .confirm_signature(&sig, Duration::from_secs(45))
            .await?;
        Ok(sig)
    }
}
