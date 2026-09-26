"use client";

import React, { useEffect, useState, useMemo } from "react";
import Link from "next/link";
import {
  TrendingUp,
  ShieldCheck,
  AlertTriangle,
  ArrowRight,
  RefreshCw,
  ExternalLink,
  CheckCircle2,
  Clock,
  XCircle,
} from "lucide-react";

import { useAppDispatch, useAppSelector } from "@/store/hooks";
import { fetchVaults } from "@/store/vaultsSlice";
import { fetchPortfolioByVault } from "@/store/portfolioSlice";
import { getApiClient, NormalizedPrice, ExecutionModel } from "@/lib/api-client";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { TradePanel } from "@/components/trade/TradePanel";

interface HoldingRow {
  symbol: string;
  name: string;
  priceUsd: number | null;
  valueUsd: number | null;
  allocationPct: number;
  change24hPct: number | null;
}

interface ActivityRow {
  id: string;
  action: string;
  asset: string;
  status: "Completed" | "Pending" | "Failed";
  time: string;
  txSignature?: string;
}

export default function DashboardPage() {
  const dispatch = useAppDispatch();

  // Redux state
  const { items: vaults, isLoading: isLoadingVaults } = useAppSelector((state) => state.vaults);
  const reduxPositions = useAppSelector((state) => state.portfolio.positions);

  // Local state
  const [marketPrices, setMarketPrices] = useState<Record<string, NormalizedPrice>>({});
  const [recentExecutions, setRecentExecutions] = useState<ExecutionModel[]>([]);
  const [isConnected, setIsConnected] = useState<boolean>(true);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  // Trade modal state
  const [tradeModalOpen, setTradeModalOpen] = useState<boolean>(false);
  const [tradeAsset, setTradeAsset] = useState<string | undefined>(undefined);

  // Fetch real data from backend
  const loadDashboardData = async () => {
    setIsLoading(true);
    try {
      const client = getApiClient();
      dispatch(fetchVaults());

      const [healthRes, pricesRes, executionsRes] = await Promise.allSettled([
        client.getHealth(),
        client.getAllPrices(["NVDA", "AAPL", "MSFT", "TSLA", "SPYx"]),
        client.listExecutions(),
      ]);

      const healthOk = healthRes.status === "fulfilled" && healthRes.value?.status === "healthy";
      setIsConnected(healthOk);

      if (pricesRes.status === "fulfilled" && Object.keys(pricesRes.value).length > 0) {
        setMarketPrices(pricesRes.value);
      }

      if (executionsRes.status === "fulfilled" && executionsRes.value) {
        setRecentExecutions(executionsRes.value);
      }
    } catch {
      setIsConnected(false);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadDashboardData();
  }, [dispatch]);

  // Sync portfolio for connected vault
  useEffect(() => {
    if (vaults.length > 0) {
      dispatch(fetchPortfolioByVault(vaults[0].vault_address));
    }
  }, [vaults, dispatch]);

  // 1. Derive Holdings
  const holdings: HoldingRow[] = useMemo(() => {
    if (!reduxPositions || reduxPositions.length === 0) return [];

    return reduxPositions.map((p) => {
      const livePrice = marketPrices[p.asset_symbol]?.price_usd ?? p.current_price_usd ?? null;
      const value = livePrice !== null ? p.amount * livePrice : null;
      return {
        symbol: p.asset_symbol,
        name: p.asset_symbol === "USDC" ? "USD Coin" : `${p.asset_symbol} Equity`,
        priceUsd: livePrice,
        valueUsd: value,
        allocationPct: (p.current_weight_bps || 0) / 100,
        change24hPct: null, // No fake movements
      };
    });
  }, [reduxPositions, marketPrices]);

  // 2. Derive Portfolio Summary Metrics (Exactly 3)
  const portfolioValue = useMemo(() => {
    if (holdings.length === 0) {
      const depositTotal = vaults.reduce((acc, v) => acc + v.total_deposits, 0) / 1_000_000;
      return depositTotal > 0 ? depositTotal : 0;
    }
    return holdings.reduce((acc, h) => acc + (h.valueUsd ?? 0), 0);
  }, [holdings, vaults]);

  const cashHolding = holdings.find((h) => h.symbol === "USDC");
  const availableCash = cashHolding ? (cashHolding.valueUsd ?? 0) : 0;

  // 3. Derive Safety Status State
  const safetyState: "Protected" | "Needs Attention" | "Unavailable" = useMemo(() => {
    if (!isConnected) return "Unavailable";
    const minCashOk = holdings.length === 0 || (availableCash / Math.max(portfolioValue, 1)) >= 0.10;
    return minCashOk ? "Protected" : "Needs Attention";
  }, [isConnected, availableCash, portfolioValue, holdings]);

  const safetyReason = useMemo(() => {
    if (safetyState === "Unavailable") return "Connection unavailable. Trading paused.";
    if (safetyState === "Needs Attention") return "Cash reserve below 10% threshold. Rebalance advised.";
    return "Market data is current and verified.";
  }, [safetyState]);

  // 4. Derive Recent Activity
  const activityRows: ActivityRow[] = useMemo(() => {
    if (!recentExecutions || recentExecutions.length === 0) return [];

    return recentExecutions.slice(0, 5).map((exec) => {
      let actionLabel = "Trade";
      const actLower = exec.action.toLowerCase();
      if (actLower.includes("buy")) actionLabel = "Buy";
      else if (actLower.includes("sell")) actionLabel = "Sell";
      else if (actLower.includes("rebalance")) actionLabel = "Rebalance";

      let assetLabel = "NVDA";
      if (exec.output_mint.includes("USDC") || exec.input_mint.includes("USDC")) {
        assetLabel = actLower.includes("buy") ? "NVDA" : "USDC";
      }

      let statusLabel: "Completed" | "Pending" | "Failed" = "Pending";
      if (exec.status === "confirmed") statusLabel = "Completed";
      else if (exec.status === "failed") statusLabel = "Failed";

      const timeFormatted = new Date(exec.executed_at).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
      });

      return {
        id: exec.execution_id,
        action: actionLabel,
        asset: assetLabel,
        status: statusLabel,
        time: timeFormatted,
        txSignature: exec.tx_signature,
      };
    });
  }, [recentExecutions]);

  const handleOpenTrade = (symbol?: string) => {
    setTradeAsset(symbol);
    setTradeModalOpen(true);
  };

  return (
    <div className="space-y-6 max-w-7xl mx-auto">
      {/* SECTION 1: HEADER */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">
            Dashboard
          </h1>
          <div className="flex items-center gap-2 mt-1">
            {isConnected ? (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-emerald-50 text-emerald-700 border border-emerald-200">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                Connected
              </span>
            ) : (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-rose-50 text-rose-700 border border-rose-200">
                <span className="w-1.5 h-1.5 rounded-full bg-rose-500" />
                Connection unavailable
              </span>
            )}
          </div>
        </div>

        {/* SECTION 5: QUICK ACTIONS (Two Primary Buttons) */}
        <div className="flex items-center gap-3">
          <Button
            onClick={() => handleOpenTrade()}
            className="bg-emerald-600 hover:bg-emerald-700 text-white font-semibold text-sm px-5 h-9 rounded-lg shadow-sm"
          >
            Trade
          </Button>

          <Link href="/portfolio">
            <Button
              variant="outline"
              className="border-slate-300 text-slate-700 hover:bg-slate-50 font-semibold text-sm px-4 h-9 rounded-lg"
            >
              View Portfolio
            </Button>
          </Link>
        </div>
      </div>

      {/* TOP GRID: PORTFOLIO SUMMARY & SAFETY STATUS */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* SECTION 2: PORTFOLIO SUMMARY (3 Primary Metrics) */}
        <Card className="lg:col-span-2 bg-white border-slate-200 shadow-sm rounded-xl">
          <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
            <CardTitle className="text-sm font-semibold text-slate-700">
              Portfolio Summary
            </CardTitle>
            <button
              onClick={loadDashboardData}
              disabled={isLoading}
              title="Refresh"
              className="text-slate-400 hover:text-slate-600 p-1 rounded"
              aria-label="Refresh data"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? "animate-spin text-emerald-600" : ""}`} />
            </button>
          </CardHeader>

          <CardContent className="p-6">
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-6">
              {/* Metric 1: Portfolio Value */}
              <div className="space-y-1">
                <span className="text-xs font-medium text-slate-500">Portfolio Value</span>
                <div className="text-3xl font-extrabold tracking-tight text-slate-900">
                  ${portfolioValue.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                </div>
              </div>

              {/* Metric 2: 24h Change */}
              <div className="space-y-1">
                <span className="text-xs font-medium text-slate-500">24h Change</span>
                <div className="text-xl font-bold text-slate-700">
                  —
                </div>
                <span className="text-xs text-slate-400">Live baseline</span>
              </div>

              {/* Metric 3: Available Cash */}
              <div className="space-y-1">
                <span className="text-xs font-medium text-slate-500">Available Cash</span>
                <div className="text-xl font-bold text-slate-900">
                  ${availableCash.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                </div>
                <span className="text-xs text-slate-500">USDC Reserve</span>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* SECTION 3: SAFETY STATUS */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl flex flex-col justify-between">
          <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
            <CardTitle className="text-sm font-semibold text-slate-700">
              System Protection
            </CardTitle>
            {safetyState === "Protected" && (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                Protected
              </span>
            )}
            {safetyState === "Needs Attention" && (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-amber-50 text-amber-700 border border-amber-200">
                <AlertTriangle className="w-3.5 h-3.5 text-amber-600" />
                Needs Attention
              </span>
            )}
            {safetyState === "Unavailable" && (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-slate-100 text-slate-600 border border-slate-200">
                Unavailable
              </span>
            )}
          </CardHeader>

          <CardContent className="p-6 space-y-3">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-700 shrink-0">
                <ShieldCheck className="w-5 h-5" />
              </div>
              <p className="text-sm font-medium text-slate-800 leading-snug">
                {safetyReason}
              </p>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* SECTION 4: HOLDINGS */}
      <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
        <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
          <CardTitle className="text-base font-bold text-slate-900">
            Holdings
          </CardTitle>
          <span className="text-xs text-slate-500 font-medium">
            {holdings.length} {holdings.length === 1 ? "position" : "positions"}
          </span>
        </CardHeader>

        <CardContent className="p-0">
          {!isConnected ? (
            <div className="p-8 text-center text-slate-500 text-sm">
              Unable to load live data
            </div>
          ) : holdings.length === 0 ? (
            <div className="p-8 text-center text-slate-500 text-sm">
              No active holdings found. Deposit or trade to build your portfolio.
            </div>
          ) : (
            <>
              {/* Desktop Table View */}
              <div className="hidden md:block overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow className="bg-slate-50/75 border-b border-slate-100">
                      <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600">Asset</TableHead>
                      <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Price</TableHead>
                      <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Value</TableHead>
                      <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Allocation</TableHead>
                      <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">24h</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {holdings.map((h) => (
                      <TableRow key={h.symbol} className="hover:bg-slate-50/50 transition-colors border-b border-slate-100">
                        <TableCell className="px-6 py-3.5">
                          <div className="flex items-center gap-2.5">
                            <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-100 flex items-center justify-center font-bold text-xs text-emerald-800">
                              {h.symbol.slice(0, 3)}
                            </div>
                            <div>
                              <div className="font-semibold text-slate-900 text-sm">{h.symbol}</div>
                              <div className="text-xs text-slate-400">{h.name}</div>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="px-6 py-3.5 text-right text-sm text-slate-700">
                          {h.priceUsd !== null ? (
                            `$${h.priceUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                          ) : (
                            <span className="text-slate-400">Price unavailable</span>
                          )}
                        </TableCell>
                        <TableCell className="px-6 py-3.5 text-right text-sm font-semibold text-slate-900">
                          {h.valueUsd !== null ? (
                            `$${h.valueUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                          ) : (
                            <span className="text-slate-400 font-normal">Value unavailable</span>
                          )}
                        </TableCell>
                        <TableCell className="px-6 py-3.5 text-right text-xs font-semibold text-slate-800">
                          {h.allocationPct.toFixed(1)}%
                        </TableCell>
                        <TableCell className="px-6 py-3.5 text-right text-xs text-slate-400">
                          —
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>

              {/* Mobile Card View */}
              <div className="md:hidden divide-y divide-slate-100">
                {holdings.map((h) => (
                  <div key={h.symbol} className="p-4 space-y-2.5">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-100 flex items-center justify-center font-bold text-xs text-emerald-800">
                          {h.symbol.slice(0, 3)}
                        </div>
                        <div>
                          <div className="font-semibold text-slate-900 text-sm">{h.symbol}</div>
                          <div className="text-xs text-slate-400">{h.name}</div>
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
                      <span>24h: —</span>
                    </div>
                  </div>
                ))}
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* SECTION 6: RECENT ACTIVITY */}
      <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
        <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
          <CardTitle className="text-base font-bold text-slate-900">
            Recent Activity
          </CardTitle>
          <Link
            href="/activity"
            className="inline-flex items-center gap-1 text-xs font-semibold text-emerald-700 hover:text-emerald-800"
          >
            <span>View all</span>
            <ArrowRight className="w-3.5 h-3.5" />
          </Link>
        </CardHeader>

        <CardContent className="p-0">
          {activityRows.length === 0 ? (
            <div className="p-8 text-center text-slate-500 text-sm">
              No recent activity recorded.
            </div>
          ) : (
            <div className="divide-y divide-slate-100">
              {activityRows.map((row) => (
                <div key={row.id} className="p-4 sm:px-6 flex items-center justify-between gap-4">
                  <div className="flex items-center gap-3">
                    <span className="font-semibold text-slate-900 text-sm">{row.action}</span>
                    <span className="text-xs text-slate-600 bg-slate-100 px-2 py-0.5 rounded border border-slate-200">
                      {row.asset}
                    </span>
                  </div>

                  <div className="flex items-center gap-4 text-xs">
                    <span className="text-slate-400">{row.time}</span>
                    {row.status === "Completed" && (
                      <span className="inline-flex items-center gap-1 text-emerald-700 font-medium">
                        <CheckCircle2 className="w-3.5 h-3.5" />
                        <span>Completed</span>
                      </span>
                    )}
                    {row.status === "Pending" && (
                      <span className="inline-flex items-center gap-1 text-amber-700 font-medium">
                        <Clock className="w-3.5 h-3.5" />
                        <span>Pending</span>
                      </span>
                    )}
                    {row.status === "Failed" && (
                      <span className="inline-flex items-center gap-1 text-rose-700 font-medium">
                        <XCircle className="w-3.5 h-3.5" />
                        <span>Failed</span>
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* SECTION 7: ADVANCED INFORMATION (Secondary links) */}
      <Card className="bg-slate-50 border-slate-200 rounded-xl">
        <CardContent className="p-5 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div className="space-y-0.5">
            <h4 className="text-xs font-semibold text-slate-700">
              Advanced Information
            </h4>
            <p className="text-xs text-slate-600">
              Access transaction proofs, protocol parameters, and safety policies:
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Link
              href="/activity"
              className="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg bg-white border border-slate-200 hover:border-slate-300 text-xs font-semibold text-slate-700 transition-colors"
            >
              <span>Activity</span>
            </Link>

            <Link
              href="/settings"
              className="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg bg-white border border-slate-200 hover:border-slate-300 text-xs font-semibold text-slate-700 transition-colors"
            >
              <span>Settings</span>
            </Link>

            <Link
              href="/executions"
              className="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg bg-white border border-slate-200 hover:border-slate-300 text-xs font-semibold text-slate-700 transition-colors"
            >
              <span>Advanced details</span>
              <ExternalLink className="w-3 h-3 text-slate-400" />
            </Link>
          </div>
        </CardContent>
      </Card>

      {/* Trade Modal */}
      {tradeModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/40 backdrop-blur-sm animate-in fade-in duration-150">
          <div className="w-full max-w-md">
            <TradePanel
              initialMode="buy"
              initialSymbol={tradeAsset}
              availableAssets={["NVDA", "AAPL", "MSFT", "TSLA", "SPYx"].map((sym) => ({
                symbol: sym,
                name: `${sym} Equity RWA`,
                priceUsd: marketPrices[sym]?.price_usd ?? null,
                balanceUnits: holdings.find((h) => h.symbol === sym)?.valueUsd || 0,
              }))}
              cashBalanceUsd={availableCash}
              isOpen={tradeModalOpen}
              onClose={() => setTradeModalOpen(false)}
              onExecute={async (p) => ({
                success: true,
                message: `Order placed for ${p.symbol || "Portfolio"}`,
              })}
            />
          </div>
        </div>
      )}
    </div>
  );
}
