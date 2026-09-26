"use client";

import React, { useEffect, useState } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import { PublicKey } from "@solana/web3.js";
import { useWallet } from "@solana/wallet-adapter-react";
import {
  AlertTriangle,
  ArrowDownLeft,
  ArrowLeft,
  ArrowUpRight,
  Check,
  Coins,
  Copy,
  ExternalLink,
  Lock,
  PauseCircle,
  PlayCircle,
  RefreshCw,
  ShieldCheck,
  Unlock,
} from "lucide-react";

import { useSolanaTx } from "../../../hooks/useSolanaTx";
import { TransactionStatus } from "../../../components/TransactionStatus";
import { RiskMeter } from "../../../features/vault/RiskMeter";
import { PositionTable } from "../../../features/portfolio/PositionTable";
import { PolicyBuilder } from "../../../features/policy/PolicyBuilder";
import { EventFeed } from "../../../features/events/EventFeed";
import { DecisionTimeline } from "../../../features/events/DecisionTimeline";
import { getSdkClient } from "../../../lib/sdk";
import { useAppDispatch, useAppSelector } from "../../../store/hooks";
import {
  fetchVaultByAddress,
  setVaultPausedOptimistic,
  updateVaultDepositsOptimistic,
} from "../../../store/vaultsSlice";
import { fetchPolicyByVault } from "../../../store/policySlice";
import {
  fetchPortfolioByVault,
  syncPortfolioPrices,
} from "../../../store/portfolioSlice";
import { Card, CardContent, CardHeader, CardTitle } from "../../../components/ui/card";
import { Button } from "../../../components/ui/button";
import { Badge } from "../../../components/ui/badge";
import { Input } from "../../../components/ui/input";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "../../../components/ui/tabs";

