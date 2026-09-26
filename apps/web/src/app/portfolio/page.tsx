"use client";

import React, { useState, useEffect, useMemo } from "react";
import {
  PieChart as PieIcon,
  RefreshCw,
  ChevronDown,
  ChevronUp,
  AlertCircle,
  CheckCircle2,
  Sliders,
  ArrowRight,
  ShieldCheck,
  X,
} from "lucide-react";

import { useAppDispatch, useAppSelector } from "@/store/hooks";
import { fetchVaults } from "@/store/vaultsSlice";
import { fetchPortfolioByVault } from "@/store/portfolioSlice";
import { getApiClient, NormalizedPrice } from "@/lib/api-client";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

interface PortfolioHolding {
  symbol: string;
  name: string;
  units: number;
  priceUsd: number | null;
  valueUsd: number | null;
  allocationPct: number;
  targetWeightPct: number;
  driftBps: number;
  pnlUsd: number | null;
  pnlPct: number | null;
  color: string;
}

const PALETTE = ["#10b981", "#06b6d4", "#6366f1", "#f59e0b", "#94a3b8"];

export default function PortfolioPage() {
  const dispatch = useAppDispatch();

  // Redux state
  const { items: vaults, isLoading: isLoadingVaults } = useAppSelector((state) => state.vaults);
  const reduxPositions = useAppSelector((state) => state.portfolio.positions);

  // Local state
  const [marketPrices, setMarketPrices] = useState<Record<string, NormalizedPrice>>({});
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  // Collapsible Advanced Section
  const [showAdvancedDetails, setShowAdvancedDetails] = useState<boolean>(false);

  // Rebalance Review Modal
  const [rebalanceReviewOpen, setRebalanceReviewOpen] = useState<boolean>(false);
  const [isSubmittingRebalance, setIsSubmittingRebalance] = useState<boolean>(false);
  const [rebalanceSuccessMessage, setRebalanceSuccessMessage] = useState<string | null>(null);

  // Default target weights
  const [targetWeights, setTargetWeights] = useState<Record<string, number>>({
    NVDA: 25,
    AAPL: 25,
    MSFT: 20,
    TSLA: 15,
    USDC: 15,
  });

  const loadData = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const client = getApiClient();
      dispatch(fetchVaults());

      const prices = await client.getAllPrices(["NVDA", "AAPL", "MSFT", "TSLA", "SPYx"]);
      if (prices && Object.keys(prices).length > 0) {
        setMarketPrices(prices);
      }
    } catch {
      setError("Unable to load live portfolio data");
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, [dispatch]);

  // Sync positions for active vault
  useEffect(() => {
    if (vaults.length > 0) {
      dispatch(fetchPortfolioByVault(vaults[0].vault_address));
    }
  }, [vaults, dispatch]);

  // 1. Derive Holdings from Real Backend Data
  const holdings: PortfolioHolding[] = useMemo(() => {
    if (!reduxPositions || reduxPositions.length === 0) return [];

    return reduxPositions.map((pos, idx) => {
      const livePrice = marketPrices[pos.asset_symbol]?.price_usd ?? pos.current_price_usd ?? null;
      const value = livePrice !== null ? pos.amount * livePrice : null;
      const targetPct = targetWeights[pos.asset_symbol] || (pos.target_weight_bps || 0) / 100;
      const currentPct = (pos.current_weight_bps || 0) / 100;
      const driftBps = pos.current_weight_bps - targetPct * 100;

      const hasEntryPrice = pos.entry_price_usd > 0 && livePrice !== null && livePrice > 0;
      const pnlUsd = hasEntryPrice ? pos.amount * (livePrice - pos.entry_price_usd) : null;
      const pnlPct = hasEntryPrice ? ((livePrice - pos.entry_price_usd) / pos.entry_price_usd) * 100 : null;

      return {
        symbol: pos.asset_symbol,
        name: pos.asset_symbol === "USDC" ? "USD Coin (Cash Reserve)" : `${pos.asset_symbol} Equity`,
        units: pos.amount,
        priceUsd: livePrice,
        valueUsd: value,
        allocationPct: currentPct,
        targetWeightPct: targetPct,
        driftBps: Math.round(driftBps),
        pnlUsd,
        pnlPct,
        color: PALETTE[idx % PALETTE.length],
      };
    });
  }, [reduxPositions, marketPrices, targetWeights]);

  // 2. Summary Metrics (Section 2)
  const portfolioValue = useMemo(() => {
    if (holdings.length === 0) {
      const depositTotal = vaults.reduce((acc, v) => acc + v.total_deposits, 0) / 1_000_000;
      return depositTotal > 0 ? depositTotal : 0;
    }
    return holdings.reduce((acc, h) => acc + (h.valueUsd ?? 0), 0);
  }, [holdings, vaults]);

  const cashHolding = holdings.find((h) => h.symbol === "USDC");
  const cashUsd = cashHolding ? (cashHolding.valueUsd ?? 0) : 0;
  const investedUsd = Math.max(0, portfolioValue - cashUsd);

  // Derive total return if real entry prices exist
  const totalReturnMetrics = useMemo(() => {
    const validPositions = holdings.filter((h) => h.pnlUsd !== null);
    if (validPositions.length === 0) return { usd: null, pct: null };
    const totalPnlUsd = validPositions.reduce((acc, h) => acc + h.pnlUsd!, 0);
    const costBasis = portfolioValue - totalPnlUsd;
    const totalPnlPct = costBasis > 0 ? (totalPnlUsd / costBasis) * 100 : null;
    return { usd: totalPnlUsd, pct: totalPnlPct };
  }, [holdings, portfolioValue]);

  // 3. Rebalance Analysis (Section 5)
  const maxDriftHolding = useMemo(() => {
    if (holdings.length === 0) return null;
    return [...holdings].sort((a, b) => Math.abs(b.driftBps) - Math.abs(a.driftBps))[0];
  }, [holdings]);

  const rebalanceExplanation = useMemo(() => {
    if (!maxDriftHolding || Math.abs(maxDriftHolding.driftBps) < 100) {
      return "All holdings are currently balanced within target allocation thresholds.";
    }
    const driftPct = (maxDriftHolding.driftBps / 100).toFixed(1);
    if (maxDriftHolding.driftBps > 0) {
      return `${maxDriftHolding.symbol} is ${driftPct}% above its target allocation.`;
    } else {
      return `${maxDriftHolding.symbol} is ${Math.abs(Number(driftPct))}% below its target allocation.`;
    }
  }, [maxDriftHolding]);

  // 4. Rebalance Review Trades Calculation (Section 6)
  const rebalanceTrades = useMemo(() => {
    if (portfolioValue <= 0 || holdings.length === 0) return [];

    return holdings.map((h) => {
      const targetVal = (portfolioValue * h.targetWeightPct) / 100;
      const diffVal = targetVal - (h.valueUsd ?? 0);
      const diffPct = h.targetWeightPct - h.allocationPct;

      let estimatedAction = "Hold";
      if (diffVal > 10) {
        estimatedAction = `Buy $${diffVal.toFixed(0)} ${h.symbol}`;
      } else if (diffVal < -10) {
        estimatedAction = `Sell $${Math.abs(diffVal).toFixed(0)} ${h.symbol}`;
      }

      return {
        symbol: h.symbol,
        currentPct: h.allocationPct,
        targetPct: h.targetWeightPct,
        changePct: diffPct,
        estimatedTrade: estimatedAction,
      };
    });
  }, [holdings, portfolioValue]);

  const handleReviewTradesSubmit = async () => {
    setIsSubmittingRebalance(true);
    try {
      const client = getApiClient();
      await client.createEvent({
        vault_address: vaults[0]?.vault_address || "DefaultVault",
        event_type: "REBALANCE_TRIGGER",
        source: "frontend_portfolio_review",
        payload: { target_weights: targetWeights },
      });

      setRebalanceSuccessMessage("Rebalance order successfully created and queued for execution.");
      setTimeout(() => {
        setIsSubmittingRebalance(false);
        setRebalanceReviewOpen(false);
      }, 1500);
    } catch {
      setIsSubmittingRebalance(false);
    }
  };

  return (
    <div className="space-y-6 max-w-7xl mx-auto">
      {/* SECTION 1: HEADER */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900 flex items-center gap-2">
            <PieIcon className="w-6 h-6 text-emerald-600" />
            <span>Portfolio</span>
          </h1>
          <p className="text-sm text-slate-500 mt-0.5">
            Understand your holdings, asset distribution, and target allocation.
          </p>
        </div>

        <Button
          variant="outline"
          size="sm"
          onClick={loadData}
          disabled={isLoading}
          className="border-slate-300 text-slate-700 text-xs font-semibold self-start sm:self-auto"
        >
          <RefreshCw className={`w-3.5 h-3.5 mr-1.5 ${isLoading ? "animate-spin text-emerald-600" : ""}`} />
          <span>Refresh</span>
        </Button>
      </div>

      {/* SECTION 2: SUMMARY (4 Primary Metrics) */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Metric 1: Portfolio Value */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardContent className="p-5 space-y-1">
            <span className="text-xs font-medium text-slate-500">Portfolio Value</span>
            <div className="text-2xl font-extrabold tracking-tight text-slate-900">
              ${portfolioValue.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </div>
          </CardContent>
        </Card>

        {/* Metric 2: Invested */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardContent className="p-5 space-y-1">
            <span className="text-xs font-medium text-slate-500">Invested</span>
            <div className="text-2xl font-bold tracking-tight text-slate-900">
              ${investedUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </div>
          </CardContent>
        </Card>

        {/* Metric 3: Cash */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardContent className="p-5 space-y-1">
            <span className="text-xs font-medium text-slate-500">Cash</span>
            <div className="text-2xl font-bold tracking-tight text-slate-900">
              ${cashUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </div>
          </CardContent>
        </Card>

        {/* Metric 4: Total Return */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardContent className="p-5 space-y-1">
            <span className="text-xs font-medium text-slate-500">Total Return</span>
            <div className="text-2xl font-bold tracking-tight">
              {totalReturnMetrics.usd !== null && totalReturnMetrics.pct !== null ? (
                <span className={totalReturnMetrics.usd >= 0 ? "text-emerald-700" : "text-rose-700"}>
                  {totalReturnMetrics.usd >= 0 ? "+" : ""}${totalReturnMetrics.usd.toFixed(2)} ({totalReturnMetrics.pct.toFixed(2)}%)
                </span>
              ) : (
                <span className="text-slate-600 font-normal text-xl">—</span>
              )}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* SECTION 3: ALLOCATION VISUALIZATION */}
      <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
        <CardHeader className="px-6 py-4 border-b border-slate-100">
          <CardTitle className="text-base font-bold text-slate-900">
            Allocation
          </CardTitle>
        </CardHeader>

        <CardContent className="p-6 space-y-5">
          {/* Segmented Bar Visualization */}
          {holdings.length > 0 ? (
            <>
              <div className="h-3 w-full rounded-full overflow-hidden flex bg-slate-100">
                {holdings.map((h) => (
                  <div
                    key={h.symbol}
                    style={{
                      width: `${Math.max(h.allocationPct, 1)}%`,
                      backgroundColor: h.color,
                    }}
                    title={`${h.symbol}: ${h.allocationPct.toFixed(1)}%`}
                    className="h-full transition-all"
                  />
                ))}
              </div>

              {/* Simple Allocation Breakdown: Asset, Allocation, Value */}
              <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3">
                {holdings.map((h) => (
                  <div
                    key={h.symbol}
                    className="p-3 rounded-lg bg-slate-50 border border-slate-100 space-y-1"
                  >
                    <div className="flex items-center gap-2">
                      <span
                        className="w-2.5 h-2.5 rounded-full shrink-0"
                        style={{ backgroundColor: h.color }}
                      />
                      <span className="font-bold text-slate-900 text-xs">{h.symbol}</span>
                    </div>

                    <div className="text-sm font-bold text-slate-900">
                      {h.allocationPct.toFixed(1)}%
                    </div>

                    <div className="text-xs text-slate-500">
                      {h.valueUsd !== null ? `$${h.valueUsd.toLocaleString(undefined, { maximumFractionDigits: 0 })}` : "Value unavailable"}
                    </div>
                  </div>
                ))}
              </div>
            </>
          ) : (
            <div className="p-6 text-center text-slate-400 text-sm">
              No allocation data available.
            </div>
          )}
        </CardContent>
      </Card>

      {/* SECTION 4: HOLDINGS TABLE */}
      <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
        <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
          <CardTitle className="text-base font-bold text-slate-900">
            Holdings
          </CardTitle>
          <span className="text-xs text-slate-500 font-medium">
            {holdings.length} {holdings.length === 1 ? "asset" : "assets"}
          </span>
        </CardHeader>

        <CardContent className="p-0">
          {error ? (
            <div className="p-8 text-center text-slate-500 text-sm">
              {error}
            </div>
          ) : holdings.length === 0 ? (
            <div className="p-8 text-center text-slate-500 text-sm">
              No holdings found. Deposit funds or trade to build your portfolio.
            </div>
          ) : (
            <>
              {/* Desktop Table View */}
              <div className="hidden md:block overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow className="bg-slate-50/75 border-b border-slate-100">
                      <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600">Asset</TableHead>
                      <TableHead className="px-4 py-3 text-xs font-semibold text-slate-600 text-right">Units</TableHead>
                      <TableHead className="px-4 py-3 text-xs font-semibold text-slate-600 text-right">Price</TableHead>
                      <TableHead className="px-4 py-3 text-xs font-semibold text-slate-600 text-right">Value</TableHead>
                      <TableHead className="px-4 py-3 text-xs font-semibold text-slate-600 text-right">Allocation</TableHead>
                      <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">P/L</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {holdings.map((h) => (
                      <TableRow key={h.symbol} className="hover:bg-slate-50/50 transition-colors border-b border-slate-100">
                        <TableCell className="px-6 py-3.5">
                          <div className="flex items-center gap-2.5">
                            <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center font-bold text-xs text-emerald-800">
                              {h.symbol.slice(0, 3)}
                            </div>
                            <div>
                              <div className="font-semibold text-slate-900 text-sm">{h.symbol}</div>
                              <div className="text-xs text-slate-400">{h.name}</div>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="px-4 py-3.5 text-right text-sm text-slate-700">
                          {h.units.toLocaleString(undefined, { maximumFractionDigits: 4 })}
                        </TableCell>
                        <TableCell className="px-4 py-3.5 text-right text-sm text-slate-700">
                          {h.priceUsd !== null ? (
                            `$${h.priceUsd.toFixed(2)}`
                          ) : (
                            <span className="text-slate-400">Price unavailable</span>
                          )}
                        </TableCell>
                        <TableCell className="px-4 py-3.5 text-right text-sm font-semibold text-slate-900">
                          {h.valueUsd !== null ? (
                            `$${h.valueUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                          ) : (
                            <span className="text-slate-400 font-normal">Value unavailable</span>
                          )}
                        </TableCell>
                        <TableCell className="px-4 py-3.5 text-right text-xs font-semibold text-slate-800">
                          {h.allocationPct.toFixed(1)}%
                        </TableCell>
                        <TableCell className="px-6 py-3.5 text-right text-xs">
                          {h.pnlUsd !== null && h.pnlPct !== null ? (
                            <span className={h.pnlUsd >= 0 ? "text-emerald-700 font-semibold" : "text-rose-700 font-semibold"}>
                              {h.pnlUsd >= 0 ? "+" : ""}${h.pnlUsd.toFixed(2)} ({h.pnlPct.toFixed(1)}%)
                            </span>
                          ) : (
                            <span className="text-slate-400">—</span>
                          )}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>

              {/* Mobile Card View */}
              <div className="md:hidden divide-y divide-slate-100">
                {holdings.map((h) => (
                  <div key={h.symbol} className="p-4 space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2.5">
                        <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center font-bold text-xs text-emerald-800">
                          {h.symbol.slice(0, 3)}
                        </div>
                        <div>
                          <div className="font-semibold text-slate-900 text-sm">{h.symbol}</div>
                          <div className="text-xs text-slate-400">{h.units.toFixed(2)} units</div>
                        </div>
                      </div>
                      <div className="text-right">
                        <div className="font-bold text-slate-900 text-sm">
                          {h.valueUsd !== null ? (
                            `$${h.valueUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                          ) : (
                            <span className="text-slate-400 font-normal">Value unavailable</span>
                          )}
                        </div>
                        <div className="text-xs text-slate-500 font-sans">
                          {h.allocationPct.toFixed(1)}%
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center justify-between text-xs text-slate-500 pt-1">
                      <span>Price: {h.priceUsd !== null ? `$${h.priceUsd.toFixed(2)}` : "Price unavailable"}</span>
                      <span>
                        P/L:{" "}
                        {h.pnlUsd !== null ? (
                          <strong className={h.pnlUsd >= 0 ? "text-emerald-700" : "text-rose-700"}>
                            {h.pnlUsd >= 0 ? "+" : ""}${h.pnlUsd.toFixed(2)}
                          </strong>
                        ) : (
                          "—"
                        )}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* SECTION 5: REBALANCE PORTFOLIO */}
      <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
        <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <CardTitle className="text-base font-bold text-slate-900">
            Rebalance portfolio
          </CardTitle>
          <Button
            onClick={() => {
              setRebalanceSuccessMessage(null);
              setRebalanceReviewOpen(true);
            }}
            className="bg-emerald-600 hover:bg-emerald-700 text-white font-semibold text-xs px-4 h-9 rounded-lg shadow-sm"
          >
            Review Rebalance
          </Button>
        </CardHeader>

        <CardContent className="p-6">
          <div className="flex items-start gap-3">
            <div className="w-10 h-10 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-700 shrink-0">
              <ShieldCheck className="w-5 h-5" />
            </div>
            <div className="space-y-1">
              <p className="text-sm font-semibold text-slate-800">
                {rebalanceExplanation}
              </p>
              <p className="text-xs text-slate-500">
                Rebalancing realigns portfolio weights to target values while maintaining minimum required cash reserves.
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* SECTION 7: ADVANCED DETAILS (Collapsible Section) */}
      <Card className="bg-slate-50 border-slate-200 rounded-xl overflow-hidden">
        <div className="px-6 py-3.5 border-b border-slate-200 flex items-center justify-between">
          <button
            type="button"
            onClick={() => setShowAdvancedDetails(!showAdvancedDetails)}
            className="flex items-center justify-between w-full text-xs font-semibold text-slate-600 hover:text-slate-900 transition-colors"
          >
            <span>Advanced details & risk thresholds</span>
            {showAdvancedDetails ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
          </button>
        </div>

        {showAdvancedDetails && (
          <CardContent className="p-6 space-y-4 text-xs">
            {/* Drift BPS breakdown */}
            <div className="space-y-1.5">
              <span className="font-bold text-slate-700 block">Position Drift (BPS):</span>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                {holdings.map((h) => (
                  <div key={h.symbol} className="p-2.5 rounded bg-white border border-slate-200">
                    <span className="text-slate-500 block text-xs">{h.symbol}</span>
                    <span className={`font-bold ${Math.abs(h.driftBps) > 200 ? "text-amber-700" : "text-emerald-700"}`}>
                      {h.driftBps >= 0 ? "+" : ""}{h.driftBps} bps
                    </span>
                  </div>
                ))}
              </div>
            </div>

            {/* Risk Thresholds & Policy Values */}
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-2">
              <div className="p-3 bg-white rounded border border-slate-200">
                <span className="text-slate-500 block text-xs">Rebalance Threshold:</span>
                <span className="font-bold text-slate-900">300 bps (3.0%)</span>
              </div>

              <div className="p-3 bg-white rounded border border-slate-200">
                <span className="text-slate-500 block text-xs">Minimum Cash Reserve:</span>
                <span className="font-bold text-slate-900">1000 bps (10.0%)</span>
              </div>

              <div className="p-3 bg-white rounded border border-slate-200">
                <span className="text-slate-500 block text-xs">Max Position Exposure:</span>
                <span className="font-bold text-slate-900">4000 bps (40.0%)</span>
              </div>
            </div>

            {/* Technical Calculations */}
            <div className="p-3 bg-white rounded border border-slate-200 text-slate-600 text-xs">
              <span className="font-bold text-slate-700 block mb-1">Technical Calculations:</span>
              <div>Formula: <code className="text-slate-800">|actual_weight_bps - target_weight_bps| &gt; threshold_bps</code></div>
              <div className="mt-0.5">Execution route: Jupiter DEX direct CPI via Solana Anchor Program.</div>
            </div>
          </CardContent>
        )}
      </Card>

      {/* SECTION 6: REBALANCE REVIEW MODAL */}
      {rebalanceReviewOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/40 backdrop-blur-sm animate-in fade-in duration-150">
          <Card className="bg-white border-slate-200 shadow-xl rounded-xl max-w-lg w-full overflow-hidden">
            <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
              <CardTitle className="text-base font-bold text-slate-900">
                Rebalance Review
              </CardTitle>
              <button
                onClick={() => setRebalanceReviewOpen(false)}
                className="p-1 rounded-lg text-slate-400 hover:text-slate-600 hover:bg-slate-100 transition-colors"
                aria-label="Close"
              >
                <X className="w-5 h-5" />
              </button>
            </CardHeader>

            <CardContent className="p-6 space-y-4">
              {rebalanceSuccessMessage ? (
                <div className="p-4 bg-emerald-50 text-emerald-800 rounded-lg flex items-center gap-2 border border-emerald-200">
                  <CheckCircle2 className="w-5 h-5 text-emerald-600 shrink-0" />
                  <span className="text-sm font-semibold">{rebalanceSuccessMessage}</span>
                </div>
              ) : (
                <>
                  <p className="text-xs text-slate-500">
                    Review proposed rebalancing actions below before submitting. No orders will execute without your confirmation.
                  </p>

                  <div className="border border-slate-200 rounded-lg overflow-hidden">
                    <Table>
                      <TableHeader>
                        <TableRow className="bg-slate-50 text-xs">
                          <TableHead className="px-4 py-2 font-semibold">Asset</TableHead>
                          <TableHead className="px-3 py-2 text-right font-semibold">Current</TableHead>
                          <TableHead className="px-3 py-2 text-right font-semibold">Target</TableHead>
                          <TableHead className="px-3 py-2 text-right font-semibold">Change</TableHead>
                          <TableHead className="px-4 py-2 text-right font-semibold">Estimated Trade</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {rebalanceTrades.map((t) => (
                          <TableRow key={t.symbol} className="text-xs border-b border-slate-100">
                            <TableCell className="px-4 py-2.5 font-bold text-slate-900">
                              {t.symbol}
                            </TableCell>
                            <TableCell className="px-3 py-2.5 text-right text-slate-700">
                              {t.currentPct.toFixed(1)}%
                            </TableCell>
                            <TableCell className="px-3 py-2.5 text-right text-slate-700">
                              {t.targetPct.toFixed(1)}%
                            </TableCell>
                            <TableCell className="px-3 py-2.5 text-right">
                              <span className={t.changePct >= 0 ? "text-emerald-700" : "text-rose-700"}>
                                {t.changePct >= 0 ? "+" : ""}{t.changePct.toFixed(1)}%
                              </span>
                            </TableCell>
                            <TableCell className="px-4 py-2.5 text-right font-semibold text-slate-900">
                              {t.estimatedTrade}
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </div>

                  <div className="pt-2">
                    <Button
                      onClick={handleReviewTradesSubmit}
                      disabled={isSubmittingRebalance || rebalanceTrades.length === 0}
                      className="w-full h-11 text-sm font-semibold bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg shadow-sm"
                    >
                      {isSubmittingRebalance ? "Submitting..." : "Review Trades"}
                    </Button>
                  </div>
                </>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
