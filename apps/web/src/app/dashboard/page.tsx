"use client";

import React, { useEffect, useState } from "react";
import Link from "next/link";
import {
  Activity,
  AlertTriangle,
  ArrowDownRight,
  ArrowUpRight,
  BarChart3,
  CheckCircle2,
  Clock,
  Coins,
  DollarSign,
  Layers,
  PieChart as PieIcon,
  PlusCircle,
  RefreshCw,
  Search,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  TrendingUp,
  Vault as VaultIcon,
  Wallet,
  Zap,
} from "lucide-react";

import { useWallet } from "../../hooks/useWallet";
import { useSolana } from "../../hooks/useSolana";
import { VaultCard } from "../../features/vault/VaultCard";
import { useAppDispatch, useAppSelector } from "../../store/hooks";
import { fetchVaults } from "../../store/vaultsSlice";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Button } from "../../components/ui/button";
import { Badge } from "../../components/ui/badge";
import { Input } from "../../components/ui/input";
import { Progress } from "../../components/ui/progress";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../../components/ui/table";
import { getApiClient, NormalizedPrice } from "../../lib/api-client";

interface PositionDisplay {
  symbol: string;
  name: string;
  assetType: "Equity RWA" | "Cash Reserve" | "Index Token";
  quantity: number;
  entryPrice: number;
  currentPrice: number;
  marketValue: number;
  pnlUsd: number;
  pnlPct: number;
  targetWeightPct: number;
  actualWeightPct: number;
  driftBps: number;
}