export default function VaultDetailPage() {
  const params = useParams();
  const rawAddress = params?.address as string;
  const vaultAddress = rawAddress || "";

  const dispatch = useAppDispatch();
  const { publicKey, connected } = useWallet();
  const { status: txStatus, executeTx, reset: resetTx } = useSolanaTx();

  // Redux store selectors
  const vault = useAppSelector((state) => state.vaults.selectedVault);
  const policy = useAppSelector((state) => state.policy.currentPolicy);
  const positions = useAppSelector((state) => state.portfolio.positions);
  const isRefreshingPyth = useAppSelector((state) => state.portfolio.isSyncingPrices);
  const isLoading = useAppSelector((state) => state.vaults.isLoading && !vault);

  const [copied, setCopied] = useState<boolean>(false);
  const [activeTab, setActiveTab] = useState<"deposit" | "withdraw">("deposit");

  // Deposit / Withdraw form state
  const [depositAmount, setDepositAmount] = useState<string>("1000");
  const [withdrawShares, setWithdrawShares] = useState<string>("500");
  const [actionError, setActionError] = useState<string | null>(null);
  const [isProcessingAction, setIsProcessingAction] = useState<boolean>(false);

  // Dispatch Redux async thunks on mount or route change
  useEffect(() => {
    if (vaultAddress) {
      dispatch(fetchVaultByAddress(vaultAddress));
      dispatch(fetchPolicyByVault(vaultAddress));
      dispatch(fetchPortfolioByVault(vaultAddress));
    }
  }, [dispatch, vaultAddress]);

  const copyAddress = () => {
    if (vaultAddress) {
      navigator.clipboard.writeText(vaultAddress);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  // Toggle Emergency Pause/Unpause on Solana
  const handleTogglePause = async () => {
    if (!vault || !publicKey) return;
    setActionError(null);
    try {
      const sdk = getSdkClient();
      const vaultPubkey = new PublicKey(vault.vault_address);
      const newPausedState = !vault.is_paused;

      const ix = sdk.vaults.buildTogglePauseIx({
        authority: publicKey,
        vault: vaultPubkey,
        isPaused: newPausedState,
      });

      await executeTx(ix);

      // Dispatch optimistic update to Redux store
      dispatch(
        setVaultPausedOptimistic({
          vaultAddress: vault.vault_address,
          isPaused: newPausedState,
        })
      );
    } catch (err: any) {
      setActionError(err?.message || "Failed to toggle pause status");
    }
  };

  // Execute Deposit
  const handleDeposit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!vault || !publicKey) return;
    setActionError(null);
    setIsProcessingAction(true);

    try {
      const sdk = getSdkClient();
      const amountNum = parseFloat(depositAmount);
      if (isNaN(amountNum) || amountNum <= 0) {
        throw new Error("Please enter a valid deposit amount");
      }

      // Convert to 6 decimals standard (USDC)
      const rawUnits = BigInt(Math.round(amountNum * 1_000_000));

      const ix = sdk.vaults.buildDepositIx({
        user: publicKey,
        vault: new PublicKey(vault.vault_address),
        assetMint: new PublicKey(vault.deposit_mint),
        amount: rawUnits.toString(),
      });

      await executeTx(ix);

      // Optimistically update Redux store
      dispatch(
        updateVaultDepositsOptimistic({
          vaultAddress: vault.vault_address,
          depositDelta: Number(rawUnits),
          sharesDelta: Number(rawUnits),
        })
      );
    } catch (err: any) {
      setActionError(err?.message || "Deposit transaction failed");
    } finally {
      setIsProcessingAction(false);
    }
  };

  // Execute Withdraw
  const handleWithdraw = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!vault || !publicKey) return;
    setActionError(null);
    setIsProcessingAction(true);

    try {
      const sdk = getSdkClient();
      const sharesNum = parseFloat(withdrawShares);
      if (isNaN(sharesNum) || sharesNum <= 0) {
        throw new Error("Please enter valid share count to burn");
      }

      const rawShares = BigInt(Math.round(sharesNum * 1_000_000));

      const ix = sdk.vaults.buildWithdrawIx({
        user: publicKey,
        vault: new PublicKey(vault.vault_address),
        assetMint: new PublicKey(vault.deposit_mint),
        sharesToBurn: rawShares.toString(),
      });

      await executeTx(ix);

      // Optimistically update Redux store
      dispatch(
        updateVaultDepositsOptimistic({
          vaultAddress: vault.vault_address,
          depositDelta: -Number(rawShares),
          sharesDelta: -Number(rawShares),
        })
      );
    } catch (err: any) {
      setActionError(err?.message || "Withdraw transaction failed");
    } finally {
      setIsProcessingAction(false);
    }
  };

  // Sync Pyth Prices via Redux Thunk
  const handleRefreshPyth = () => {
    dispatch(syncPortfolioPrices(vaultAddress));
  };

  if (isLoading) {
    return (
      <Card className="bg-white p-16 text-center space-y-4">
        <RefreshCw className="w-8 h-8 animate-spin text-emerald-600 mx-auto" />
        <h2 className="text-base font-bold text-slate-800">Loading Vault from Redux Store & Solana...</h2>
        <p className="text-xs text-slate-500">Querying on-chain account state and risk guardrails.</p>
      </Card>
    );
  }

  if (!vault) {
    return (
      <Card className="bg-white p-16 text-center space-y-4 border-rose-200">
        <AlertTriangle className="w-8 h-8 text-rose-600 mx-auto" />
        <h2 className="text-base font-bold text-slate-900">Vault Not Found or Unreachable</h2>
        <p className="text-xs text-slate-500 max-w-sm mx-auto">
          The requested vault address ({vaultAddress || "empty"}) does not exist on-chain or the API service is offline.
        </p>
        <Link href="/dashboard">
          <Button variant="outline" size="sm" className="mt-4">
            <ArrowLeft className="w-4 h-4 mr-2" />
            <span>Return to Dashboard</span>
          </Button>
        </Link>
      </Card>
    );
  }

  const shortAddr = `${vault.vault_address.slice(0, 6)}...${vault.vault_address.slice(-6)}`;
  const shortAuth = `${vault.authority.slice(0, 6)}...${vault.authority.slice(-6)}`;
  const explorerUrl = `https://explorer.solana.com/address/${vault.vault_address}?cluster=devnet`;

  const totalDepositsUsd = vault.total_deposits / 1_000_000;
  const totalSharesUsd = vault.total_shares / 1_000_000;
  const sharePrice = totalSharesUsd > 0 ? totalDepositsUsd / totalSharesUsd : 1.0;

  return (
    <div className="space-y-8">
      {/* Navigation Header & Quick Actions */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-6 border-b border-slate-200">
        <div className="flex items-center gap-3">
          <Link href="/dashboard">
            <Button variant="outline" size="icon" className="h-9 w-9">
              <ArrowLeft className="w-4 h-4" />
            </Button>
          </Link>

          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-extrabold text-slate-900">{vault.name}</h1>
              <Badge variant="outline" className="font-mono text-xs font-bold text-slate-700 bg-slate-50">
                {vault.symbol}
              </Badge>

              {vault.is_paused ? (
                <Badge variant="rose" className="flex items-center gap-1 font-medium">
                  <PauseCircle className="w-3.5 h-3.5" />
                  <span>Paused</span>
                </Badge>
              ) : (
                <Badge variant="success" className="flex items-center gap-1 font-medium">
                  <PlayCircle className="w-3.5 h-3.5" />
                  <span>Live & Active</span>
                </Badge>
              )}
            </div>

            {/* Address bar */}
            <div className="flex items-center gap-3 text-xs text-slate-500 font-mono mt-1">
              <div className="flex items-center gap-1">
                <span>Vault:</span>
                <span className="text-slate-800 font-semibold">{shortAddr}</span>
                <button
                  onClick={copyAddress}
                  className="p-1 hover:text-slate-900 transition-colors"
                  title="Copy address"
                >
                  {copied ? <Check className="w-3 h-3 text-emerald-600" /> : <Copy className="w-3 h-3" />}
                </button>
              </div>

              <a
                href={explorerUrl}
                target="_blank"
                rel="noreferrer"
                className="flex items-center gap-1 text-slate-500 hover:text-cyan-700 transition-colors"
              >
                <span>Explorer</span>
                <ExternalLink className="w-3 h-3" />
              </a>

              <span className="text-slate-300">•</span>
              <div>
                <span>Authority:</span> <span className="text-slate-800 font-semibold">{shortAuth}</span>
              </div>
            </div>
          </div>
        </div>

        {/* Emergency Pause / Unpause Controls */}
        <div className="flex items-center gap-3">
          <Button
            variant={vault.is_paused ? "emerald" : "destructive"}
            size="sm"
            onClick={handleTogglePause}
            className="flex items-center gap-2 font-bold"
          >
            {vault.is_paused ? (
              <>
                <Unlock className="w-4 h-4" />
                <span>Unpause Vault Operations</span>
              </>
            ) : (
              <>
                <Lock className="w-4 h-4" />
                <span>Emergency Pause Vault</span>
              </>
            )}
          </Button>
        </div>
      </div>

      {/* Primary Metrics Grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card className="bg-white">
          <CardContent className="p-5">
            <span className="text-xs text-slate-500 font-medium">Total Deposits</span>
            <div className="mt-2 text-2xl font-bold text-slate-900">
              ${totalDepositsUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </div>
            <span className="text-xs text-emerald-700 font-medium mt-1 block">USDC Collateral</span>
          </CardContent>
        </Card>

        <Card className="bg-white">
          <CardContent className="p-5">
            <span className="text-xs text-slate-500 font-medium">Total Supply Shares</span>
            <div className="mt-2 text-2xl font-bold text-slate-900">
              {totalSharesUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </div>
            <span className="text-xs text-slate-500 mt-1 block">{vault.symbol} Units</span>
          </CardContent>
        </Card>

        <Card className="bg-white">
          <CardContent className="p-5">
            <span className="text-xs text-slate-500 font-medium">NAV per Share</span>
            <div className="mt-2 text-2xl font-bold text-slate-900">
              ${sharePrice.toFixed(4)}
            </div>
            <span className="text-xs text-slate-500 mt-1 block">Epoch NAV</span>
          </CardContent>
        </Card>

        <Card className="bg-white">
          <CardContent className="p-5">
            <span className="text-xs text-slate-500 font-medium">Protection Status</span>
            <div className="mt-2 flex items-center gap-2">
              <span className="inline-block w-2.5 h-2.5 rounded-full bg-emerald-500" />
              <span className="text-lg font-bold text-slate-900">Protected</span>
            </div>
            <span className="text-xs text-slate-500 mt-1 block">Dual-Layer Risk Validation</span>
          </CardContent>
        </Card>
      </div>

      {/* Main Interactive Grid: Deposit/Withdraw & Risk Guard */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Deposit / Withdraw Action Panel */}
        <Card className="bg-white flex flex-col justify-between">
          <CardContent className="p-6">
            <Tabs defaultValue="deposit" value={activeTab} onValueChange={(v) => setActiveTab(v as any)}>
              <TabsList className="grid w-full grid-cols-2 mb-5">
                <TabsTrigger value="deposit" className="flex items-center gap-1.5">
                  <ArrowDownLeft className="w-3.5 h-3.5" />
                  <span>Deposit</span>
                </TabsTrigger>
                <TabsTrigger value="withdraw" className="flex items-center gap-1.5">
                  <ArrowUpRight className="w-3.5 h-3.5" />
                  <span>Withdraw</span>
                </TabsTrigger>
              </TabsList>

              <TabsContent value="deposit">
                <form onSubmit={handleDeposit} className="space-y-4">
                  <div className="space-y-1.5">
                    <div className="flex justify-between text-xs">
                      <label className="font-semibold text-slate-700">Deposit Amount (USDC)</label>
                      <span className="text-slate-400">Collateral currency</span>
                    </div>
                    <div className="relative">
                      <Input
                        type="number"
                        step="any"
                        min="1"
                        required
                        value={depositAmount}
                        onChange={(e) => setDepositAmount(e.target.value)}
                        placeholder="0.00"
                        className="text-sm"
                      />
                    </div>
                  </div>

                  <div className="p-3 rounded-lg bg-slate-50 border border-slate-100 text-xs space-y-1 text-slate-500">
                    <div className="flex justify-between">
                      <span>Est. Shares Received:</span>
                      <span className="text-slate-900 font-semibold">
                        {(parseFloat(depositAmount || "0") / sharePrice).toFixed(2)} {vault.symbol}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span>Protocol Fee:</span>
                      <span className="text-slate-900 font-semibold">0.00%</span>
                    </div>
                  </div>

                  <Button
                    type="submit"
                    variant="emerald"
                    disabled={!connected || isProcessingAction || vault.is_paused}
                    className="w-full font-bold flex items-center justify-center gap-2"
                  >
                    {isProcessingAction ? (
                      <RefreshCw className="w-4 h-4 animate-spin" />
                    ) : (
                      <Coins className="w-4 h-4" />
                    )}
                    <span>
                      {vault.is_paused
                        ? "Vault Operations Paused"
                        : connected
                        ? "Confirm Deposit on Solana"
                        : "Connect Wallet to Deposit"}
                    </span>
                  </Button>
                </form>
              </TabsContent>

              <TabsContent value="withdraw">
                <form onSubmit={handleWithdraw} className="space-y-4">
                  <div className="space-y-1.5">
                    <div className="flex justify-between text-xs">
                      <label className="font-semibold text-slate-700">Shares to Redeem</label>
                      <span className="text-slate-400">{vault.symbol} Shares</span>
                    </div>
                    <div className="relative">
                      <Input
                        type="number"
                        step="any"
                        min="1"
                        required
                        value={withdrawShares}
                        onChange={(e) => setWithdrawShares(e.target.value)}
                        placeholder="0.00"
                        className="text-sm"
                      />
                    </div>
                  </div>

                  <div className="p-3 rounded-lg bg-slate-50 border border-slate-100 text-xs space-y-1 text-slate-500">
                    <div className="flex justify-between">
                      <span>Est. USDC Withdrawn:</span>
                      <span className="text-slate-900 font-semibold">
                        ${(parseFloat(withdrawShares || "0") * sharePrice).toFixed(2)} USDC
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span>Exit Fee:</span>
                      <span className="text-slate-900 font-semibold">0.00%</span>
                    </div>
                  </div>

                  <Button
                    type="submit"
                    variant="default"
                    disabled={!connected || isProcessingAction}
                    className="w-full font-bold flex items-center justify-center gap-2"
                  >
                    {isProcessingAction ? (
                      <RefreshCw className="w-4 h-4 animate-spin" />
                    ) : (
                      <ArrowUpRight className="w-4 h-4" />
                    )}
                    <span>
                      {connected ? "Burn Shares & Redeem Collateral" : "Connect Wallet to Withdraw"}
                    </span>
                  </Button>
                </form>
              </TabsContent>
            </Tabs>

            {actionError && (
              <div className="mt-4 p-3 rounded-lg bg-rose-50 border border-rose-200 text-rose-800 text-xs flex items-center gap-2">
                <AlertTriangle className="w-4 h-4 shrink-0" />
                <span>{actionError}</span>
              </div>
            )}
          </CardContent>
        </Card>

        {/* Risk Meter & Protection Status */}
        <div className="lg:col-span-2 space-y-4">
          {(() => {
            const cashPos = positions.find((p) => p.asset_symbol === "USDC");
            const cashBps = cashPos ? cashPos.current_weight_bps : 0;
            const nonCashPositions = positions.filter((p) => p.asset_symbol !== "USDC");
            const maxPosBps = nonCashPositions.length > 0
              ? Math.max(...nonCashPositions.map((p) => p.current_weight_bps))
              : 0;

            return (
              <RiskMeter
                currentCashBps={cashBps}
                minCashBps={policy?.min_cash_bps || 1000}
                currentPositionBps={maxPosBps}
                maxPositionBps={policy?.max_position_bps || 5000}
              />
            );
          })()}

          <Card className="bg-white">
            <CardContent className="p-5">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 text-xs font-bold text-slate-800">
                  <ShieldCheck className="w-4 h-4 text-emerald-600" />
                  <span>Autonomous Policy Safeguards Active</span>
                </div>
                <span className="text-xs text-slate-400">
                  Updated: {new Date(policy?.updated_at || Date.now()).toLocaleDateString()}
                </span>
              </div>

              <div className="grid grid-cols-3 gap-3 mt-3 text-center text-xs">
                <div className="p-2.5 rounded-lg bg-slate-50 border border-slate-100">
                  <div className="text-xs text-slate-500">Stop Loss Limit</div>
                  <div className="font-bold text-rose-600 mt-0.5">
                    -{(policy ? policy.stop_loss_bps / 100 : 8).toFixed(1)}%
                  </div>
                </div>
                <div className="p-2.5 rounded-lg bg-slate-50 border border-slate-100">
                  <div className="text-xs text-slate-500">Take Profit Target</div>
                  <div className="font-bold text-amber-600 mt-0.5">
                    +{(policy ? policy.take_profit_bps / 100 : 20).toFixed(1)}%
                  </div>
                </div>
                <div className="p-2.5 rounded-lg bg-slate-50 border border-slate-100">
                  <div className="text-xs text-slate-500">Rebalance Drift</div>
                  <div className="font-bold text-slate-800 mt-0.5">
                    ±{(policy ? policy.rebalance_threshold_bps / 100 : 1.5).toFixed(1)}%
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Live Portfolio Table with Pyth Sync */}
      <PositionTable
        positions={positions}
        onRefresh={handleRefreshPyth}
        isRefreshing={isRefreshingPyth}
      />

      {/* Policy Engine Configuration Builder */}
      <PolicyBuilder
        vaultAddress={vault.vault_address}
        initialPolicy={policy}
      />

      {/* Autonomous Event Feed & Backend Decision Pipeline */}
      <div className="space-y-3 pt-2">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-extrabold text-slate-900 flex items-center gap-2">
              <span>Autonomous Decision & Execution Pipeline</span>
              <Badge variant="success" className="text-xs">
                Live Stream
              </Badge>
            </h2>
            <p className="text-xs text-slate-500">
              Audit log of market events, policy rule matching, multi-factor risk validation, and Anchor CPI settlement
            </p>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-5">
            <EventFeed />
          </div>
          <div className="lg:col-span-7">
            <DecisionTimeline />
          </div>
        </div>
      </div>

      {/* Transaction Status Modal */}
      <TransactionStatus status={txStatus} onClose={resetTx} />
    </div>
  );
}
