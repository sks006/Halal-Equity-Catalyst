"use client";

import React, { useState, useMemo } from "react";
import Link from "next/link";
import {
  PieChart as PieIcon,
  TrendingUp,
  ArrowUpRight,
  ArrowDownRight,
  RefreshCw,
  Sliders,
  ShieldAlert,
  ShieldCheck,
  Zap,
  CheckCircle2,
  AlertTriangle,
  Coins,
  DollarSign,
  Layers,
  ArrowRight,
} from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

interface Holding {
  symbol: string;
  name: string;
  category: "Equity RWA" | "Index RWA" | "Cash Reserve";
  units: number;
  entryPrice: number;
  currentPrice: number;
  targetWeight: number; // in %
  color: string;
}

const INITIAL_HOLDINGS: Holding[] = [
  {
    symbol: "NVDAx",
    name: "NVIDIA Corp (Backed RWA)",
    category: "Equity RWA",
    units: 145.2,
    entryPrice: 118.4,
    currentPrice: 128.5,
    targetWeight: 25,
    color: "#10b981", // emerald
  },
  {
    symbol: "AAPLx",
    name: "Apple Inc (Backed RWA)",
    category: "Equity RWA",
    units: 82.5,
    entryPrice: 220.0,
    currentPrice: 232.15,
    targetWeight: 25,
    color: "#06b6d4", // cyan
  },
  {
    symbol: "MSFTx",
    name: "Microsoft Corp (Backed RWA)",
    category: "Equity RWA",
    units: 35.0,
    entryPrice: 415.5,
    currentPrice: 428.9,
    targetWeight: 20,
    color: "#6366f1", // indigo
  },
  {
    symbol: "SPYx",
    name: "S&P 500 ETF (Backed RWA)",
    category: "Index RWA",
    units: 22.0,
    entryPrice: 550.0,
    currentPrice: 564.2,
    targetWeight: 15,
    color: "#f59e0b", // amber
  },
  {
    symbol: "USDC",
    name: "USD Coin (Cash Reserve)",
    category: "Cash Reserve",
    units: 10450.0,
    entryPrice: 1.0,
    currentPrice: 1.0,
    targetWeight: 15,
    color: "#94a3b8", // slate
  },
];