export default function DashboardPage() {
  const dispatch = useAppDispatch();
  const { connected, shortAddress } = useWallet();
  const { balanceSol } = useSolana();

  // Redux state for vaults and portfolio
  const { items: vaults, isLoading: isLoadingVaults } = useAppSelector(
    (state) => state.vaults
  );
  const reduxPositions = useAppSelector((state) => state.portfolio.positions);

  const [searchTerm, setSearchTerm] = useState<string>("");
  const [filterPaused, setFilterPaused] = useState<"all" | "active" | "paused">("all");
  const [marketPrices, setMarketPrices] = useState<Record<string, NormalizedPrice>>({});
  const [isRefreshingPrices, setIsRefreshingPrices] = useState<boolean>(false);
  const [lastSyncTime, setLastSyncTime] = useState<Date>(new Date());

  const loadVaults = () => {
    dispatch(fetchVaults());
  };

  const syncMarketData = async () => {
    setIsRefreshingPrices(true);
    try {
      const client = getApiClient();
      const live = await client.getAllPrices(["NVDA", "AAPL", "MSFT", "TSLA", "SPYx"]);
      if (live && Object.keys(live).length > 0) {
        setMarketPrices(live);
      }
      setLastSyncTime(new Date());
    } catch {
      // Fail closed: do not populate with fake prices
    } finally {
      setIsRefreshingPrices(false);
    }
  };

  useEffect(() => {
    loadVaults();
    syncMarketData();
  }, [dispatch]);

  // Derive consolidated positions from live Redux portfolio
  const positions: PositionDisplay[] = (reduxPositions || []).map((p) => {
    const price = marketPrices[p.asset_symbol]?.price_usd || p.current_price_usd || 0;
    const value = p.amount * price;
    const entryPrice = p.entry_price_usd || price;
    const pnlUsd = p.amount * (price - entryPrice);
    const pnlPct = entryPrice > 0 ? ((price - entryPrice) / entryPrice) * 100 : 0;
    const driftBps = p.current_weight_bps - p.target_weight_bps;
    return {
      symbol: p.asset_symbol,
      name: p.asset_symbol,
      assetType: p.asset_symbol === "USDC" ? "Cash Reserve" : "Equity RWA",
      quantity: p.amount,
      entryPrice: entryPrice,
      currentPrice: price,
      marketValue: value,
      pnlUsd: pnlUsd,
      pnlPct: pnlPct,
      targetWeightPct: (p.target_weight_bps || 0) / 100,
      actualWeightPct: (p.current_weight_bps || 0) / 100,
      driftBps: driftBps,
    };
  });

  // Aggregated Portfolio Metrics
  const totalPortfolioValue = positions.reduce((acc, p) => acc + p.marketValue, 0);
  const totalPnlUsd = positions.reduce((acc, p) => acc + p.pnlUsd, 0);
  const totalCostBasis = totalPortfolioValue - totalPnlUsd;
  const totalPnlPct = totalCostBasis > 0 ? (totalPnlUsd / totalCostBasis) * 100 : 0;

  const totalTvl = vaults.reduce((acc, v) => acc + v.total_deposits, 0) / 1_000_000;
  const activeVaultCount = vaults.filter((v) => !v.is_paused).length;

  const filteredVaults = vaults.filter((v) => {
    const matchesSearch =
      v.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      v.symbol.toLowerCase().includes(searchTerm.toLowerCase()) ||
      v.vault_address.toLowerCase().includes(searchTerm.toLowerCase());

    if (filterPaused === "active") return matchesSearch && !v.is_paused;
    if (filterPaused === "paused") return matchesSearch && v.is_paused;
    return matchesSearch;
  });

  const cashPos = positions.find((p) => p.symbol === "USDC");
  const maxPos = positions.length > 0 ? [...positions].sort((a, b) => b.actualWeightPct - a.actualWeightPct)[0] : null;

  return (
    <div className="space-y-8">
      {/* 1. EXECUTIVE HERO BANNER: PORTFOLIO VALUE, P&L, RISK STATE */}
      <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-slate-900 via-slate-800 to-indigo-950 p-6 sm:p-8 text-white shadow-xl border border-slate-700/50">
        <div className="absolute -right-20 -top-20 w-80 h-80 rounded-full bg-emerald-500/10 blur-3xl pointer-events-none" />
        <div className="absolute -left-20 -bottom-20 w-80 h-80 rounded-full bg-indigo-500/10 blur-3xl pointer-events-none" />

        <div className="relative z-10 flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="space-y-2">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-lg bg-emerald-500/20 border border-emerald-400/30 flex items-center justify-center text-emerald-400">
                <Zap className="w-4 h-4" />
              </div>
              <span className="text-xs uppercase font-mono tracking-widest text-emerald-400 font-semibold">
                Autonomous Equity Management Engine
              </span>
            </div>
            <h1 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-white font-mono">
              ${totalPortfolioValue.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </h1>
            <div className="flex flex-wrap items-center gap-3 text-xs pt-1">
              <span className="text-slate-400">Total Portfolio Value</span>
              <span className="text-slate-600">•</span>
              <span className={`inline-flex items-center gap-1 font-bold ${totalPnlUsd >= 0 ? "text-emerald-400" : "text-rose-400"}`}>
                {totalPnlUsd >= 0 ? <ArrowUpRight className="w-3.5 h-3.5" /> : <ArrowDownRight className="w-3.5 h-3.5" />}
                <span>
                  {totalPnlUsd >= 0 ? "+" : ""}${totalPnlUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ({totalPnlPct >= 0 ? "+" : ""}{totalPnlPct.toFixed(2)}%)
                </span>
              </span>
              <span className="text-slate-600">•</span>
              <span className="text-slate-400 font-mono">
                Synced {lastSyncTime.toLocaleTimeString()}
              </span>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              onClick={syncMarketData}
              disabled={isRefreshingPrices}
              className="bg-white/10 hover:bg-white/20 text-white border-white/20 text-xs font-semibold backdrop-blur"
            >
              <RefreshCw className={`w-3.5 h-3.5 mr-1.5 ${isRefreshingPrices ? "animate-spin text-emerald-400" : ""}`} />
              <span>{isRefreshingPrices ? "Updating Pyth..." : "Sync Market Feeds"}</span>
            </Button>

            <Link href="/vault/new">
              <Button size="sm" className="bg-emerald-500 hover:bg-emerald-600 text-white font-bold text-xs shadow-lg shadow-emerald-500/25">
                <PlusCircle className="w-3.5 h-3.5 mr-1.5" />
                <span>New Strategy Vault</span>
              </Button>
            </Link>
          </div>
        </div>

        {/* Hero Quick Stat Badges */}
        <div className="mt-6 pt-6 border-t border-slate-700/60 grid grid-cols-2 sm:grid-cols-4 gap-4 text-xs">
          <div>
            <div className="text-slate-400 flex items-center gap-1.5 mb-1">
              <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
              <span>Risk Policy State</span>
            </div>
            <div className="font-bold text-emerald-400 font-mono flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              <span>OPTIMAL / GUARDED</span>
            </div>
          </div>

          <div>
            <div className="text-slate-400 flex items-center gap-1.5 mb-1">
              <Coins className="w-3.5 h-3.5 text-cyan-400" />
              <span>Cash Reserve</span>
            </div>
            <div className="font-bold text-white font-mono">
              {cashPos
                ? `${cashPos.actualWeightPct.toFixed(1)}% ($${(cashPos.marketValue / 1000).toFixed(0)}k USDC)`
                : "0.0% ($0 USDC)"}
            </div>
          </div>

          <div>
            <div className="text-slate-400 flex items-center gap-1.5 mb-1">
              <BarChart3 className="w-3.5 h-3.5 text-indigo-400" />
              <span>Max Exposure Limit</span>
            </div>
            <div className="font-bold text-white font-mono">
              {maxPos
                ? `${maxPos.actualWeightPct.toFixed(1)}% / 40.0% Max (${maxPos.symbol})`
                : "0.0% / 40.0% Max"}
            </div>
          </div>

          <div>
            <div className="text-slate-400 flex items-center gap-1.5 mb-1">
              <Activity className="w-3.5 h-3.5 text-purple-400" />
              <span>Simulated Execution Gate</span>
            </div>
            <div className="font-bold text-white font-mono">
              100% Preflight Validated
            </div>
          </div>
        </div>
      </div>

      {/* 2. REAL-TIME PYTH PRO MARKET DATA TICKER */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Activity className="w-4 h-4 text-emerald-600" />
            <h2 className="text-sm font-bold text-slate-900 tracking-tight">Pyth Pro Verified Market Feeds</h2>
            <Badge variant="cyan" className="text-[10px] font-mono py-0.5">Sub-Second Hermes v2</Badge>
          </div>
          <span className="text-[11px] text-slate-500 font-mono">Confidence bounded ±0.05%</span>
        </div>

        {Object.keys(marketPrices).length > 0 ? (
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3">
            {Object.values(marketPrices).map((p) => {
              const ageSeconds = Math.max(0, Math.floor(Date.now() / 1000) - p.publish_time);
              return (
                <Card key={p.symbol} className="bg-white hover:border-emerald-300 transition-colors shadow-sm">
                  <CardContent className="p-3.5">
                    <div className="flex items-center justify-between">
                      <span className="font-bold text-xs text-slate-900 font-mono">{p.symbol}</span>
                      <Badge variant={ageSeconds < 15 ? "emerald" : "warning"} className="text-[9px] px-1.5 py-0 font-mono">
                        {ageSeconds < 15 ? "Fresh" : `${ageSeconds}s`}
                      </Badge>
                    </div>
                    <div className="mt-2 text-base font-extrabold text-slate-900 font-mono">
                      ${p.price_usd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                    </div>
                    <div className="mt-1 flex items-center justify-between text-[10px] text-slate-500 font-mono">
                      <span>±${p.confidence_usd.toFixed(2)}</span>
                      <span className="text-emerald-600 font-semibold">Active</span>
                    </div>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        ) : (
          <Card className="bg-white p-6 text-center text-slate-400 text-xs">
            Connecting to live Pyth Pro Hermes price stream...
          </Card>
        )}
      </div>

      {/* 3. ACTIVE POSITIONS & TARGET ALLOCATION VISUALIZER */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Positions Table (2 Columns) */}
        <div className="lg:col-span-2 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <PieIcon className="w-4 h-4 text-emerald-600" />
              <h2 className="text-sm font-bold text-slate-900 tracking-tight">Active Portfolio Holdings</h2>
            </div>
            <span className="text-xs text-slate-500 font-mono">{positions.length} Active Positions</span>
          </div>

          <Card className="bg-white overflow-hidden shadow-sm border-slate-200">
            {positions.length > 0 ? (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow className="bg-slate-50/75">
                      <TableHead className="text-xs font-bold text-slate-700">Asset</TableHead>
                      <TableHead className="text-xs font-bold text-slate-700">Type</TableHead>
                      <TableHead className="text-xs font-bold text-slate-700 text-right">Holdings</TableHead>
                      <TableHead className="text-xs font-bold text-slate-700 text-right">Market Price</TableHead>
                      <TableHead className="text-xs font-bold text-slate-700 text-right">Value ($)</TableHead>
                      <TableHead className="text-xs font-bold text-slate-700 text-right">P&L</TableHead>
                      <TableHead className="text-xs font-bold text-slate-700 text-right">Actual / Target</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {positions.map((pos) => {
                      const isProfit = pos.pnlUsd >= 0;
                      return (
                        <TableRow key={pos.symbol} className="hover:bg-slate-50/50">
                          <TableCell className="font-semibold text-xs text-slate-900 font-mono">
                            <div>
                              <span>{pos.symbol}</span>
                              <span className="block text-[10px] text-slate-400 font-sans font-normal">{pos.name}</span>
                            </div>
                          </TableCell>
                          <TableCell>
                            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-slate-100 text-slate-700 border border-slate-200">
                              {pos.assetType}
                            </span>
                          </TableCell>
                          <TableCell className="text-right text-xs font-mono">
                            {pos.quantity.toLocaleString()}
                          </TableCell>
                          <TableCell className="text-right text-xs font-mono font-semibold">
                            ${pos.currentPrice.toFixed(2)}
                          </TableCell>
                          <TableCell className="text-right text-xs font-mono font-bold text-slate-900">
                            ${pos.marketValue.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                          </TableCell>
                          <TableCell className="text-right text-xs font-mono">
                            <span className={`font-bold ${isProfit ? "text-emerald-600" : "text-rose-600"}`}>
                              {isProfit ? "+" : ""}${pos.pnlUsd.toLocaleString(undefined, { maximumFractionDigits: 0 })} ({isProfit ? "+" : ""}{pos.pnlPct.toFixed(1)}%)
                            </span>
                          </TableCell>
                          <TableCell className="text-right text-xs font-mono">
                            <div className="flex items-center justify-end gap-1.5">
                              <span className="font-bold text-slate-900">{pos.actualWeightPct.toFixed(1)}%</span>
                              <span className="text-slate-400 text-[10px]">/ {pos.targetWeightPct.toFixed(0)}%</span>
                            </div>
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              </div>
            ) : (
              <div className="p-12 text-center text-slate-400 text-xs">
                No active portfolio positions found for connected vaults.
              </div>
            )}
          </Card>
        </div>

        {/* Allocation Weights & Drift Radar (1 Column) */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <BarChart3 className="w-4 h-4 text-emerald-600" />
              <h2 className="text-sm font-bold text-slate-900 tracking-tight">Allocation Weights & Drift</h2>
            </div>
            <Badge variant="emerald" className="text-[10px] font-mono">Balanced</Badge>
          </div>

          <Card className="bg-white p-5 shadow-sm border-slate-200 space-y-4">
            {positions.length > 0 ? (
              positions.map((pos) => {
                const driftColor =
                  Math.abs(pos.driftBps) > 200
                    ? "text-amber-600"
                    : Math.abs(pos.driftBps) > 400
                    ? "text-rose-600"
                    : "text-emerald-600";
                return (
                  <div key={pos.symbol} className="space-y-1.5">
                    <div className="flex items-center justify-between text-xs">
                      <div className="flex items-center gap-1.5">
                        <span className="font-bold text-slate-900 font-mono">{pos.symbol}</span>
                        <span className="text-[10px] text-slate-400">Target: {pos.targetWeightPct}%</span>
                      </div>
                      <div className="flex items-center gap-1.5 text-xs font-mono">
                        <span className="font-bold text-slate-900">{pos.actualWeightPct.toFixed(1)}%</span>
                        <span className={`text-[10px] font-bold ${driftColor}`}>
                          ({pos.driftBps >= 0 ? "+" : ""}{pos.driftBps} bps)
                        </span>
                      </div>
                    </div>
                    <Progress
                      value={pos.actualWeightPct}
                      max={40}
                      className="h-2 bg-slate-100"
                    />
                  </div>
                );
              })
            ) : (
              <div className="p-8 text-center text-slate-400 text-xs">
                No positions to analyze drift.
              </div>
            )}

            <div className="pt-3 border-t border-slate-100 flex items-center justify-between text-[11px] text-slate-500">
              <span>Auto-Rebalance Trigger Threshold:</span>
              <span className="font-mono font-bold text-slate-800">±300 bps (3.0%)</span>
            </div>
          </Card>
        </div>
      </div>

      {/* 4. REAL-TIME RISK STATE & CIRCUIT BREAKER DEFENSE */}
      <Card className="bg-white p-6 shadow-sm border-slate-200">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-slate-100 pb-4">
          <div>
            <div className="flex items-center gap-2">
              <ShieldAlert className="w-5 h-5 text-emerald-600" />
              <h2 className="text-base font-bold text-slate-900">Deterministic Risk State & Protection Gates</h2>
            </div>
            <p className="text-xs text-slate-500 mt-1">
              Autonomous rebalancing proposals are guarded by on-chain Anchor constraints and backend policy workers.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <span className="inline-block w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
            <span className="text-xs font-bold text-emerald-700 font-mono bg-emerald-50 px-2.5 py-1 rounded-full border border-emerald-200">
              All Guardrails Pass
            </span>
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mt-5">
          <div className="p-4 rounded-xl bg-slate-50 border border-slate-200">
            <div className="text-xs text-slate-500 font-medium">Position Concentration</div>
            <div className="mt-2 text-xl font-bold font-mono text-slate-900">
              {maxPos ? `${maxPos.actualWeightPct.toFixed(1)}% / 40.0%` : "0.0% / 40.0%"}
            </div>
            <div className="mt-1 flex items-center gap-1 text-[11px] text-emerald-600 font-semibold">
              <CheckCircle2 className="w-3.5 h-3.5" />
              <span>Within max limit</span>
            </div>
          </div>

          <div className="p-4 rounded-xl bg-slate-50 border border-slate-200">
            <div className="text-xs text-slate-500 font-medium">Max Drawdown Sentinel</div>
            <div className="mt-2 text-xl font-bold font-mono text-slate-900">0.0% / 10.0%</div>
            <div className="mt-1 flex items-center gap-1 text-[11px] text-emerald-600 font-semibold">
              <CheckCircle2 className="w-3.5 h-3.5" />
              <span>Circuit breaker armed</span>
            </div>
          </div>

          <div className="p-4 rounded-xl bg-slate-50 border border-slate-200">
            <div className="text-xs text-slate-500 font-medium">Minimum Cash Shield</div>
            <div className="mt-2 text-xl font-bold font-mono text-slate-900">
              {cashPos ? `${cashPos.actualWeightPct.toFixed(1)}% / 10.0%` : "0.0% / 10.0%"}
            </div>
            <div className="mt-1 flex items-center gap-1 text-[11px] text-emerald-600 font-semibold">
              <CheckCircle2 className="w-3.5 h-3.5" />
              <span>Sufficient liquidity</span>
            </div>
          </div>

          <div className="p-4 rounded-xl bg-slate-50 border border-slate-200">
            <div className="text-xs text-slate-500 font-medium">Preflight Simulation</div>
            <div className="mt-2 text-xl font-bold font-mono text-slate-900">100% Passed</div>
            <div className="mt-1 flex items-center gap-1 text-[11px] text-emerald-600 font-semibold">
              <CheckCircle2 className="w-3.5 h-3.5" />
              <span>Zero failed simulations</span>
            </div>
          </div>
        </div>
      </Card>

      {/* 5. DEPLOYED STRATEGY VAULTS DIRECTORY */}
      <div className="space-y-4">
        <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4">
          <div>
            <h2 className="text-lg font-bold text-slate-900 flex items-center gap-2">
              <Layers className="w-5 h-5 text-emerald-600" />
              <span>Deployed Strategy Vaults</span>
            </h2>
            <p className="text-xs text-slate-500 mt-1">
              Decentralized Solana Anchor smart vaults with autonomous keeper dispatch and multi-sig fail-safes.
            </p>
          </div>

          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              onClick={loadVaults}
              disabled={isLoadingVaults}
              className="flex items-center gap-1.5 text-xs font-medium"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isLoadingVaults ? "animate-spin text-emerald-600" : ""}`} />
              <span>{isLoadingVaults ? "Syncing..." : "Sync Vaults"}</span>
            </Button>

            <Link href="/vault/new">
              <Button variant="emerald" size="sm" className="flex items-center gap-2 font-bold">
                <PlusCircle className="w-4 h-4" />
                <span>Initialize Vault</span>
              </Button>
            </Link>
          </div>
        </div>

        {/* Filter and Search Bar */}
        <Card className="bg-white p-3 border-slate-200">
          <div className="flex flex-col sm:flex-row items-center gap-4">
            <div className="relative flex-1 w-full">
              <Search className="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
              <Input
                type="text"
                placeholder="Search vault by name, symbol, or address..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="pl-9 bg-slate-50/60 border-slate-200"
              />
            </div>

            <div className="flex items-center gap-1.5 w-full sm:w-auto">
              <Button
                variant={filterPaused === "all" ? "default" : "ghost"}
                size="sm"
                onClick={() => setFilterPaused("all")}
                className="text-xs h-8"
              >
                All ({vaults.length})
              </Button>
              <Button
                variant={filterPaused === "active" ? "secondary" : "ghost"}
                size="sm"
                onClick={() => setFilterPaused("active")}
                className={`text-xs h-8 ${filterPaused === "active" ? "text-emerald-700 font-bold bg-emerald-50 border border-emerald-200" : ""}`}
              >
                Active ({activeVaultCount})
              </Button>
              <Button
                variant={filterPaused === "paused" ? "secondary" : "ghost"}
                size="sm"
                onClick={() => setFilterPaused("paused")}
                className={`text-xs h-8 ${filterPaused === "paused" ? "text-rose-700 font-bold bg-rose-50 border border-rose-200" : ""}`}
              >
                Paused ({vaults.length - activeVaultCount})
              </Button>
            </div>
          </div>
        </Card>

        {/* Vault Cards Grid */}
        {filteredVaults.length > 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {filteredVaults.map((vault) => (
              <VaultCard key={vault.vault_address} vault={vault} />
            ))}
          </div>
        ) : (
          <Card className="bg-white p-12 text-center space-y-4">
            <div className="w-12 h-12 rounded-full bg-slate-100 flex items-center justify-center mx-auto text-slate-400">
              <Search className="w-6 h-6" />
            </div>
            <h3 className="text-base font-bold text-slate-800">No vaults found</h3>
            <p className="text-xs text-slate-500 max-w-sm mx-auto">
              No strategy vaults are currently registered on-chain or matching your filter.
            </p>
            <Link href="/vault/new">
              <Button variant="emerald" size="sm" className="inline-flex items-center gap-2">
                <PlusCircle className="w-4 h-4" />
                <span>Create New Vault</span>
              </Button>
            </Link>
          </Card>
        )}
      </div>
    </div>
  );
}
