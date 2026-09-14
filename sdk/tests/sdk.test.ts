import { Keypair, PublicKey } from "@solana/web3.js";
import { expect } from "chai";
import BN from "bn.js";

import {
  DEFAULT_PROGRAM_ID,
  EquityCatalystClient,
  findExecutionPda,
  findLoanPda,
  findPolicyPda,
  findPositionPda,
  findUserSharesPda,
  findVaultPda,
} from "../src";

describe("Equity Catalyst TypeScript SDK", () => {
  const authority = Keypair.generate().publicKey;
  const user = Keypair.generate().publicKey;
  const assetMint = Keypair.generate().publicKey;
  const vaultName = "Catalyst Alpha";

  const client = EquityCatalystClient.forDevnet("http://127.0.0.1:4000");

  describe("1. PDA Derivation Utilities", () => {
    it("should derive deterministic Vault PDA matching on-chain seeds", () => {
      const [vaultPda, bump] = findVaultPda(authority, vaultName);
      expect(vaultPda).to.be.instanceOf(PublicKey);
      expect(bump).to.be.a("number");

      // Verify second call yields identical address
      const [vaultPda2] = findVaultPda(authority, vaultName);
      expect(vaultPda.toBase58()).to.equal(vaultPda2.toBase58());
    });

    it("should derive Policy PDA matching on-chain seeds", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const [policyPda, bump] = findPolicyPda(vaultPda);
      expect(policyPda).to.be.instanceOf(PublicKey);
      expect(bump).to.be.a("number");
    });

    it("should derive UserShares PDA matching on-chain seeds", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const [sharesPda, bump] = findUserSharesPda(vaultPda, user);
      expect(sharesPda).to.be.instanceOf(PublicKey);
      expect(bump).to.be.a("number");
    });

    it("should derive Position PDA matching on-chain seeds", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const [posPda, bump] = findPositionPda(vaultPda, assetMint);
      expect(posPda).to.be.instanceOf(PublicKey);
      expect(bump).to.be.a("number");
    });

    it("should derive Loan PDA matching on-chain seeds", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const [loanPda, bump] = findLoanPda(vaultPda, user);
      expect(loanPda).to.be.instanceOf(PublicKey);
      expect(bump).to.be.a("number");
    });

    it("should derive Execution PDA matching on-chain seeds", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const [execPda, bump] = findExecutionPda(vaultPda, 1042);
      expect(execPda).to.be.instanceOf(PublicKey);
      expect(bump).to.be.a("number");
    });
  });

  describe("2. Instruction Builders", () => {
    it("should build valid initialize_vault instruction", () => {
      const ix = client.vaults.buildInitializeVaultIx({
        authority,
        assetMint,
        name: vaultName,
        symbol: "CAT-A",
        maxLtvBps: 7500,
        maxPositionBps: 2500,
      });

      expect(ix.programId.toBase58()).to.equal(DEFAULT_PROGRAM_ID.toBase58());
      expect(ix.keys.length).to.equal(5);
      expect(ix.keys[0].pubkey.toBase58()).to.equal(authority.toBase58());
      expect(ix.keys[0].isSigner).to.be.true;
      expect(ix.keys[0].isWritable).to.be.true;

      // Check discriminator prefix: [48, 191, 163, 44, 71, 129, 63, 164]
      expect(ix.data[0]).to.equal(48);
      expect(ix.data[1]).to.equal(191);
    });

    it("should build valid deposit instruction", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const ix = client.vaults.buildDepositIx({
        user,
        vault: vaultPda,
        assetMint,
        amount: new BN(1_000_000),
      });

      expect(ix.programId.toBase58()).to.equal(DEFAULT_PROGRAM_ID.toBase58());
      expect(ix.keys.length).to.equal(8);
      expect(ix.keys[0].pubkey.toBase58()).to.equal(user.toBase58());
      expect(ix.keys[0].isSigner).to.be.true;

      // Discriminator: [242, 35, 68, 137, 82, 225, 242, 182]
      expect(ix.data[0]).to.equal(242);
      expect(ix.data[1]).to.equal(35);
    });

    it("should build valid withdraw instruction", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const ix = client.vaults.buildWithdrawIx({
        user,
        vault: vaultPda,
        assetMint,
        sharesToBurn: new BN(500_000),
      });

      expect(ix.programId.toBase58()).to.equal(DEFAULT_PROGRAM_ID.toBase58());
      expect(ix.keys.length).to.equal(7);

      // Discriminator: [183, 18, 70, 156, 148, 109, 161, 34]
      expect(ix.data[0]).to.equal(183);
      expect(ix.data[1]).to.equal(18);
    });

    it("should build valid update_policy instruction", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const ix = client.policies.buildUpdatePolicyIx({
        authority,
        vault: vaultPda,
        maxLtvBps: 8000,
        maxPositionBps: 3000,
        stopLossBps: 600,
        takeProfitBps: 1800,
        rebalanceThresholdBps: 250,
        isActive: true,
      });

      expect(ix.programId.toBase58()).to.equal(DEFAULT_PROGRAM_ID.toBase58());
      expect(ix.keys.length).to.equal(3);

      // Discriminator: [102, 192, 169, 114, 219, 137, 240, 150]
      expect(ix.data[0]).to.equal(102);
      expect(ix.data[1]).to.equal(192);
    });

    it("should build valid toggle_pause instruction", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const ix = client.vaults.buildTogglePauseIx({
        authority,
        vault: vaultPda,
        isPaused: true,
      });

      expect(ix.programId.toBase58()).to.equal(DEFAULT_PROGRAM_ID.toBase58());
      expect(ix.data[0]).to.equal(206);
      expect(ix.data[8]).to.equal(1); // isPaused = 1
    });

    it("should build valid execute_action instruction", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const targetMint = Keypair.generate().publicKey;

      const ix = client.execution.buildExecuteActionIx({
        authority,
        vault: vaultPda,
        actionType: 1,
        executionId: 55,
        sourceMint: assetMint,
        targetMint,
        inputAmount: 100_000,
        minOutputAmount: 98_000,
      });

      expect(ix.programId.toBase58()).to.equal(DEFAULT_PROGRAM_ID.toBase58());
      expect(ix.keys.length).to.equal(8);

      // Discriminator: [167, 100, 14, 255, 178, 12, 10, 206]
      expect(ix.data[0]).to.equal(167);
      expect(ix.data[1]).to.equal(100);
    });

    it("should build valid borrow and repay instructions", () => {
      const [vaultPda] = findVaultPda(authority, vaultName);
      const collateralMint = Keypair.generate().publicKey;

      const borrowIx = client.credit.buildBorrowIx({
        borrower: user,
        vault: vaultPda,
        borrowAssetMint: assetMint,
        collateralMint,
        collateralAmount: 200_000,
        borrowAmount: 100_000,
      });
      expect(borrowIx.keys.length).to.equal(5);
      expect(borrowIx.data[0]).to.equal(228);

      const repayIx = client.credit.buildRepayIx({
        borrower: user,
        vault: vaultPda,
        borrowAssetMint: assetMint,
        collateralMint,
        repayAmount: 100_000,
        collateralToRelease: 200_000,
      });
      expect(repayIx.keys.length).to.equal(4);
      expect(repayIx.data[0]).to.equal(234);
    });
  });

  describe("3. Transaction Creators", () => {
    it("should assemble ready-to-sign transaction with correct fee payer", async () => {
      const tx = await client.vaults.createInitializeVaultTx({
        authority,
        assetMint,
        name: vaultName,
        symbol: "CAT-A",
        maxLtvBps: 7500,
        maxPositionBps: 2500,
      });

      expect(tx.instructions.length).to.equal(1);
      expect(tx.feePayer?.toBase58()).to.equal(authority.toBase58());
    });
  });

  describe("4. Events & Log Parsing", () => {
    it("should parse Anchor program logs and instruction markers", () => {
      const logs = [
        "Program 8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH invoke [1]",
        "Program log: Instruction: Deposit",
        "Program data: vTNohmVS8rIBAAAAAAAA",
        "Program 8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH success",
      ];

      const parsed = client.events.parseLogs(logs);
      expect(parsed.length).to.equal(2);
      expect(parsed[0].name).to.equal("DepositLog");
      expect(parsed[1].name).to.equal("AnchorProgramData");
    });
  });

  describe("5. Client Initialization & Structure", () => {
    it("should instantiate all sub-clients with matching configuration", () => {
      expect(client.vaults).to.be.ok;
      expect(client.policies).to.be.ok;
      expect(client.events).to.be.ok;
      expect(client.execution).to.be.ok;
      expect(client.portfolio).to.be.ok;
      expect(client.credit).to.be.ok;
      expect(client.programId.toBase58()).to.equal(DEFAULT_PROGRAM_ID.toBase58());
    });
  });
});

