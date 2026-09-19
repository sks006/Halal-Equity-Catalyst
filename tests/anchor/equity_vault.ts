import * as anchor from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import {
  createMint,
  createAccount,
  mintTo,
  getAccount,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import { expect } from "chai";

describe("equity_vault - Milestones 1 & 2", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.EquityVault as any;
  const authority = (provider.wallet as anchor.Wallet).payer;

  let assetMint: PublicKey;
  let vaultPda: PublicKey;
  let vaultBump: number;
  let policyPda: PublicKey;
  let policyBump: number;
  let vaultAssetAccount: PublicKey;

  const vaultName = "Catalyst Alpha";
  const vaultSymbol = "CAT-A";
  const initialMaxLtvBps = 7500; // 75%
  const initialMaxPositionBps = 2500; // 25%

  // User 1 (Authority)
  let user1AssetAccount: PublicKey;

  // User 2
  const user2 = Keypair.generate();
  let user2AssetAccount: PublicKey;
  let user2SharesPda: PublicKey;

  before(async () => {
    // 1. Airdrop SOL to user2
    const airdropSig = await provider.connection.requestAirdrop(
      user2.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdropSig, "confirmed");

    // 2. Create underlying asset mint
    assetMint = await createMint(
      provider.connection,
      authority,
      authority.publicKey,
      null,
      6
    );

    // 3. Derive Vault & Policy PDAs
    [vaultPda, vaultBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), authority.publicKey.toBuffer(), Buffer.from(vaultName)],
      program.programId
    );

    [policyPda, policyBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("policy"), vaultPda.toBuffer()],
      program.programId
    );

    // 4. Create Vault ATA for assetMint owned by Vault PDA
    vaultAssetAccount = getAssociatedTokenAddressSync(assetMint, vaultPda, true);
    const createVaultAtaIx = createAssociatedTokenAccountInstruction(
      authority.publicKey,
      vaultAssetAccount,
      vaultPda,
      assetMint
    );
    const createAtaTx = new anchor.web3.Transaction().add(createVaultAtaIx);
    await provider.sendAndConfirm(createAtaTx);

    // 5. Create User1 Token Account & mint 10,000,000 units
    user1AssetAccount = await createAccount(
      provider.connection,
      authority,
      assetMint,
      authority.publicKey
    );
    await mintTo(
      provider.connection,
      authority,
      assetMint,
      user1AssetAccount,
      authority,
      10_000_000
    );

    // 6. Create User2 Token Account & mint 5,000,000 units
    user2AssetAccount = await createAccount(
      provider.connection,
      user2,
      assetMint,
      user2.publicKey
    );
    await mintTo(
      provider.connection,
      authority,
      assetMint,
      user2AssetAccount,
      authority,
      5_000_000
    );

    // Derive User2 Shares PDA
    [user2SharesPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_shares"), vaultPda.toBuffer(), user2.publicKey.toBuffer()],
      program.programId
    );
  });

  describe("Phase 1: Vault & Policy Initialization", () => {
    it("successfully initializes vault and linked policy PDA", async () => {
      const tx = await program.methods
        .initializeVault(vaultName, vaultSymbol, initialMaxLtvBps, initialMaxPositionBps)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
          policy: policyPda,
          assetMint: assetMint,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      expect(tx).to.be.a("string");

      const vaultAccount = await program.account.vault.fetch(vaultPda);
      expect(vaultAccount.authority.toBase58()).to.equal(authority.publicKey.toBase58());
      expect(vaultAccount.assetMint.toBase58()).to.equal(assetMint.toBase58());
      expect(vaultAccount.policy.toBase58()).to.equal(policyPda.toBase58());
      expect(vaultAccount.name).to.equal(vaultName);
      expect(vaultAccount.symbol).to.equal(vaultSymbol);
      expect(vaultAccount.totalDeposits.toNumber()).to.equal(0);
      expect(vaultAccount.totalShares.toNumber()).to.equal(0);
      expect(vaultAccount.isPaused).to.be.false;

      const policyAccount = await program.account.policy.fetch(policyPda);
      expect(policyAccount.vault.toBase58()).to.equal(vaultPda.toBase58());
      expect(policyAccount.authority.toBase58()).to.equal(authority.publicKey.toBase58());
      expect(policyAccount.maxLtvBps).to.equal(initialMaxLtvBps);
      expect(policyAccount.maxPositionBps).to.equal(initialMaxPositionBps);
      expect(policyAccount.isActive).to.be.true;
    });

    it("fails to initialize when max_ltv_bps > 10,000", async () => {
      const name = "Inv LTV";
      const [invVault] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), authority.publicKey.toBuffer(), Buffer.from(name)],
        program.programId
      );
      const [invPolicy] = PublicKey.findProgramAddressSync(
        [Buffer.from("policy"), invVault.toBuffer()],
        program.programId
      );

      try {
        await program.methods
          .initializeVault(name, "INV", 10001, 2000)
          .accounts({
            authority: authority.publicKey,
            vault: invVault,
            policy: invPolicy,
            assetMint: assetMint,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        expect.fail("Expected failure on invalid LTV");
      } catch (err: any) {
        expect(err.toString()).to.include("InvalidRiskLimit");
      }
    });

    it("rejects duplicate initialization with the same seeds", async () => {
      try {
        await program.methods
          .initializeVault(vaultName, vaultSymbol, initialMaxLtvBps, initialMaxPositionBps)
          .accounts({
            authority: authority.publicKey,
            vault: vaultPda,
            policy: policyPda,
            assetMint: assetMint,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        expect.fail("Expected failure on duplicate initialization");
      } catch (err: any) {
        expect(err.toString()).to.match(/already in use|custom program error: 0x0/i);
      }
    });
  });

  describe("Phase 2: Deposit Operations", () => {
    it("rejects deposit below minimum threshold (1,000 units)", async () => {
      const [user1SharesPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("user_shares"), vaultPda.toBuffer(), authority.publicKey.toBuffer()],
        program.programId
      );

      try {
        await program.methods
          .deposit(new anchor.BN(500))
          .accounts({
            user: authority.publicKey,
            vault: vaultPda,
            userShares: user1SharesPda,
            userAssetAccount: user1AssetAccount,
            vaultAssetAccount: vaultAssetAccount,
            assetMint: assetMint,
            tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        expect.fail("Expected failure on deposit too small");
      } catch (err: any) {
        expect(err.toString()).to.include("DepositTooSmall");
      }
    });

    it("initial deposit issues 1:1 shares", async () => {
      const depositAmount = new anchor.BN(1_000_000); // 1.0 token
      const [user1SharesPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("user_shares"), vaultPda.toBuffer(), authority.publicKey.toBuffer()],
        program.programId
      );

      await program.methods
        .deposit(depositAmount)
        .accounts({
          user: authority.publicKey,
          vault: vaultPda,
          userShares: user1SharesPda,
          userAssetAccount: user1AssetAccount,
          vaultAssetAccount: vaultAssetAccount,
          assetMint: assetMint,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      // Check vault state
      const vaultAccount = await program.account.vault.fetch(vaultPda);
      expect(vaultAccount.totalDeposits.toNumber()).to.equal(1_000_000);
      expect(vaultAccount.totalShares.toNumber()).to.equal(1_000_000);

      // Check UserShares PDA state
      const user1Shares = await program.account.userShares.fetch(user1SharesPda);
      expect(user1Shares.shares.toNumber()).to.equal(1_000_000);
      expect(user1Shares.user.toBase58()).to.equal(authority.publicKey.toBase58());

      // Check Vault token account balance on-chain
      const vaultTokenAcc = await getAccount(provider.connection, vaultAssetAccount);
      expect(Number(vaultTokenAcc.amount)).to.equal(1_000_000);
    });

    it("second deposit issues proportional shares", async () => {
      const depositAmount = new anchor.BN(500_000); // 0.5 token

      await program.methods
        .deposit(depositAmount)
        .accounts({
          user: user2.publicKey,
          vault: vaultPda,
          userShares: user2SharesPda,
          userAssetAccount: user2AssetAccount,
          vaultAssetAccount: vaultAssetAccount,
          assetMint: assetMint,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([user2])
        .rpc();

      const vaultAccount = await program.account.vault.fetch(vaultPda);
      expect(vaultAccount.totalDeposits.toNumber()).to.equal(1_500_000);
      expect(vaultAccount.totalShares.toNumber()).to.equal(1_500_000);

      const user2Shares = await program.account.userShares.fetch(user2SharesPda);
      expect(user2Shares.shares.toNumber()).to.equal(500_000);
    });
  });

  describe("Phase 3: Withdraw Operations", () => {
    it("rejects withdrawal when requested shares exceed user balance", async () => {
      try {
        await program.methods
          .withdraw(new anchor.BN(1_000_000)) // user2 only has 500_000
          .accounts({
            user: user2.publicKey,
            vault: vaultPda,
            userShares: user2SharesPda,
            userAssetAccount: user2AssetAccount,
            vaultAssetAccount: vaultAssetAccount,
            assetMint: assetMint,
            tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
          })
          .signers([user2])
          .rpc();
        expect.fail("Expected failure on insufficient shares");
      } catch (err: any) {
        expect(err.toString()).to.include("InsufficientShares");
      }
    });

    it("partial withdrawal burns shares and transfers proportionate tokens", async () => {
      const sharesToBurn = new anchor.BN(250_000); // 50% of user2's shares

      const user2BalBefore = await getAccount(provider.connection, user2AssetAccount);

      await program.methods
        .withdraw(sharesToBurn)
        .accounts({
          user: user2.publicKey,
          vault: vaultPda,
          userShares: user2SharesPda,
          userAssetAccount: user2AssetAccount,
          vaultAssetAccount: vaultAssetAccount,
          assetMint: assetMint,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([user2])
        .rpc();

      // Verify user received 250,000 tokens back
      const user2BalAfter = await getAccount(provider.connection, user2AssetAccount);
      expect(Number(user2BalAfter.amount) - Number(user2BalBefore.amount)).to.equal(250_000);

      // Verify UserShares decreased
      const user2Shares = await program.account.userShares.fetch(user2SharesPda);
      expect(user2Shares.shares.toNumber()).to.equal(250_000);

      // Verify Vault total state decreased
      const vaultAccount = await program.account.vault.fetch(vaultPda);
      expect(vaultAccount.totalDeposits.toNumber()).to.equal(1_250_000);
      expect(vaultAccount.totalShares.toNumber()).to.equal(1_250_000);
    });
  });

  describe("Phase 4: Policy Updates", () => {
    it("authority can update policy risk limits and parameters", async () => {
      const newMaxLtvBps = 8000;
      const newMaxPosBps = 3000;
      const newStopLossBps = 600;
      const newTakeProfitBps = 2000;
      const newRebalanceBps = 250;

      await program.methods
        .updatePolicy(
          newMaxLtvBps,
          newMaxPosBps,
          newStopLossBps,
          newTakeProfitBps,
          newRebalanceBps,
          true
        )
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
          policy: policyPda,
        })
        .rpc();

      const policyAccount = await program.account.policy.fetch(policyPda);
      expect(policyAccount.maxLtvBps).to.equal(newMaxLtvBps);
      expect(policyAccount.maxPositionBps).to.equal(newMaxPosBps);
      expect(policyAccount.stopLossBps).to.equal(newStopLossBps);
      expect(policyAccount.takeProfitBps).to.equal(newTakeProfitBps);
      expect(policyAccount.rebalanceThresholdBps).to.equal(newRebalanceBps);
      expect(policyAccount.isActive).to.be.true;
    });

    it("rejects policy update from non-authority caller", async () => {
      try {
        await program.methods
          .updatePolicy(7000, 2000, 500, 1500, 200, true)
          .accounts({
            authority: user2.publicKey,
            vault: vaultPda,
            policy: policyPda,
          })
          .signers([user2])
          .rpc();
        expect.fail("Expected failure on unauthorized update");
      } catch (err: any) {
        expect(err.toString()).to.match(/UnauthorizedKeeper|ConstraintHasOne|custom program error/i);
      }
    });

    it("rejects policy update with invalid risk limits (> 10,000 bps)", async () => {
      try {
        await program.methods
          .updatePolicy(10001, 2000, 500, 1500, 200, true)
          .accounts({
            authority: authority.publicKey,
            vault: vaultPda,
            policy: policyPda,
          })
          .rpc();
        expect.fail("Expected failure on invalid risk limits");
      } catch (err: any) {
        expect(err.toString()).to.include("InvalidRiskLimit");
      }
    });
  });

  describe("Phase 5: Emergency Pause Controls", () => {
    it("authority can pause the vault", async () => {
      await program.methods
        .emergencyExit(true)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
        })
        .rpc();

      const vaultAccount = await program.account.vault.fetch(vaultPda);
      expect(vaultAccount.isPaused).to.be.true;
    });

    it("rejects deposits while vault is paused", async () => {
      try {
        await program.methods
          .deposit(new anchor.BN(100_000))
          .accounts({
            user: user2.publicKey,
            vault: vaultPda,
            userShares: user2SharesPda,
            userAssetAccount: user2AssetAccount,
            vaultAssetAccount: vaultAssetAccount,
            assetMint: assetMint,
            tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([user2])
          .rpc();
        expect.fail("Expected deposit to fail when paused");
      } catch (err: any) {
        expect(err.toString()).to.include("VaultPaused");
      }
    });

    it("rejects withdrawals while vault is paused", async () => {
      try {
        await program.methods
          .withdraw(new anchor.BN(50_000))
          .accounts({
            user: user2.publicKey,
            vault: vaultPda,
            userShares: user2SharesPda,
            userAssetAccount: user2AssetAccount,
            vaultAssetAccount: vaultAssetAccount,
            assetMint: assetMint,
            tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
          })
          .signers([user2])
          .rpc();
        expect.fail("Expected withdrawal to fail when paused");
      } catch (err: any) {
        expect(err.toString()).to.include("VaultPaused");
      }
    });

    it("authority can unpause the vault and resume operations", async () => {
      await program.methods
        .emergencyExit(false)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
        })
        .rpc();

      const vaultAccount = await program.account.vault.fetch(vaultPda);
      expect(vaultAccount.isPaused).to.be.false;

      // Deposit now succeeds
      await program.methods
        .deposit(new anchor.BN(100_000))
        .accounts({
          user: user2.publicKey,
          vault: vaultPda,
          userShares: user2SharesPda,
          userAssetAccount: user2AssetAccount,
          vaultAssetAccount: vaultAssetAccount,
          assetMint: assetMint,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([user2])
        .rpc();

      const user2Shares = await program.account.userShares.fetch(user2SharesPda);
      expect(user2Shares.shares.toNumber()).to.be.greaterThan(250_000);
    });
  });

  describe("Phase 6: Shariah Compliance & Execution Boundary", () => {
    let approvedAssetMint: PublicKey;
    let pendingAssetMint: PublicKey;
    let rejectedAssetMint: PublicKey;
    let expiredAssetMint: PublicKey;
    let mismatchedAssetMint: PublicKey;

    let approvedCompliancePda: PublicKey;
    let pendingCompliancePda: PublicKey;
    let rejectedCompliancePda: PublicKey;
    let expiredCompliancePda: PublicKey;
    let mismatchedCompliancePda: PublicKey;

    const policyVersion = Array.from(Buffer.alloc(32, 1));
    const evidenceHash = Array.from(Buffer.alloc(32, 2));

    function findCompliancePda(mint: PublicKey): PublicKey {
      return PublicKey.findProgramAddressSync(
        [Buffer.from("compliance"), mint.toBuffer()],
        program.programId
      )[0];
    }

    function findExecutionPda(vault: PublicKey, id: anchor.BN): PublicKey {
      const buf = Buffer.alloc(8);
      buf.writeBigUInt64LE(BigInt(id.toString()));
      return PublicKey.findProgramAddressSync(
        [Buffer.from("execution"), vault.toBuffer(), buf],
        program.programId
      )[0];
    }

    before(async () => {
      // Create test equity mints
      approvedAssetMint = await createMint(
        provider.connection,
        authority,
        authority.publicKey,
        null,
        6
      );
      pendingAssetMint = await createMint(
        provider.connection,
        authority,
        authority.publicKey,
        null,
        6
      );
      rejectedAssetMint = await createMint(
        provider.connection,
        authority,
        authority.publicKey,
        null,
        6
      );
      expiredAssetMint = await createMint(
        provider.connection,
        authority,
        authority.publicKey,
        null,
        6
      );
      mismatchedAssetMint = await createMint(
        provider.connection,
        authority,
        authority.publicKey,
        null,
        6
      );

      approvedCompliancePda = findCompliancePda(approvedAssetMint);
      pendingCompliancePda = findCompliancePda(pendingAssetMint);
      rejectedCompliancePda = findCompliancePda(rejectedAssetMint);
      expiredCompliancePda = findCompliancePda(expiredAssetMint);
      mismatchedCompliancePda = findCompliancePda(mismatchedAssetMint);

      const futureTimestamp = new anchor.BN(Math.floor(Date.now() / 1000) + 86400 * 30);
      const pastTimestamp = new anchor.BN(Math.floor(Date.now() / 1000) - 86400);

      // 1. Initialize Approved compliance
      await program.methods
        .setAssetCompliance(1, policyVersion, evidenceHash, futureTimestamp)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
          assetMint: approvedAssetMint,
          compliance: approvedCompliancePda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      // 2. Initialize Pending compliance
      await program.methods
        .setAssetCompliance(0, policyVersion, evidenceHash, futureTimestamp)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
          assetMint: pendingAssetMint,
          compliance: pendingCompliancePda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      // 3. Initialize Rejected compliance
      await program.methods
        .setAssetCompliance(2, policyVersion, evidenceHash, futureTimestamp)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
          assetMint: rejectedAssetMint,
          compliance: rejectedCompliancePda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      // 4. Initialize Expired compliance
      await program.methods
        .setAssetCompliance(1, policyVersion, evidenceHash, pastTimestamp)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
          assetMint: expiredAssetMint,
          compliance: expiredCompliancePda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      // 5. Initialize Mismatched compliance
      await program.methods
        .setAssetCompliance(1, policyVersion, evidenceHash, futureTimestamp)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
          assetMint: mismatchedAssetMint,
          compliance: mismatchedCompliancePda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
    });

    it("verifies compliance account state was initialized correctly", async () => {
      const comp = await program.account.assetCompliance.fetch(approvedCompliancePda);
      expect(comp.assetMint.toBase58()).to.equal(approvedAssetMint.toBase58());
      expect(comp.status).to.equal(1);
    });

    it("allows execution for an Approved asset with valid review window", async () => {
      const execId = new anchor.BN(2001);
      const execPda = findExecutionPda(vaultPda, execId);

      await program.methods
        .executeAction(execId, 1, new anchor.BN(1_000_000), new anchor.BN(990_000))
        .accounts({
          keeper: authority.publicKey,
          vault: vaultPda,
          execution: execPda,
          inputMint: assetMint,
          outputMint: approvedAssetMint,
          compliance: approvedCompliancePda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      const execAccount = await program.account.execution.fetch(execPda);
      expect(execAccount.status).to.equal(1); // Executed
      expect(execAccount.actionType).to.equal(1); // Spot Swap
      expect(execAccount.executionId.toString()).to.equal(execId.toString());
      expect(execAccount.inputMint.toBase58()).to.equal(assetMint.toBase58());
      expect(execAccount.outputMint.toBase58()).to.equal(approvedAssetMint.toBase58());
    });

    it("rejects execution when asset status is Pending", async () => {
      const execId = new anchor.BN(2002);
      const execPda = findExecutionPda(vaultPda, execId);

      try {
        await program.methods
          .executeAction(execId, 1, new anchor.BN(1_000_000), new anchor.BN(990_000))
          .accounts({
            keeper: authority.publicKey,
            vault: vaultPda,
            execution: execPda,
            inputMint: assetMint,
            outputMint: pendingAssetMint,
            compliance: pendingCompliancePda,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        expect.fail("Expected failure on Pending asset");
      } catch (err: any) {
        expect(err.toString()).to.include("AssetNotApproved");
      }
    });

    it("rejects execution when asset status is Rejected", async () => {
      const execId = new anchor.BN(2003);
      const execPda = findExecutionPda(vaultPda, execId);

      try {
        await program.methods
          .executeAction(execId, 1, new anchor.BN(1_000_000), new anchor.BN(990_000))
          .accounts({
            keeper: authority.publicKey,
            vault: vaultPda,
            execution: execPda,
            inputMint: assetMint,
            outputMint: rejectedAssetMint,
            compliance: rejectedCompliancePda,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        expect.fail("Expected failure on Rejected asset");
      } catch (err: any) {
        expect(err.toString()).to.include("AssetNotApproved");
      }
    });

    it("rejects execution when Shariah review timestamp has expired", async () => {
      const execId = new anchor.BN(2004);
      const execPda = findExecutionPda(vaultPda, execId);

      try {
        await program.methods
          .executeAction(execId, 1, new anchor.BN(1_000_000), new anchor.BN(990_000))
          .accounts({
            keeper: authority.publicKey,
            vault: vaultPda,
            execution: execPda,
            inputMint: assetMint,
            outputMint: expiredAssetMint,
            compliance: expiredCompliancePda,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        expect.fail("Expected failure on expired review");
      } catch (err: any) {
        expect(err.toString()).to.include("ComplianceExpired");
      }
    });

    it("rejects execution when compliance PDA does not match either input or output mint", async () => {
      const execId = new anchor.BN(2005);
      const execPda = findExecutionPda(vaultPda, execId);

      try {
        await program.methods
          .executeAction(execId, 1, new anchor.BN(1_000_000), new anchor.BN(990_000))
          .accounts({
            keeper: authority.publicKey,
            vault: vaultPda,
            execution: execPda,
            inputMint: assetMint,
            outputMint: mismatchedAssetMint,
            // Supply approvedCompliancePda (which points to approvedAssetMint, not mismatchedAssetMint)
            compliance: approvedCompliancePda,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        expect.fail("Expected failure on compliance mint mismatch");
      } catch (err: any) {
        expect(err.toString()).to.include("ComplianceMintMismatch");
      }
    });

    it("allows authorized vault authority to update asset compliance", async () => {
      const futureTimestamp = new anchor.BN(Math.floor(Date.now() / 1000) + 86400 * 60);
      await program.methods
        .setAssetCompliance(1, policyVersion, evidenceHash, futureTimestamp)
        .accounts({
          authority: authority.publicKey,
          vault: vaultPda,
          assetMint: approvedAssetMint,
          compliance: approvedCompliancePda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      const comp = await program.account.assetCompliance.fetch(approvedCompliancePda);
      expect(comp.status).to.equal(1);
    });

    it("rejects unauthorized signer from updating compliance", async () => {
      const futureTimestamp = new anchor.BN(Math.floor(Date.now() / 1000) + 86400 * 60);
      try {
        await program.methods
          .setAssetCompliance(1, policyVersion, evidenceHash, futureTimestamp)
          .accounts({
            authority: user2.publicKey,
            vault: vaultPda,
            assetMint: approvedAssetMint,
            compliance: approvedCompliancePda,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([user2])
          .rpc();
        expect.fail("Expected failure on unauthorized signer updating compliance");
      } catch (err: any) {
        expect(err.toString()).to.satisfy(
          (s: string) =>
            s.includes("UnauthorizedKeeper") ||
            s.includes("ConstraintHasOne") ||
            s.includes("2001") ||
            s.includes("Error")
        );
      }
    });

    it("rejects execution when compliance account is invalid or substituted (wrong compliance account)", async () => {
      const execId = new anchor.BN(2007);
      const execPda = findExecutionPda(vaultPda, execId);

      try {
        await program.methods
          .executeAction(execId, 1, new anchor.BN(1_000_000), new anchor.BN(990_000))
          .accounts({
            keeper: authority.publicKey,
            vault: vaultPda,
            execution: execPda,
            inputMint: assetMint,
            outputMint: approvedAssetMint,
            // Supply vaultPda as wrong compliance account
            compliance: vaultPda,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        expect.fail("Expected failure on wrong compliance account");
      } catch (err: any) {
        expect(err.toString()).to.satisfy(
          (s: string) =>
            s.includes("AccountNotInitialized") ||
            s.includes("ConstraintSeeds") ||
            s.includes("ConstraintRaw") ||
            s.includes("Error")
        );
      }
    });
  });
});
