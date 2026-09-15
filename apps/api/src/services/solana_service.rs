//! Service layer coordinating Solana RPC, WebSocket subscriptions, and Anchor client interactions.
//! Enforces read-only safety by default; transaction submission is only unlocked after reads are operational.

use deadpool_postgres::Pool;
use equity_catalyst_solana::{
    accounts::*,
    rpc::{SignatureStatus, SolanaRpcClient},
    websocket::{LogsNotification, SolanaWebSocketClient},
    AnchorClient, SolanaError,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, instrument, warn};

use crate::{
    engines::decision_engine::signer::ExecutionSigner,
    error::ApiError,
    repositories::vault_repository::VaultRepository,
};

#[derive(Clone)]
pub struct SolanaService {
    rpc: Arc<SolanaRpcClient>,
    ws: Arc<SolanaWebSocketClient>,
    anchor_client: Arc<AnchorClient>,
    signer: Option<Arc<ExecutionSigner>>,
    program_id: Pubkey,
    allow_transactions: Arc<RwLock<bool>>,
}

impl SolanaService {
    pub fn new(
        rpc_url: &str,
        ws_url: &str,
        program_id_opt: Option<Pubkey>,
        signer: Option<Arc<ExecutionSigner>>,
    ) -> Self {
        let rpc = Arc::new(SolanaRpcClient::new(rpc_url));
        let ws = Arc::new(SolanaWebSocketClient::new(ws_url));
        let p_id = program_id_opt.unwrap_or_else(program_id);
        let anchor = AnchorClient::new(Arc::clone(&rpc)).with_program_id(p_id);
        // Ensure read-only by default
        assert!(!anchor.is_transaction_submission_enabled());

        Self {
            rpc,
            ws,
            anchor_client: Arc::new(anchor),
            signer,
            program_id: p_id,
            allow_transactions: Arc::new(RwLock::new(false)),
        }
    }

    pub fn program_id(&self) -> &Pubkey {
        &self.program_id
    }

    pub fn rpc(&self) -> &SolanaRpcClient {
        &self.rpc
    }

    pub fn ws(&self) -> &SolanaWebSocketClient {
        &self.ws
    }

    pub fn anchor_client(&self) -> &AnchorClient {
        &self.anchor_client
    }

    pub fn signer(&self) -> Option<&Arc<ExecutionSigner>> {
        self.signer.as_ref()
    }

    pub async fn is_read_only(&self) -> bool {
        !*self.allow_transactions.read().await
    }

    /// Enables write operations (transaction submission) after read-only verification.
    pub async fn enable_transaction_submission(&self) {
        let mut write_guard = self.allow_transactions.write().await;
        *write_guard = true;
        info!("Solana transaction submission has been ENABLED");
    }

    // ==========================================
    // READ-ONLY INTEGRATION METHODS
    // ==========================================

    #[instrument(skip(self))]
    pub async fn read_vault_state(&self, vault_address: &str) -> Result<Option<VaultAccount>, ApiError> {
        let pubkey = Pubkey::from_str(vault_address)
            .map_err(|e| ApiError::BadRequest(format!("Invalid vault address: {}", e)))?;

        match self.anchor_client.fetch_vault(&pubkey).await {
            Ok(vault) => Ok(Some(vault)),
            Err(SolanaError::AccountNotFound(_)) => Ok(None),
            Err(e) => Err(ApiError::InternalServerError(format!(
                "Failed to fetch on-chain vault: {}",
                e
            ))),
        }
    }

    #[instrument(skip(self))]
    pub async fn read_policy_state(&self, vault_address: &str) -> Result<Option<PolicyAccount>, ApiError> {
        let vault_pda = Pubkey::from_str(vault_address)
            .map_err(|e| ApiError::BadRequest(format!("Invalid vault address: {}", e)))?;
        let (policy_pda, _) = find_policy_pda(&vault_pda, &self.program_id);

        match self.anchor_client.fetch_policy(&policy_pda).await {
            Ok(policy) => Ok(Some(policy)),
            Err(SolanaError::AccountNotFound(_)) => Ok(None),
            Err(e) => Err(ApiError::InternalServerError(format!(
                "Failed to fetch on-chain policy: {}",
                e
            ))),
        }
    }

    #[instrument(skip(self))]
    pub async fn read_user_shares(
        &self,
        vault_address: &str,
        user_address: &str,
    ) -> Result<Option<UserSharesAccount>, ApiError> {
        let vault_pda = Pubkey::from_str(vault_address)
            .map_err(|e| ApiError::BadRequest(format!("Invalid vault address: {}", e)))?;
        let user_pubkey = Pubkey::from_str(user_address)
            .map_err(|e| ApiError::BadRequest(format!("Invalid user address: {}", e)))?;
        let (shares_pda, _) = find_user_shares_pda(&vault_pda, &user_pubkey, &self.program_id);

        match self.anchor_client.fetch_user_shares(&shares_pda).await {
            Ok(shares) => Ok(Some(shares)),
            Err(SolanaError::AccountNotFound(_)) => Ok(None),
            Err(e) => Err(ApiError::InternalServerError(format!(
                "Failed to fetch on-chain user shares: {}",
                e
            ))),
        }
    }

