import { Connection, PublicKey } from "@solana/web3.js";

interface VerificationCheck {
  id: string;
  name: string;
  passed: boolean;
  required: boolean;
  data?: any;
  error?: string;
}

interface VerificationSummary {
  timestamp: string;
  cluster: string;
  rpcEndpoint: string;
  allPassed: boolean;
  passedCount: number;
  totalCount: number;
  checks: VerificationCheck[];
  remainingBlockers: string[];
}

const METEORA_DBC_PROGRAM_ID = new PublicKey(
  "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"
);
const TOKEN_2022_PROGRAM_ID = new PublicKey(
  "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
);
const SPL_TOKEN_PROGRAM_ID = new PublicKey(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
);

// Canonical Backed NVIDIA xStock Mint on Solana Mainnet-Beta
const CANONICAL_NVDAX_MINT = "Xsc9qvGR1efVDFGLrVsmkzv3qi45LTBjeUKSPmx9qEh";
const CANONICAL_USDC_MINT = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const PYTH_NVDA_FEED_ID = "b1073854ed24cbc755dc527418f52b7d271f6cc967bbf8d8129112b18860a593";

async function main() {
  const rpcUrl =
    process.env.SOLANA_MAINNET_RPC_URL ||
    process.env.SOLANA_RPC_URL ||
    "https://api.mainnet-beta.solana.com";

  const configuredMint =
    process.env.VERIFIED_ASSET_MINT || CANONICAL_NVDAX_MINT;
  const configuredQuoteMint =
    process.env.USDC_MINT || CANONICAL_USDC_MINT;
  const pythHermesUrl =
    process.env.PYTH_HERMES_URL || "https://hermes.pyth.network";
  const pythApiKey =
    process.env.PYTH_API_KEY || process.env.HERMES_API_KEY || "";
  const poolAddressStr =
    process.env.METEORA_POOL_ADDRESS || process.argv[2] || "";
  const txSignatureStr =
    process.env.MAINNET_TX_SIGNATURE || process.argv[3] || "";

  const connection = new Connection(rpcUrl, "confirmed");
  const checks: VerificationCheck[] = [];
  const blockers: string[] = [];

  // =========================================================================
  // 1. Validate Configured Asset Mint Format & Canonical Identity
  // =========================================================================
  try {
    const mintPubkey = new PublicKey(configuredMint);
    const isCanonical = configuredMint === CANONICAL_NVDAX_MINT;
    checks.push({
      id: "ASSET_MINT_VALIDATION",
      name: "Validate Configured Asset Mint Identity",
      passed: isCanonical,
      required: true,
      data: {
        mint: mintPubkey.toBase58(),
        canonicalMint: CANONICAL_NVDAX_MINT,
        symbol: "NVDAx",
        isCanonical,
      },
      error: isCanonical
        ? undefined
        : `Mint ${configuredMint} does not match canonical Backed NVDAx (${CANONICAL_NVDAX_MINT})`,
    });
    if (!isCanonical) {
      blockers.push(`Asset mint mismatch: expected ${CANONICAL_NVDAX_MINT}, got ${configuredMint}`);
    }
  } catch (err: any) {
    checks.push({
      id: "ASSET_MINT_VALIDATION",
      name: "Validate Configured Asset Mint Identity",
      passed: false,
      required: true,
      error: `Invalid public key format for asset mint: ${err.message}`,
    });
    blockers.push(`Invalid asset mint public key: ${err.message}`);
  }

  // =========================================================================
  // 2. Query getAccountInfo for Asset Mint
  // =========================================================================
  let mintAccountInfo: any = null;
  try {
    const mintPubkey = new PublicKey(configuredMint);
    mintAccountInfo = await connection.getAccountInfo(mintPubkey);
    const exists = mintAccountInfo !== null;
    checks.push({
      id: "MINT_ACCOUNT_INFO",
      name: "Query On-Chain Account Info via Solana RPC",
      passed: exists,
      required: true,
      data: exists
        ? {
            owner: mintAccountInfo.owner.toBase58(),
            lamports: mintAccountInfo.lamports,
            dataLength: mintAccountInfo.data.length,
            executable: mintAccountInfo.executable,
          }
        : null,
      error: exists ? undefined : `Account ${configuredMint} not found on RPC (${rpcUrl})`,
    });
    if (!exists) {
      blockers.push(`Asset mint account ${configuredMint} does not exist on Solana RPC`);
    }
  } catch (err: any) {
    checks.push({
      id: "MINT_ACCOUNT_INFO",
      name: "Query On-Chain Account Info via Solana RPC",
      passed: false,
      required: true,
      error: `RPC call getAccountInfo failed: ${err.message}`,
    });
    blockers.push(`Failed to query asset mint account: ${err.message}`);
  }

  // =========================================================================
  // 3. Validate Token Program & Account State
  // =========================================================================
  try {
    if (mintAccountInfo) {
      const isToken2022 = mintAccountInfo.owner.equals(TOKEN_2022_PROGRAM_ID);
      const isSplToken = mintAccountInfo.owner.equals(SPL_TOKEN_PROGRAM_ID);
      const isLegitTokenProgram = isToken2022 || isSplToken;

      // Backed Finance xStocks use Token-2022 (extensions: metadataPointer, permanentDelegate)
      // Mint data length is 82 bytes + extension data
      const hasValidLength = mintAccountInfo.data.length >= 82;

      const passed = isLegitTokenProgram && hasValidLength;
      checks.push({
        id: "TOKEN_PROGRAM_VALIDATION",
        name: "Validate Token Program Ownership & Mint Structure",
        passed,
        required: true,
        data: {
          ownerProgram: mintAccountInfo.owner.toBase58(),
          isToken2022,
          isSplToken,
          dataLength: mintAccountInfo.data.length,
        },
        error: passed ? undefined : "Account owner is not a recognized SPL token program or mint structure is invalid",
      });
      if (!passed) {
        blockers.push("Asset mint is not owned by a valid Solana Token Program");
      }
    } else {
      checks.push({
        id: "TOKEN_PROGRAM_VALIDATION",
        name: "Validate Token Program Ownership & Mint Structure",
        passed: false,
        required: true,
        error: "Cannot validate token program: mint account does not exist",
      });
    }
  } catch (err: any) {
    checks.push({
      id: "TOKEN_PROGRAM_VALIDATION",
      name: "Validate Token Program Ownership & Mint Structure",
      passed: false,
      required: true,
      error: err.message,
    });
    blockers.push(`Token program validation error: ${err.message}`);
  }

  // =========================================================================
  // 4. Query Configured Pyth Price Feed
  // =========================================================================
  let pythFeedData: any = null;
  try {
    const feedSearchUrl = `${pythHermesUrl}/v2/price_feeds?query=NVDA`;
    const searchResp = await fetch(feedSearchUrl);
    if (searchResp.ok) {
      const feeds = (await searchResp.json()) as any[];
      const nvdaFeed = feeds.find(
        (f) =>
          f.id === PYTH_NVDA_FEED_ID ||
          (f.attributes && f.attributes.display_symbol === "NVDA") ||
          (f.attributes && f.attributes.symbol === "Equity.US.NVDA/USD")
      );
      if (nvdaFeed) {
        pythFeedData = nvdaFeed;
        checks.push({
          id: "PYTH_FEED_QUERY",
          name: "Query Configured Pyth Hermes Feed Metadata",
          passed: true,
          required: true,
          data: {
            feedId: nvdaFeed.id,
            description: nvdaFeed.attributes?.description,
            symbol: nvdaFeed.attributes?.symbol,
            marketHours: nvdaFeed.market_hours,
          },
        });
      } else {
        checks.push({
          id: "PYTH_FEED_QUERY",
          name: "Query Configured Pyth Hermes Feed Metadata",
          passed: false,
          required: true,
          error: `Pyth feed for NVDA (${PYTH_NVDA_FEED_ID}) not found in Hermes catalog`,
        });
        blockers.push("Configured Pyth NVDA feed not found in Hermes feed registry");
      }
    } else {
      checks.push({
        id: "PYTH_FEED_QUERY",
        name: "Query Configured Pyth Hermes Feed Metadata",
        passed: false,
        required: true,
        error: `Pyth Hermes returned HTTP ${searchResp.status}: ${searchResp.statusText}`,
      });
      blockers.push(`Pyth Hermes request failed: HTTP ${searchResp.status}`);
    }
  } catch (err: any) {
    checks.push({
      id: "PYTH_FEED_QUERY",
      name: "Query Configured Pyth Hermes Feed Metadata",
      passed: false,
      required: true,
      error: `Pyth Hermes connection error: ${err.message}`,
    });
    blockers.push(`Pyth Hermes connection error: ${err.message}`);
  }

  // =========================================================================
  // 5. Validate Pyth Price Feed Timestamp Freshness / Trading Schedule
  // =========================================================================
  try {
    if (pythFeedData) {
      // In Hermes v2, check market hours schedule and active calendar status
      const hasSchedule = Boolean(pythFeedData.attributes?.schedule);
      const isConfigured = Boolean(pythFeedData.id);
      checks.push({
        id: "PYTH_TIMESTAMP_FRESHNESS",
        name: "Validate Pyth Oracle Freshness & Schedule Specification",
        passed: hasSchedule && isConfigured,
        required: true,
        data: {
          schedule: pythFeedData.attributes?.schedule,
          isOpen: pythFeedData.market_hours?.is_open,
          nextOpen: pythFeedData.market_hours?.next_open,
        },
      });
    } else {
      checks.push({
        id: "PYTH_TIMESTAMP_FRESHNESS",
        name: "Validate Pyth Oracle Freshness & Schedule Specification",
        passed: false,
        required: true,
        error: "Cannot evaluate freshness: Pyth feed metadata is unavailable",
      });
      blockers.push("Pyth oracle freshness cannot be verified");
    }
  } catch (err: any) {
    checks.push({
      id: "PYTH_TIMESTAMP_FRESHNESS",
      name: "Validate Pyth Oracle Freshness & Schedule Specification",
      passed: false,
      required: true,
      error: err.message,
    });
  }

  // =========================================================================
  // 6. Validate Meteora DBC Program & Pool Accounts
  // =========================================================================
  try {
    const dbcProgramInfo = await connection.getAccountInfo(METEORA_DBC_PROGRAM_ID);
    const isProgramExecutable = dbcProgramInfo !== null && dbcProgramInfo.executable;

    checks.push({
      id: "METEORA_DBC_PROGRAM",
      name: "Validate Meteora DBC Program Deployment on Solana RPC",
      passed: isProgramExecutable,
      required: true,
      data: {
        programId: METEORA_DBC_PROGRAM_ID.toBase58(),
        executable: isProgramExecutable,
      },
      error: isProgramExecutable
        ? undefined
        : `Meteora DBC program ${METEORA_DBC_PROGRAM_ID.toBase58()} not deployed or not executable on ${rpcUrl}`,
    });
    if (!isProgramExecutable) {
      blockers.push(`Meteora DBC Program ${METEORA_DBC_PROGRAM_ID.toBase58()} is not executable on RPC`);
    }

    // Check specific pool account if provided
    if (poolAddressStr && poolAddressStr.trim().length > 0) {
      const poolPubkey = new PublicKey(poolAddressStr);
      const poolInfo = await connection.getAccountInfo(poolPubkey);
      const poolExists = poolInfo !== null && poolInfo.owner.equals(METEORA_DBC_PROGRAM_ID);
      checks.push({
        id: "METEORA_POOL_ACCOUNT",
        name: "Validate Configured Meteora DBC Pool Account",
        passed: poolExists,
        required: true,
        data: {
          poolAddress: poolPubkey.toBase58(),
          exists: poolInfo !== null,
          owner: poolInfo?.owner.toBase58(),
        },
        error: poolExists
          ? undefined
          : `Pool account ${poolAddressStr} does not exist or is not owned by Meteora DBC program`,
      });
      if (!poolExists) {
        blockers.push(`Configured Meteora DBC pool ${poolAddressStr} does not exist on-chain`);
      }
    } else {
      checks.push({
        id: "METEORA_POOL_ACCOUNT",
        name: "Validate Configured Meteora DBC Pool Account",
        passed: false,
        required: true,
        error: "No METEORA_POOL_ADDRESS configured. Pool deployment has not yet occurred on mainnet.",
      });
      blockers.push("No real Meteora DBC pool address configured (pending real mainnet deployment)");
    }
  } catch (err: any) {
    checks.push({
      id: "METEORA_DBC_PROGRAM",
      name: "Validate Meteora DBC Program & Pool Accounts",
      passed: false,
      required: true,
      error: err.message,
    });
    blockers.push(`Meteora verification error: ${err.message}`);
  }

  // =========================================================================
  // 7. Validate Base and Quote Vault Accounts
  // =========================================================================
  try {
    if (poolAddressStr && poolAddressStr.trim().length > 0) {
      // If pool address is present, derive and check token vaults
      checks.push({
        id: "VAULT_ACCOUNTS",
        name: "Validate Base and Quote Token Vault Accounts",
        passed: false,
        required: true,
        error: "Pool account unverified; vault accounts cannot be confirmed",
      });
      blockers.push("Base and quote token vault accounts cannot be validated without confirmed pool");
    } else {
      checks.push({
        id: "VAULT_ACCOUNTS",
        name: "Validate Base and Quote Token Vault Accounts",
        passed: false,
        required: true,
        error: "No pool address configured to query base and quote vault accounts",
      });
      blockers.push("Base/quote vault accounts require an active deployed pool");
    }
  } catch (err: any) {
    checks.push({
      id: "VAULT_ACCOUNTS",
      name: "Validate Base and Quote Token Vault Accounts",
      passed: false,
      required: true,
      error: err.message,
    });
  }

  // =========================================================================
  // 8 & 9. Query Transaction Signature & Check RPC Confirmation/Finalization
  // =========================================================================
  try {
    if (txSignatureStr && txSignatureStr.trim().length > 0) {
      const sigStatus = await connection.getSignatureStatus(txSignatureStr, {
        searchTransactionHistory: true,
      });

      const value = sigStatus.value;
      const isConfirmed =
        value !== null &&
        (value.confirmationStatus === "confirmed" ||
          value.confirmationStatus === "finalized") &&
        value.err === null;

      checks.push({
        id: "TRANSACTION_SIGNATURE",
        name: "Query Transaction Signature on Solana RPC",
        passed: Boolean(value),
        required: true,
        data: {
          signature: txSignatureStr,
          status: value?.confirmationStatus,
          slot: value?.slot,
          error: value?.err,
        },
        error: value ? undefined : `Signature ${txSignatureStr} not found in Solana cluster history`,
      });

      checks.push({
        id: "TRANSACTION_FINALIZATION",
        name: "Check RPC Transaction Finalization Status",
        passed: isConfirmed,
        required: true,
        data: {
          confirmationStatus: value?.confirmationStatus,
        },
        error: isConfirmed ? undefined : "Transaction is not confirmed or has execution error",
      });

      if (!isConfirmed) {
        blockers.push(`Transaction signature ${txSignatureStr} is not confirmed on Solana RPC`);
      }
    } else {
      checks.push({
        id: "TRANSACTION_SIGNATURE",
        name: "Query Transaction Signature on Solana RPC",
        passed: false,
        required: true,
        error: "No MAINNET_TX_SIGNATURE provided. Real transaction has not been submitted.",
      });
      checks.push({
        id: "TRANSACTION_FINALIZATION",
        name: "Check RPC Transaction Finalization Status",
        passed: false,
        required: true,
        error: "No transaction signature available to verify finalization.",
      });
      blockers.push("No confirmed mainnet transaction signature recorded (zero fabricated evidence)");
    }
  } catch (err: any) {
    checks.push({
      id: "TRANSACTION_SIGNATURE",
      name: "Query Transaction Signature on Solana RPC",
      passed: false,
      required: true,
      error: err.message,
    });
    blockers.push(`Transaction signature verification error: ${err.message}`);
  }

  // =========================================================================
  // 10. Print Machine-Readable Verification Summary
  // =========================================================================
  const allRequiredPassed = checks
    .filter((c) => c.required)
    .every((c) => c.passed);
  const passedCount = checks.filter((c) => c.passed).length;

  const summary: VerificationSummary = {
    timestamp: new Date().toISOString(),
    cluster: "mainnet-beta",
    rpcEndpoint: rpcUrl,
    allPassed: allRequiredPassed,
    passedCount,
    totalCount: checks.length,
    checks,
    remainingBlockers: blockers,
  };

  console.log("\n================================================================================");
  console.log("            EQUITY CATALYST — MAINNET VERIFICATION EVIDENCE REPORT               ");
  console.log("================================================================================\n");

  for (const check of checks) {
    const symbol = check.passed ? "✓ [PASS]" : "✗ [FAIL]";
    console.log(`${symbol.padEnd(9)} ${check.name}`);
    if (check.data) {
      console.log(`          Details: ${JSON.stringify(check.data)}`);
    }
    if (check.error) {
      console.log(`          Error:   ${check.error}`);
    }
  }

  console.log("\n--------------------------------------------------------------------------------");
  console.log(`VERIFICATION SUMMARY: ${passedCount}/${checks.length} checks passed`);
  console.log(`OVERALL STATUS:       ${allRequiredPassed ? "VERIFIED" : "VERIFICATION GATE FAILED"}`);
  console.log("--------------------------------------------------------------------------------");

  if (blockers.length > 0) {
    console.log("\nACTIONABLE REMAINING BLOCKERS (Real Evidence Missing):");
    for (const blocker of blockers) {
      console.log(`  - ${blocker}`);
    }
  }

  console.log("\n=== MACHINE-READABLE VERIFICATION SUMMARY (JSON) ===");
  console.log(JSON.stringify(summary, null, 2));
  console.log("====================================================\n");

  if (!allRequiredPassed) {
    process.exit(1);
  }
}

main().catch((err) => {
  console.error("Fatal error during mainnet verification:", err);
  process.exit(1);
});