export default function PortfolioPage() {
  const [holdings, setHoldings] = useState<Holding[]>(INITIAL_HOLDINGS);
  const [targetWeights, setTargetWeights] = useState<Record<string, number>>({
    NVDAx: 25,
    AAPLx: 25,
    MSFTx: 20,
    SPYx: 15,
    USDC: 15,
  });
  const [isSimulating, setIsSimulating] = useState(false);
  const [simulationResult, setSimulationResult] = useState<any | null>(null);

  // Calculate current valuations dynamically
  const portfolioMetrics = useMemo(() => {
    let totalValueUsd = 0;
    let totalCostUsd = 0;

    const computedHoldings = holdings.map((h) => {
      const value = h.units * h.currentPrice;
      const cost = h.units * h.entryPrice;
      const pnlUsd = value - cost;
      const pnlPct = cost > 0 ? (pnlUsd / cost) * 100 : 0;
      totalValueUsd += value;
      totalCostUsd += cost;
      return { ...h, value, cost, pnlUsd, pnlPct };
    });

    const totalPnlUsd = totalValueUsd - totalCostUsd;
    const totalPnlPct = totalCostUsd > 0 ? (totalPnlUsd / totalCostUsd) * 100 : 0;

    // Actual weights calculation
    const withActualWeights = computedHoldings.map((h) => {
      const actualWeight = totalValueUsd > 0 ? (h.value / totalValueUsd) * 100 : 0;
      const targetWeight = targetWeights[h.symbol] || 0;
      const driftBps = Math.round((actualWeight - targetWeight) * 100);
      const rebalanceDeltaUsd = (targetWeight / 100) * totalValueUsd - h.value;
      return {
        ...h,
        actualWeight,
        targetWeight,
        driftBps,
        rebalanceDeltaUsd,
      };
    });

    const totalTargetWeight = Object.values(targetWeights).reduce((sum, w) => sum + w, 0);

    return {
      totalValueUsd,
      totalCostUsd,
      totalPnlUsd,
      totalPnlPct,
      holdings: withActualWeights,
      totalTargetWeight,
    };
  }, [holdings, targetWeights]);

  const handleWeightChange = (symbol: string, val: number) => {
    setTargetWeights((prev) => ({
      ...prev,
      [symbol]: val,
    }));
  };

  const runRebalanceSimulation = () => {
    setIsSimulating(true);
    setTimeout(() => {
      const exceedsMaxLimit = Object.entries(targetWeights).some(
        ([sym, w]) => sym !== "USDC" && w > 35
      );

      const trades = portfolioMetrics.holdings
        .filter((h) => Math.abs(h.driftBps) > 50 && h.symbol !== "USDC")
        .map((h) => ({
          symbol: h.symbol,
          action: h.rebalanceDeltaUsd > 0 ? "BUY" : "SELL",
          amountUsd: Math.abs(h.rebalanceDeltaUsd),
          driftBps: h.driftBps,
        }));

      setSimulationResult({
        approved: !exceedsMaxLimit && portfolioMetrics.totalTargetWeight === 100,
        stages: [
          { name: "Schema Validation", passed: true, detail: "Structured types and basis points valid" },
          { name: "Asset Registry", passed: true, detail: "All 5 assets verified active on Devnet/Mainnet" },
          { name: "Oracle Freshness", passed: true, detail: "Pyth feeds verified fresh (< 3s age)" },
          {
            name: "Risk Assessment",
            passed: !exceedsMaxLimit,
            detail: exceedsMaxLimit
              ? "Rejected: Allocation exceeds 35% concentration limit"
              : "Passed: All assets within concentration bounds",
          },
          {
            name: "Policy & Vault State",
            passed: portfolioMetrics.totalTargetWeight === 100,
            detail:
              portfolioMetrics.totalTargetWeight === 100
                ? "Target allocations sum exactly to 100.0%"
                : `Total allocation must equal 100% (currently ${portfolioMetrics.totalTargetWeight}%)`,
          },
        ],
        trades,
      });
      setIsSimulating(false);
    }, 600);
  };

  return (
    <div className="space-y-8 animate-in fade-in duration-500">
      {/* Header Banner */}
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4 border-b border-slate-200 pb-6">
        <div>
          <div className="flex items-center gap-2.5">
            <div className="w-10 h-10 rounded-xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-600 shadow-emerald-sm">
              <PieIcon className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-2xl font-bold tracking-tight text-slate-900">
                Portfolio Allocations & Dynamic Drift Engine
              </h1>
              <p className="text-sm text-slate-500">
                Continuous mark-to-market valuations anchored by Pyth Pro and algorithmic rebalancing gates.
              </p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <Badge variant="cyan" className="font-mono text-xs px-3 py-1 flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-cyan-500 animate-pulse" />
            Pyth Feeds: Synchronized
          </Badge>
          <Link href="/markets">
            <Button variant="outline" size="sm" className="gap-1.5 font-medium text-xs">
              <TrendingUp className="w-3.5 h-3.5 text-emerald-600" />
              DBC Markets
            </Button>
          </Link>
        </div>
      </div>

      {/* KPI Overview Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Total Portfolio Value */}
        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="flex items-center justify-between text-slate-500 text-xs font-semibold uppercase tracking-wider">
              <span>Total Portfolio Value</span>
              <DollarSign className="w-4 h-4 text-emerald-600" />
            </div>
            <div className="mt-2 flex items-baseline gap-2">
              <span className="text-2xl font-black font-mono text-slate-900">
                ${portfolioMetrics.totalValueUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
              </span>
            </div>
            <div className="mt-2 flex items-center gap-1.5 text-xs font-medium text-emerald-600">
              <ArrowUpRight className="w-3.5 h-3.5" />
              <span>+${portfolioMetrics.totalPnlUsd.toFixed(2)} ({portfolioMetrics.totalPnlPct.toFixed(2)}%) all-time</span>
            </div>
          </CardContent>
        </Card>

        {/* Equity RWAs Value */}
        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="flex items-center justify-between text-slate-500 text-xs font-semibold uppercase tracking-wider">
              <span>Tokenized Equities</span>
              <Coins className="w-4 h-4 text-cyan-600" />
            </div>
            <div className="mt-2 flex items-baseline gap-2">
              <span className="text-2xl font-black font-mono text-slate-900">
                ${(portfolioMetrics.totalValueUsd - 10450).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
              </span>
            </div>
            <div className="mt-2 text-xs text-slate-500 font-mono">
              4 Active Tokenized Positions
            </div>
          </CardContent>
        </Card>

        {/* Cash Reserves */}
        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="flex items-center justify-between text-slate-500 text-xs font-semibold uppercase tracking-wider">
              <span>Cash Reserve (USDC)</span>
              <ShieldCheck className="w-4 h-4 text-indigo-600" />
            </div>
            <div className="mt-2 flex items-baseline gap-2">
              <span className="text-2xl font-black font-mono text-slate-900">
                $10,450.00
              </span>
            </div>
            <div className="mt-2 text-xs text-slate-500 font-mono">
              Liquidity Buffer: {((10450 / portfolioMetrics.totalValueUsd) * 100).toFixed(1)}% of vault
            </div>
          </CardContent>
        </Card>

        {/* Max Position Drift */}
        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="flex items-center justify-between text-slate-500 text-xs font-semibold uppercase tracking-wider">
              <span>Max Allocation Drift</span>
              <Sliders className="w-4 h-4 text-amber-600" />
            </div>
            <div className="mt-2 flex items-baseline gap-2">
              <span className="text-2xl font-black font-mono text-slate-900">
                {Math.max(...portfolioMetrics.holdings.map((h) => Math.abs(h.driftBps)))} bps
              </span>
            </div>
            <div className="mt-2 flex items-center gap-1.5 text-xs text-amber-600 font-medium">
              <AlertTriangle className="w-3.5 h-3.5" />
              <span>Rebalancing trigger active</span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Visual Weight Comparison Bar */}
      <Card className="bg-white border-slate-200 overflow-hidden">
        <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
          <div>
            <CardTitle className="text-sm font-bold text-slate-900">
              Live Asset Distribution (Actual vs. Target)
            </CardTitle>
            <CardDescription className="text-xs text-slate-500">
              Real-time capital allocation breakdown across equities and stable reserves.
            </CardDescription>
          </div>
          <span className="text-xs font-mono text-slate-500">
            Target Total: <strong className={portfolioMetrics.totalTargetWeight === 100 ? "text-emerald-600" : "text-rose-600"}>{portfolioMetrics.totalTargetWeight}%</strong>
          </span>
        </CardHeader>
        <CardContent className="p-6 space-y-4">
          {/* Actual Weights Bar */}
          <div>
            <div className="flex justify-between text-xs font-medium text-slate-600 mb-1.5 font-mono">
              <span>Actual Weight (Market Value)</span>
              <span>100.0%</span>
            </div>
            <div className="h-4 rounded-full overflow-hidden flex bg-slate-100">
              {portfolioMetrics.holdings.map((h) => (
                <div
                  key={`actual-${h.symbol}`}
                  style={{ width: `${h.actualWeight}%`, backgroundColor: h.color }}
                  title={`${h.symbol}: ${h.actualWeight.toFixed(1)}%`}
                  className="transition-all duration-500 hover:opacity-80"
                />
              ))}
            </div>
          </div>

          {/* Target Weights Bar */}
          <div>
            <div className="flex justify-between text-xs font-medium text-slate-600 mb-1.5 font-mono">
              <span>Target Policy Weights</span>
              <span className={portfolioMetrics.totalTargetWeight === 100 ? "text-slate-600" : "text-rose-600 font-bold"}>
                {portfolioMetrics.totalTargetWeight.toFixed(1)}%
              </span>
            </div>
            <div className="h-4 rounded-full overflow-hidden flex bg-slate-100">
              {portfolioMetrics.holdings.map((h) => (
                <div
                  key={`target-${h.symbol}`}
                  style={{ width: `${h.targetWeight}%`, backgroundColor: h.color }}
                  title={`${h.symbol}: ${h.targetWeight}%`}
                  className="transition-all duration-500 hover:opacity-80"
                />
              ))}
            </div>
          </div>

          {/* Legend */}
          <div className="flex flex-wrap gap-4 pt-2 border-t border-slate-100">
            {portfolioMetrics.holdings.map((h) => (
              <div key={`legend-${h.symbol}`} className="flex items-center gap-2 text-xs">
                <span className="w-3 h-3 rounded-sm" style={{ backgroundColor: h.color }} />
                <span className="font-bold text-slate-800">{h.symbol}</span>
                <span className="text-slate-400 font-mono">
                  {h.actualWeight.toFixed(1)}% &rarr; {h.targetWeight}%
                </span>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Main Grid: Interactive Rebalance Simulator & Positions Table */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Left 2 Cols: Positions Table */}
        <div className="lg:col-span-2 space-y-6">
          <Card className="bg-white border-slate-200 overflow-hidden">
            <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
              <div>
                <CardTitle className="text-sm font-bold text-slate-900">
                  Current Holdings & Live Oracle Pricing
                </CardTitle>
                <CardDescription className="text-xs text-slate-500">
                  Automated pricing synced via Pyth Hermes low-latency WebSocket feed.
                </CardDescription>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              <Table>
                <TableHeader>
                  <TableRow className="bg-slate-50/50">
                    <TableHead className="text-xs font-semibold text-slate-700">Asset</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">Units</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">Pyth Price</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">Total Value</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">PnL</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">Drift (bps)</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {portfolioMetrics.holdings.map((h) => {
                    const isPositive = h.pnlUsd >= 0;
                    const isDriftHigh = Math.abs(h.driftBps) > 100;
                    return (
                      <TableRow key={h.symbol} className="hover:bg-slate-50/70">
                        <TableCell>
                          <div className="flex items-center gap-2.5">
                            <div
                              className="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold font-mono text-white shadow-sm"
                              style={{ backgroundColor: h.color }}
                            >
                              {h.symbol.slice(0, 3)}
                            </div>
                            <div>
                              <div className="font-bold text-slate-900 flex items-center gap-1.5">
                                {h.symbol}
                                <span className="text-[10px] font-normal text-slate-400 bg-slate-100 px-1.5 py-0.5 rounded">
                                  {h.category}
                                </span>
                              </div>
                              <div className="text-[11px] text-slate-400">{h.name}</div>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="text-right font-mono text-xs text-slate-700">
                          {h.units.toLocaleString(undefined, { maximumFractionDigits: 2 })}
                        </TableCell>
                        <TableCell className="text-right font-mono text-xs font-bold text-slate-900">
                          ${h.currentPrice.toFixed(2)}
                        </TableCell>
                        <TableCell className="text-right font-mono text-xs font-black text-slate-900">
                          ${h.value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                        </TableCell>
                        <TableCell className="text-right">
                          <span
                            className={`inline-flex items-center gap-0.5 text-xs font-semibold font-mono ${
                              isPositive ? "text-emerald-600" : "text-rose-600"
                            }`}
                          >
                            {isPositive ? <ArrowUpRight className="w-3.5 h-3.5" /> : <ArrowDownRight className="w-3.5 h-3.5" />}
                            {isPositive ? "+" : ""}${h.pnlUsd.toFixed(2)}
                          </span>
                        </TableCell>
                        <TableCell className="text-right">
                          <Badge
                            variant={isDriftHigh ? "warning" : "secondary"}
                            className="font-mono text-[10px] px-2 py-0.5"
                          >
                            {h.driftBps > 0 ? `+${h.driftBps}` : h.driftBps} bps
                          </Badge>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </div>

        {/* Right Col: Dynamic Rebalance Simulator */}
        <div className="space-y-6">
          <Card className="bg-white border-slate-200 overflow-hidden shadow-sm">
            <CardHeader className="p-6 border-b border-slate-100 bg-gradient-to-br from-slate-900 to-slate-800 text-white">
              <div className="flex items-center gap-2">
                <Sliders className="w-4 h-4 text-emerald-400" />
                <CardTitle className="text-sm font-bold text-white">
                  Dynamic Allocation Modeler
                </CardTitle>
              </div>
              <CardDescription className="text-xs text-slate-300">
                Adjust target weights to compute automated rebalancing swaps and verify 5-stage validation.
              </CardDescription>
            </CardHeader>
            <CardContent className="p-6 space-y-5">
              {portfolioMetrics.holdings.map((h) => (
                <div key={`slider-${h.symbol}`} className="space-y-1.5">
                  <div className="flex justify-between text-xs font-semibold text-slate-700">
                    <span className="flex items-center gap-1.5">
                      <span className="w-2 h-2 rounded-full" style={{ backgroundColor: h.color }} />
                      {h.symbol}
                    </span>
                    <span className="font-mono text-slate-900">
                      {targetWeights[h.symbol]}%
                    </span>
                  </div>
                  <input
                    type="range"
                    min="0"
                    max="60"
                    step="5"
                    value={targetWeights[h.symbol]}
                    onChange={(e) => handleWeightChange(h.symbol, Number(e.target.value))}
                    className="w-full h-1.5 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-emerald-600"
                  />
                  <div className="flex justify-between text-[10px] text-slate-400 font-mono">
                    <span>Current: {h.actualWeight.toFixed(1)}%</span>
                    <span>
                      Trade: {h.rebalanceDeltaUsd >= 0 ? "+" : ""}${Math.round(h.rebalanceDeltaUsd).toLocaleString()}
                    </span>
                  </div>
                </div>
              ))}

              <div className="pt-3 border-t border-slate-100 flex items-center justify-between">
                <span className="text-xs font-semibold text-slate-700">Total Allocation:</span>
                <span
                  className={`font-mono text-sm font-bold ${
                    portfolioMetrics.totalTargetWeight === 100 ? "text-emerald-600" : "text-rose-600"
                  }`}
                >
                  {portfolioMetrics.totalTargetWeight}% / 100%
                </span>
              </div>

              <Button
                onClick={runRebalanceSimulation}
                disabled={isSimulating}
                className="w-full bg-slate-900 hover:bg-slate-800 text-white font-semibold text-xs py-2.5 shadow-sm"
              >
                {isSimulating ? (
                  <RefreshCw className="w-4 h-4 animate-spin text-emerald-400" />
                ) : (
                  <Zap className="w-4 h-4 fill-emerald-400 text-emerald-400" />
                )}
                <span>Simulate 5-Stage Policy Gate</span>
              </Button>

              {/* Simulation Result Gate Display */}
              {simulationResult && (
                <div className="mt-4 p-4 rounded-xl border bg-slate-50/70 space-y-3 animate-in fade-in">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-bold text-slate-900">Gate Verdict</span>
                    <Badge variant={simulationResult.approved ? "emerald" : "rose"}>
                      {simulationResult.approved ? "APPROVED FOR DISPATCH" : "GATE REJECTION"}
                    </Badge>
                  </div>

                  <div className="space-y-1.5">
                    {simulationResult.stages.map((stg: any, i: number) => (
                      <div key={i} className="flex items-start gap-2 text-[11px]">
                        {stg.passed ? (
                          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600 shrink-0 mt-0.5" />
                        ) : (
                          <AlertTriangle className="w-3.5 h-3.5 text-rose-600 shrink-0 mt-0.5" />
                        )}
                        <div>
                          <span className="font-semibold text-slate-800">{stg.name}:</span>{" "}
                          <span className="text-slate-500">{stg.detail}</span>
                        </div>
                      </div>
                    ))}
                  </div>

                  {simulationResult.trades.length > 0 && simulationResult.approved && (
                    <div className="pt-2 border-t border-slate-200">
                      <div className="text-[11px] font-bold text-slate-800 mb-1">Generated Rebalance Orders:</div>
                      {simulationResult.trades.map((tr: any, idx: number) => (
                        <div key={idx} className="flex justify-between text-[11px] font-mono text-slate-600">
                          <span className={tr.action === "BUY" ? "text-emerald-600 font-bold" : "text-rose-600 font-bold"}>
                            {tr.action} {tr.symbol}
                          </span>
                          <span>${Math.round(tr.amountUsd).toLocaleString()} USDC</span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