    #[instrument(skip(self))]
    pub async fn read_token_balance(&self, token_account: &str) -> Result<u64, ApiError> {
        let pubkey = Pubkey::from_str(token_account)
            .map_err(|e| ApiError::BadRequest(format!("Invalid token account: {}", e)))?;

        let res = self
            .rpc
            .get_token_account_balance(&pubkey)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to query token balance: {}", e)))?;
        Ok(res.amount)
    }

    #[instrument(skip(self))]
    pub async fn read_sol_balance(&self, account: &str) -> Result<u64, ApiError> {
        let pubkey = Pubkey::from_str(account)
            .map_err(|e| ApiError::BadRequest(format!("Invalid account: {}", e)))?;

        let balance = self
            .rpc
            .get_balance(&pubkey)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to query SOL balance: {}", e)))?;
        Ok(balance)
    }

    /// Synchronizes on-chain vault state observations directly to the PostgreSQL database.
    #[instrument(skip(self, pool))]
    pub async fn sync_vault_state_to_db(
        &self,
        vault_address: &str,
        pool: &Pool,
    ) -> Result<Option<VaultAccount>, ApiError> {
        let onchain_opt = self.read_vault_state(vault_address).await?;

        if let Some(ref onchain) = onchain_opt {
            let repo = VaultRepository::new(pool.clone());
            if let Some(_existing) = repo.find_by_address(vault_address).await? {
                repo.update_totals(vault_address, onchain.total_shares, onchain.total_deposits)
                    .await?;
                repo.set_paused(vault_address, onchain.is_paused).await?;
                info!(vault_address, "Successfully synchronized on-chain state with database");
            } else {
                warn!(vault_address, "Vault not found in database for sync update");
            }
        }

        Ok(onchain_opt)
    }

