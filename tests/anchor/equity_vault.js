"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
const anchor = __importStar(require("@coral-xyz/anchor"));
const web3_js_1 = require("@solana/web3.js");
const spl_token_1 = require("@solana/spl-token");
const chai_1 = require("chai");
describe("equity_vault - Milestones 1 & 2", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const program = anchor.workspace.EquityVault;
    const authority = provider.wallet.payer;
    let assetMint;
    let vaultPda;
    let vaultBump;
    let policyPda;
    let policyBump;
    let vaultAssetAccount;
    const vaultName = "Catalyst Alpha";
    const vaultSymbol = "CAT-A";
    const initialMaxLtvBps = 7500; // 75%
    const initialMaxPositionBps = 2500; // 25%
    // User 1 (Authority)
    let user1AssetAccount;
    // User 2
    const user2 = web3_js_1.Keypair.generate();
    let user2AssetAccount;
    let user2SharesPda;
    before(async () => {
        // 1. Airdrop SOL to user2
        const airdropSig = await provider.connection.requestAirdrop(user2.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
        await provider.connection.confirmTransaction(airdropSig, "confirmed");
        // 2. Create underlying asset mint
        assetMint = await (0, spl_token_1.createMint)(provider.connection, authority, authority.publicKey, null, 6);
        // 3. Derive Vault & Policy PDAs
        [vaultPda, vaultBump] = web3_js_1.PublicKey.findProgramAddressSync([Buffer.from("vault"), authority.publicKey.toBuffer(), Buffer.from(vaultName)], program.programId);
        [policyPda, policyBump] = web3_js_1.PublicKey.findProgramAddressSync([Buffer.from("policy"), vaultPda.toBuffer()], program.programId);
        // 4. Create Vault ATA for assetMint owned by Vault PDA
        vaultAssetAccount = (0, spl_token_1.getAssociatedTokenAddressSync)(assetMint, vaultPda, true);
        const createVaultAtaIx = (0, spl_token_1.createAssociatedTokenAccountInstruction)(authority.publicKey, vaultAssetAccount, vaultPda, assetMint);
        const createAtaTx = new anchor.web3.Transaction().add(createVaultAtaIx);
        await provider.sendAndConfirm(createAtaTx);
        // 5. Create User1 Token Account & mint 10,000,000 units
        user1AssetAccount = await (0, spl_token_1.createAccount)(provider.connection, authority, assetMint, authority.publicKey);
        await (0, spl_token_1.mintTo)(provider.connection, authority, assetMint, user1AssetAccount, authority, 10000000);
        // 6. Create User2 Token Account & mint 5,000,000 units
        user2AssetAccount = await (0, spl_token_1.createAccount)(provider.connection, user2, assetMint, user2.publicKey);
        await (0, spl_token_1.mintTo)(provider.connection, authority, assetMint, user2AssetAccount, authority, 5000000);
        // Derive User2 Shares PDA
        [user2SharesPda] = web3_js_1.PublicKey.findProgramAddressSync([Buffer.from("user_shares"), vaultPda.toBuffer(), user2.publicKey.toBuffer()], program.programId);
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
            (0, chai_1.expect)(tx).to.be.a("string");
            const vaultAccount = await program.account.vault.fetch(vaultPda);
            (0, chai_1.expect)(vaultAccount.authority.toBase58()).to.equal(authority.publicKey.toBase58());
            (0, chai_1.expect)(vaultAccount.assetMint.toBase58()).to.equal(assetMint.toBase58());
            (0, chai_1.expect)(vaultAccount.policy.toBase58()).to.equal(policyPda.toBase58());
            (0, chai_1.expect)(vaultAccount.name).to.equal(vaultName);
            (0, chai_1.expect)(vaultAccount.symbol).to.equal(vaultSymbol);
            (0, chai_1.expect)(vaultAccount.totalDeposits.toNumber()).to.equal(0);
            (0, chai_1.expect)(vaultAccount.totalShares.toNumber()).to.equal(0);
            (0, chai_1.expect)(vaultAccount.isPaused).to.be.false;
            const policyAccount = await program.account.policy.fetch(policyPda);
            (0, chai_1.expect)(policyAccount.vault.toBase58()).to.equal(vaultPda.toBase58());
            (0, chai_1.expect)(policyAccount.authority.toBase58()).to.equal(authority.publicKey.toBase58());
            (0, chai_1.expect)(policyAccount.maxLtvBps).to.equal(initialMaxLtvBps);
            (0, chai_1.expect)(policyAccount.maxPositionBps).to.equal(initialMaxPositionBps);
            (0, chai_1.expect)(policyAccount.isActive).to.be.true;
        });
        it("fails to initialize when max_ltv_bps > 10,000", async () => {
            const name = "Inv LTV";
            const [invVault] = web3_js_1.PublicKey.findProgramAddressSync([Buffer.from("vault"), authority.publicKey.toBuffer(), Buffer.from(name)], program.programId);
            const [invPolicy] = web3_js_1.PublicKey.findProgramAddressSync([Buffer.from("policy"), invVault.toBuffer()], program.programId);
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
                chai_1.expect.fail("Expected failure on invalid LTV");
            }
            catch (err) {
                (0, chai_1.expect)(err.toString()).to.include("InvalidRiskLimit");
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
                chai_1.expect.fail("Expected failure on duplicate initialization");
            }
            catch (err) {
                (0, chai_1.expect)(err.toString()).to.match(/already in use|custom program error: 0x0/i);
            }
        });
    });
    describe("Phase 2: Deposit Operations", () => {
        it("rejects deposit below minimum threshold (1,000 units)", async () => {
            const [user1SharesPda] = web3_js_1.PublicKey.findProgramAddressSync([Buffer.from("user_shares"), vaultPda.toBuffer(), authority.publicKey.toBuffer()], program.programId);
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
                chai_1.expect.fail("Expected failure on deposit too small");
            }
            catch (err) {
                (0, chai_1.expect)(err.toString()).to.include("DepositTooSmall");
            }
        });
        it("initial deposit issues 1:1 shares", async () => {
            const depositAmount = new anchor.BN(1000000); // 1.0 token
            const [user1SharesPda] = web3_js_1.PublicKey.findProgramAddressSync([Buffer.from("user_shares"), vaultPda.toBuffer(), authority.publicKey.toBuffer()], program.programId);
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
            (0, chai_1.expect)(vaultAccount.totalDeposits.toNumber()).to.equal(1000000);
            (0, chai_1.expect)(vaultAccount.totalShares.toNumber()).to.equal(1000000);
            // Check UserShares PDA state
            const user1Shares = await program.account.userShares.fetch(user1SharesPda);
            (0, chai_1.expect)(user1Shares.shares.toNumber()).to.equal(1000000);
            (0, chai_1.expect)(user1Shares.user.toBase58()).to.equal(authority.publicKey.toBase58());
            // Check Vault token account balance on-chain
            const vaultTokenAcc = await (0, spl_token_1.getAccount)(provider.connection, vaultAssetAccount);
            (0, chai_1.expect)(Number(vaultTokenAcc.amount)).to.equal(1000000);
        });
        it("second deposit issues proportional shares", async () => {
            const depositAmount = new anchor.BN(500000); // 0.5 token
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
            (0, chai_1.expect)(vaultAccount.totalDeposits.toNumber()).to.equal(1500000);
            (0, chai_1.expect)(vaultAccount.totalShares.toNumber()).to.equal(1500000);
            const user2Shares = await program.account.userShares.fetch(user2SharesPda);
            (0, chai_1.expect)(user2Shares.shares.toNumber()).to.equal(500000);
        });
    });
    describe("Phase 3: Withdraw Operations", () => {
        it("rejects withdrawal when requested shares exceed user balance", async () => {
            try {
                await program.methods
                    .withdraw(new anchor.BN(1000000)) // user2 only has 500_000
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
                chai_1.expect.fail("Expected failure on insufficient shares");
            }
            catch (err) {
                (0, chai_1.expect)(err.toString()).to.include("InsufficientShares");
            }
        });
        it("partial withdrawal burns shares and transfers proportionate tokens", async () => {
            const sharesToBurn = new anchor.BN(250000); // 50% of user2's shares
            const user2BalBefore = await (0, spl_token_1.getAccount)(provider.connection, user2AssetAccount);
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
            const user2BalAfter = await (0, spl_token_1.getAccount)(provider.connection, user2AssetAccount);
            (0, chai_1.expect)(Number(user2BalAfter.amount) - Number(user2BalBefore.amount)).to.equal(250000);
            // Verify UserShares decreased
            const user2Shares = await program.account.userShares.fetch(user2SharesPda);
            (0, chai_1.expect)(user2Shares.shares.toNumber()).to.equal(250000);
            // Verify Vault total state decreased
            const vaultAccount = await program.account.vault.fetch(vaultPda);
            (0, chai_1.expect)(vaultAccount.totalDeposits.toNumber()).to.equal(1250000);
            (0, chai_1.expect)(vaultAccount.totalShares.toNumber()).to.equal(1250000);
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
                .updatePolicy(newMaxLtvBps, newMaxPosBps, newStopLossBps, newTakeProfitBps, newRebalanceBps, true)
                .accounts({
                authority: authority.publicKey,
                vault: vaultPda,
                policy: policyPda,
            })
                .rpc();
            const policyAccount = await program.account.policy.fetch(policyPda);
            (0, chai_1.expect)(policyAccount.maxLtvBps).to.equal(newMaxLtvBps);
            (0, chai_1.expect)(policyAccount.maxPositionBps).to.equal(newMaxPosBps);
            (0, chai_1.expect)(policyAccount.stopLossBps).to.equal(newStopLossBps);
            (0, chai_1.expect)(policyAccount.takeProfitBps).to.equal(newTakeProfitBps);
            (0, chai_1.expect)(policyAccount.rebalanceThresholdBps).to.equal(newRebalanceBps);
            (0, chai_1.expect)(policyAccount.isActive).to.be.true;
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
                chai_1.expect.fail("Expected failure on unauthorized update");
            }
            catch (err) {
                (0, chai_1.expect)(err.toString()).to.match(/UnauthorizedKeeper|ConstraintHasOne|custom program error/i);
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
                chai_1.expect.fail("Expected failure on invalid risk limits");
            }
            catch (err) {
                (0, chai_1.expect)(err.toString()).to.include("InvalidRiskLimit");
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
            (0, chai_1.expect)(vaultAccount.isPaused).to.be.true;
        });
        it("rejects deposits while vault is paused", async () => {
            try {
                await program.methods
                    .deposit(new anchor.BN(100000))
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
                chai_1.expect.fail("Expected deposit to fail when paused");
            }
            catch (err) {
                (0, chai_1.expect)(err.toString()).to.include("VaultPaused");
            }
        });
        it("rejects withdrawals while vault is paused", async () => {
            try {
                await program.methods
                    .withdraw(new anchor.BN(50000))
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
                chai_1.expect.fail("Expected withdrawal to fail when paused");
            }
            catch (err) {
                (0, chai_1.expect)(err.toString()).to.include("VaultPaused");
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
            (0, chai_1.expect)(vaultAccount.isPaused).to.be.false;
            // Deposit now succeeds
            await program.methods
                .deposit(new anchor.BN(100000))
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
            (0, chai_1.expect)(user2Shares.shares.toNumber()).to.be.greaterThan(250000);
        });
    });
});