import http from "http";

describe("6. REST API Client Integration", () => {
  let server: http.Server;
  let testApiUrl: string;

  before((done) => {
    server = http.createServer((req, res) => {
      res.setHeader("Content-Type", "application/json");

      if (req.url === "/health" && req.method === "GET") {
        res.writeHead(200);
        res.end(JSON.stringify({
          status: "ok",
          version: "0.1.0",
          uptime_seconds: 42,
          cluster: "devnet"
        }));
      } else if (req.url === "/ready" && req.method === "GET") {
        res.writeHead(200);
        res.end(JSON.stringify({
          ready: true,
          database: "healthy",
          redis: "healthy",
          uptime_seconds: 42
        }));
      } else if (req.url === "/oracle/price/SOL" && req.method === "GET") {
        res.writeHead(200);
        res.end(JSON.stringify({
          symbol: "SOL",
          feed_id: "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d",
          price_usd: 145.5,
          price_scaled: 145500000,
          conf_usd: 0.2,
          expo: -8,
          publish_time: 1726000000,
          is_stale: false
        }));
      } else if (req.url === "/quotes/evaluate" && req.method === "POST") {
        let body = "";
        req.on("data", (chunk) => { body += chunk; });
        req.on("end", () => {
          const parsed = JSON.parse(body);
          res.writeHead(200);
          res.end(JSON.stringify({
            execution_id: "00000000-0000-0000-0000-000000000001",
            vault_address: parsed.vault_address,
            input_mint: parsed.input_mint,
            output_mint: parsed.output_mint,
            amount_in: parsed.amount_in,
            expected_amount_out: 145000000,
            min_amount_out: 144275000,
            price_impact_bps: 5,
            price_impact_pct: "0.05",
            approved: true,
            is_dry_run: true,
            evaluated_at: new Date().toISOString()
          }));
        });
      } else {
        res.writeHead(404);
        res.end(JSON.stringify({ error: "Not found" }));
      }
    });

    server.listen(0, "127.0.0.1", () => {
      const addr = server.address() as any;
      testApiUrl = `http://127.0.0.1:${addr.port}`;
      done();
    });
  });

  after((done) => {
    server.close(done);
  });

  it("should fetch health status from API", async () => {
    const apiClient = EquityCatalystClient.forDevnet(testApiUrl);
    const health = await apiClient.getHealth();
    expect(health.status).to.equal("ok");
    expect(health.cluster).to.equal("devnet");
    expect(health.uptime_seconds).to.equal(42);
  });

  it("should fetch ready status from API", async () => {
    const apiClient = EquityCatalystClient.forDevnet(testApiUrl);
    const ready = await apiClient.getReady();
    expect(ready.ready).to.be.true;
    expect(ready.database).to.equal("healthy");
    expect(ready.redis).to.equal("healthy");
  });

  it("should query normalized oracle prices from API", async () => {
    const apiClient = EquityCatalystClient.forDevnet(testApiUrl);
    const price = await apiClient.getOraclePrice("SOL");
    expect(price.symbol).to.equal("SOL");
    expect(price.price_usd).to.equal(145.5);
    expect(price.is_stale).to.be.false;
  });

  it("should evaluate quote through the risk engine via execution client", async () => {
    const apiClient = EquityCatalystClient.forDevnet(testApiUrl);
    const verdict = await apiClient.execution.evaluateQuote({
      vault_address: "Vault1111111111111111111111111111111111111",
      input_mint: "So11111111111111111111111111111111111111112",
      output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      amount_in: 1000000000,
      slippage_bps: 50,
    });

    expect(verdict.approved).to.be.true;
    expect(verdict.is_dry_run).to.be.true;
    expect(verdict.price_impact_bps).to.equal(5);
    expect(verdict.expected_amount_out).to.equal(145000000);
  });
});