    /// Subscribes to real-time Anchor program logs and event notifications via WebSocket.
    pub async fn subscribe_events(&self) -> Result<mpsc::Receiver<LogsNotification>, ApiError> {
        self.ws
            .logs_subscribe(&self.program_id)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("WebSocket subscription failed: {}", e)))
    }

    // ==========================================
    // TRANSACTION DISPATCH & ANCHOR ACTIONS
    // (Protected by allow_transactions flag)
    // ==========================================

    #[instrument(skip(self, signers))]
    pub async fn submit_transaction(
        &self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Signature, ApiError> {
        if self.is_read_only().await {
            warn!("Rejected transaction dispatch: SolanaService is currently in READ-ONLY mode");
            return Err(ApiError::BadRequest(
                "Transaction submission disabled: SolanaService is currently operating in read-only mode"
                    .to_string(),
            ));
        }

        let (recent_blockhash, _) = self
            .rpc
            .get_latest_blockhash()
            .await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
            instructions,
            Some(payer),
            signers,
            recent_blockhash,
        );

        let tx_bytes = bincode::serialize(&tx)
            .map_err(|e| ApiError::InternalServerError(format!("Serialization error: {}", e)))?;

        let sig = self
            .rpc
            .send_transaction(&tx_bytes)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("RPC submission failed: {}", e)))?;

        info!(sig = %sig, "Transaction sent to Solana cluster");
        Ok(sig)
    }

    /// Simulates a transaction against the cluster prior to broadcast to verify validity.
    #[instrument(skip(self, instructions, signers))]
    pub async fn simulate_transaction(
        &self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<serde_json::Value, ApiError> {
        let (recent_blockhash, _) = self
            .rpc
            .get_latest_blockhash()
            .await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
            instructions,
            Some(payer),
            signers,
            recent_blockhash,
        );

        let tx_bytes = bincode::serialize(&tx)
            .map_err(|e| ApiError::InternalServerError(format!("Serialization error: {}", e)))?;

        self.rpc
            .simulate_transaction(&tx_bytes)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Simulation failed: {}", e)))
    }

    #[instrument(skip(self))]
    pub async fn confirm_signature(
        &self,
        sig: &Signature,
        timeout: Duration,
    ) -> Result<SignatureStatus, ApiError> {
        self.rpc
            .confirm_signature(sig, timeout)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Confirmation failed: {}", e)))
    }

    // High-Level Anchor Instruction Dispatchers

    pub async fn initialize_vault(
        &self,
        authority: &Keypair,
        asset_mint: &Pubkey,
        name: &str,
        symbol: &str,
        max_ltv_bps: u16,
        max_position_bps: u16,
    ) -> Result<(Signature, Pubkey, Pubkey), ApiError> {
        let (ix, vault_pda, policy_pda) = self
            .anchor_client
            .build_initialize_vault_ix(
                &authority.pubkey(),
                asset_mint,
                name,
                symbol,
                max_ltv_bps,
                max_position_bps,
            )
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        let sig = self
            .submit_transaction(&[ix], &authority.pubkey(), &[authority])
            .await?;
        Ok((sig, vault_pda, policy_pda))
    }

    pub async fn update_policy(
        &self,
        authority: &Keypair,
        vault_pda: &Pubkey,
        max_ltv_bps: u16,
        max_position_bps: u16,
        stop_loss_bps: u16,
        take_profit_bps: u16,
        rebalance_threshold_bps: u16,
        is_active: bool,
    ) -> Result<(Signature, Pubkey), ApiError> {
        let (ix, policy_pda) = self
            .anchor_client
            .build_update_policy_ix(
                &authority.pubkey(),
                vault_pda,
                max_ltv_bps,
                max_position_bps,
                stop_loss_bps,
                take_profit_bps,
                rebalance_threshold_bps,
                is_active,
            )
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        let sig = self
            .submit_transaction(&[ix], &authority.pubkey(), &[authority])
            .await?;
        Ok((sig, policy_pda))
    }

    pub async fn deposit(
        &self,
        user: &Keypair,
        vault_pda: &Pubkey,
        user_asset_account: &Pubkey,
        vault_asset_account: &Pubkey,
        asset_mint: &Pubkey,
        amount: u64,
    ) -> Result<(Signature, Pubkey), ApiError> {
        let (ix, user_shares_pda) = self
            .anchor_client
            .build_deposit_ix(
                &user.pubkey(),
                vault_pda,
                user_asset_account,
                vault_asset_account,
                asset_mint,
                amount,
            )
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        let sig = self
            .submit_transaction(&[ix], &user.pubkey(), &[user])
            .await?;
        Ok((sig, user_shares_pda))
    }

    pub async fn withdraw(
        &self,
        user: &Keypair,
        vault_pda: &Pubkey,
        user_asset_account: &Pubkey,
        vault_asset_account: &Pubkey,
        asset_mint: &Pubkey,
        shares_to_burn: u64,
    ) -> Result<(Signature, Pubkey), ApiError> {
        let (ix, user_shares_pda) = self
            .anchor_client
            .build_withdraw_ix(
                &user.pubkey(),
                vault_pda,
                user_asset_account,
                vault_asset_account,
                asset_mint,
                shares_to_burn,
            )
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        let sig = self
            .submit_transaction(&[ix], &user.pubkey(), &[user])
            .await?;
        Ok((sig, user_shares_pda))
    }

    pub async fn emergency_exit(
        &self,
        authority: &Keypair,
        vault_pda: &Pubkey,
        is_paused: bool,
    ) -> Result<Signature, ApiError> {
        let ix = self
            .anchor_client
            .build_emergency_exit_ix(&authority.pubkey(), vault_pda, is_paused)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        self.submit_transaction(&[ix], &authority.pubkey(), &[authority])
            .await
    }

    pub async fn execute_action(
        &self,
        keeper: &Keypair,
        vault_pda: &Pubkey,
        execution_id: u64,
        action_type: u8,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        input_amount: u64,
        min_output_amount: u64,
    ) -> Result<(Signature, Pubkey), ApiError> {
        let (ix, execution_pda) = self
            .anchor_client
            .build_execute_action_ix(
                &keeper.pubkey(),
                vault_pda,
                execution_id,
                action_type,
                input_mint,
                output_mint,
                input_amount,
                min_output_amount,
            )
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        let sig = self
            .submit_transaction(&[ix], &keeper.pubkey(), &[keeper])
            .await?;
        Ok((sig, execution_pda))
    }

    pub async fn borrow(
        &self,
        borrower: &Keypair,
        vault_pda: &Pubkey,
        amount: u64,
    ) -> Result<(Signature, Pubkey), ApiError> {
        let (ix, loan_pda) = self
            .anchor_client
            .build_borrow_ix(&borrower.pubkey(), vault_pda, amount)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        let sig = self
            .submit_transaction(&[ix], &borrower.pubkey(), &[borrower])
            .await?;
        Ok((sig, loan_pda))
    }

    pub async fn repay(
        &self,
        borrower: &Keypair,
        vault_pda: &Pubkey,
        amount: u64,
    ) -> Result<(Signature, Pubkey), ApiError> {
        let (ix, loan_pda) = self
            .anchor_client
            .build_repay_ix(&borrower.pubkey(), vault_pda, amount)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        let sig = self
            .submit_transaction(&[ix], &borrower.pubkey(), &[borrower])
            .await?;
        Ok((sig, loan_pda))
    }
}
