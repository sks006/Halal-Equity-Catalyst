import * as fs from "fs";
import * as path from "path";
import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import {
  createMint,
  createAccount,
  mintTo,
  getAccount,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import * as child_process from "child_process";

// ANSI colors
const RESET = "\x1b[0m";
const BOLD = "\x1b[1m";
const GREEN = "\x1b[32m";
const CYAN = "\x1b[36m";
const YELLOW = "\x1b[33m";
const MAGENTA = "\x1b[35m";

async function main() {
  console.log(`\n${BOLD}${CYAN}=======================================================${RESET}`);
  console.log(`${BOLD}${CYAN}   EQUITY CATALYST — Devnet State Initialization       ${RESET}`);
  console.log(`${BOLD}${CYAN}=======================================================${RESET}\n`);

  // 1. Connection & Keypair
  const rpcUrl = process.env.SOLANA_RPC_URL || "https://api.devnet.solana.com";
  const connection = new Connection(rpcUrl, "confirmed");
  console.log(`${GREEN}✓${RESET} Connected to Solana Devnet RPC: ${rpcUrl}`);

  const keypairPath = path.resolve(
    process.env.HOME || "",
    ".config/solana/id.json"
  );
  if (!fs.existsSync(keypairPath)) {
    throw new Error(`Signer keypair not found at ${keypairPath}`);
  }
  const secretKey = Uint8Array.from(JSON.parse(fs.readFileSync(keypairPath, "utf8")));
  const authority = Keypair.fromSecretKey(secretKey);
  console.log(`${GREEN}✓${RESET} Loaded Authority Signer: ${BOLD}${authority.publicKey.toBase58()}${RESET}`);

  const balance = await connection.getBalance(authority.publicKey);
  console.log(`  SOL Balance: ${(balance / 1e9).toFixed(4)} SOL`);

  // 2. Load Anchor Program & IDL
  const programId = new PublicKey("8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH");
  const idlPath = path.resolve(__dirname, "../target/idl/equity_vault.json");
  const idl = JSON.parse(fs.readFileSync(idlPath, "utf8"));

  const wallet = new anchor.Wallet(authority);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });
  const program = new anchor.Program(idl as anchor.Idl, provider);
  console.log(`${GREEN}✓${RESET} Program ID: ${BOLD}${programId.toBase58()}${RESET}`);

  // 3. Create or derive underlying deposit mint (USDC mock)
  console.log(`\n${CYAN}Creating / deriving underlying test mints on Devnet...${RESET}`);
  const assetMint = await createMint(
    connection,
    authority,
    authority.publicKey,
    null,
    6 // 6 decimals
  );
  console.log(`${GREEN}✓${RESET} Created Test Deposit Mint (USDC): ${BOLD}${assetMint.toBase58()}${RESET}`);

  // Mock tokenized equity mint (NVDA)
  const nvdaMint = await createMint(
    connection,
    authority,
    authority.publicKey,
    null,
    6
  );
  console.log(`${GREEN}✓${RESET} Created Test Equity Mint (xStock NVDA): ${BOLD}${nvdaMint.toBase58()}${RESET}`);

  // 4. Derive Vault and Policy PDAs
  const vaultName = "Catalyst Alpha";
  const vaultSymbol = "CAT-A";

  const [vaultPda, vaultBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), authority.publicKey.toBuffer(), Buffer.from(vaultName)],
    programId
  );
  console.log(`${GREEN}✓${RESET} Derived Vault PDA: ${BOLD}${vaultPda.toBase58()}${RESET} (bump: ${vaultBump})`);

  const [policyPda, policyBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("policy"), vaultPda.toBuffer()],
    programId
  );
  console.log(`${GREEN}✓${RESET} Derived Policy PDA: ${BOLD}${policyPda.toBase58()}${RESET} (bump: ${policyBump})`);

  // 5. Initialize Vault & Policy On-Chain
  console.log(`\n${CYAN}Initializing Vault & Policy on Solana Devnet...${RESET}`);
  const initialMaxLtvBps = 6500;        // 65.00%
  const initialMaxPositionBps = 2500;   // 25.00%

  try {
    const initTx = await (program.methods as any)
      .initializeVault(vaultName, vaultSymbol, initialMaxLtvBps, initialMaxPositionBps)
      .accounts({
        authority: authority.publicKey,
        vault: vaultPda,
        policy: policyPda,
        assetMint: assetMint,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log(`${GREEN}✓${RESET} initializeVault Tx: ${initTx}`);
  } catch (err: any) {
    if (err.toString().includes("already in use")) {
      console.log(`${YELLOW}ℹ${RESET} Vault PDA already initialized on-chain`);
    } else {
      throw err;
    }
  }

  // 6. Update Policy Risk Guardrails on-chain
  console.log(`\n${CYAN}Updating Policy Guardrails on Solana Devnet...${RESET}`);
  const stopLossBps = 800;         // 8.00%
  const takeProfitBps = 2000;      // 20.00%
  const rebalanceThresholdBps = 150; // 1.50%

  const updatePolicyTx = await (program.methods as any)
    .updatePolicy(
      initialMaxLtvBps,
      initialMaxPositionBps,
      stopLossBps,
      takeProfitBps,
      rebalanceThresholdBps,
      true // is_active
    )
    .accounts({
      authority: authority.publicKey,
      vault: vaultPda,
      policy: policyPda,
    })
    .rpc();
  console.log(`${GREEN}✓${RESET} updatePolicy Tx: ${updatePolicyTx}`);

  // 7. Setup Token Accounts and Deposit Initial Collateral
  console.log(`\n${CYAN}Setting up Vault ATA and executing test deposit...${RESET}`);
  const vaultAssetAccount = getAssociatedTokenAddressSync(assetMint, vaultPda, true);
  const vaultAtaInfo = await connection.getAccountInfo(vaultAssetAccount);
  if (!vaultAtaInfo) {
    const createVaultAtaIx = createAssociatedTokenAccountInstruction(
      authority.publicKey,
      vaultAssetAccount,
      vaultPda,
      assetMint
    );
    const tx = new Transaction().add(createVaultAtaIx);
    await provider.sendAndConfirm(tx);
    console.log(`${GREEN}✓${RESET} Created Vault ATA: ${vaultAssetAccount.toBase58()}`);
  }

  // Create User Token Account & Mint test USDC
  const userAssetAccount = await createAccount(
    connection,
    authority,
    assetMint,
    authority.publicKey
  );
  const mintAmount = 10_000_000; // 10 USDC
  await mintTo(
    connection,
    authority,
    assetMint,
    userAssetAccount,
    authority,
    mintAmount
  );
  console.log(`${GREEN}✓${RESET} Minted ${mintAmount / 1e6} USDC to user token account: ${userAssetAccount.toBase58()}`);

  // Derive User Shares PDA
  const [userSharesPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("user_shares"), vaultPda.toBuffer(), authority.publicKey.toBuffer()],
    programId
  );

  // Deposit 2.5 USDC into vault
  const depositAmount = new anchor.BN(2_500_000);
  const depositTx = await (program.methods as any)
    .deposit(depositAmount)
    .accounts({
      user: authority.publicKey,
      vault: vaultPda,
      userShares: userSharesPda,
      userAssetAccount: userAssetAccount,
      vaultAssetAccount: vaultAssetAccount,
      assetMint: assetMint,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log(`${GREEN}✓${RESET} Executed on-chain deposit (2.5 USDC) Tx: ${depositTx}`);

  // 8. Verify on-chain state
  console.log(`\n${CYAN}Verifying On-Chain Devnet State...${RESET}`);
  const vaultAcc = await (program.account as any).vault.fetch(vaultPda);
  const policyAcc = await (program.account as any).policy.fetch(policyPda);
  const vaultTokenAcc = await getAccount(connection, vaultAssetAccount);

  console.log(`  Vault Name:             ${vaultAcc.name}`);
  console.log(`  Vault Symbol:           ${vaultAcc.symbol}`);
  console.log(`  Total Shares:           ${vaultAcc.totalShares.toString()} micro-units`);
  console.log(`  Total Deposits:         ${vaultAcc.totalDeposits.toString()} micro-units`);
  console.log(`  On-chain Token Balance: ${vaultTokenAcc.amount.toString()} micro-units`);
  console.log(`  Max LTV:                ${policyAcc.maxLtvBps / 100}%`);
  console.log(`  Max Position:           ${policyAcc.maxPositionBps / 100}%`);
  console.log(`  Stop Loss:              ${policyAcc.stopLossBps / 100}%`);
  console.log(`  Policy Active:          ${policyAcc.isActive}`);

  // 9. Sync state to local PostgreSQL database
  console.log(`\n${CYAN}Syncing Devnet Vault & Policy to Local PostgreSQL...${RESET}`);
  try {
    const runSql = (sql: string) => {
      child_process.execSync(
        `PGPASSWORD=postgres psql -h localhost -U postgres -d equity_catalyst -c "${sql.replace(/"/g, '\\"')}"`,
        { stdio: "pipe" }
      );
    };

    // Upsert Vault record
    runSql(`
      INSERT INTO vaults (vault_address, authority, name, symbol, deposit_mint, vault_token_account, total_shares, total_deposits, is_paused, bump, created_at, updated_at)
      VALUES ('${vaultPda.toBase58()}', '${authority.publicKey.toBase58()}', '${vaultName}', '${vaultSymbol}', '${assetMint.toBase58()}', '${vaultAssetAccount.toBase58()}', ${vaultAcc.totalShares.toNumber()}, ${vaultAcc.totalDeposits.toNumber()}, ${vaultAcc.isPaused}, ${vaultBump}, NOW(), NOW())
      ON CONFLICT (vault_address) DO UPDATE SET
        total_shares = EXCLUDED.total_shares,
        total_deposits = EXCLUDED.total_deposits,
        updated_at = NOW();
    `);
    console.log(`${GREEN}✓${RESET} Persisted Vault to PostgreSQL`);

    // Upsert Policy record
    runSql(`
      INSERT INTO policies (policy_address, vault_address, authority, max_ltv_bps, max_position_bps, stop_loss_bps, take_profit_bps, rebalance_threshold_bps, is_active, bump, created_at, updated_at)
      VALUES ('${policyPda.toBase58()}', '${vaultPda.toBase58()}', '${authority.publicKey.toBase58()}', ${policyAcc.maxLtvBps}, ${policyAcc.maxPositionBps}, ${policyAcc.stopLossBps}, ${policyAcc.takeProfitBps}, ${policyAcc.rebalanceThresholdBps}, ${policyAcc.isActive}, ${policyBump}, NOW(), NOW())
      ON CONFLICT (policy_address) DO UPDATE SET
        max_ltv_bps = EXCLUDED.max_ltv_bps,
        max_position_bps = EXCLUDED.max_position_bps,
        updated_at = NOW();
    `);
    console.log(`${GREEN}✓${RESET} Persisted Policy to PostgreSQL`);

    // Seed initial portfolio position
    runSql(`
      INSERT INTO portfolios (portfolio_id, vault_address, asset_symbol, asset_mint, amount, entry_price_usd, current_price_usd, current_value_usd, target_weight_bps, current_weight_bps, updated_at)
      VALUES (gen_random_uuid(), '${vaultPda.toBase58()}', 'USDC', '${assetMint.toBase58()}', ${vaultAcc.totalDeposits.toNumber()}, 1.0, 1.0, ${vaultAcc.totalDeposits.toNumber() / 1e6}, 8000, 8000, NOW()),
             (gen_random_uuid(), '${vaultPda.toBase58()}', 'NVDA', '${nvdaMint.toBase58()}', 50, 125.0, 128.4, 6420.0, 2000, 2000, NOW())
      ON CONFLICT DO NOTHING;
    `);
    console.log(`${GREEN}✓${RESET} Seeded initial Devnet Portfolio holdings in PostgreSQL`);
  } catch (dbErr: any) {
    console.log(`${YELLOW}Warning: PostgreSQL sync error: ${dbErr.message}${RESET}`);
  }

  console.log(`\n${BOLD}${GREEN}=======================================================${RESET}`);
  console.log(`${BOLD}${GREEN}   DEVNET STATE INITIALIZATION COMPLETE!               ${RESET}`);
  console.log(`${BOLD}${GREEN}   Vault Address:  ${vaultPda.toBase58()}               ${RESET}`);
  console.log(`${BOLD}${GREEN}   Policy Address: ${policyPda.toBase58()}              ${RESET}`);
  console.log(`${BOLD}${GREEN}=======================================================${RESET}\n`);
}

main().catch((err) => {
  console.error("\nError during Devnet initialization:", err);
  process.exit(1);
});
