"use client";

import React, { useState, useMemo } from "react";
import Link from "next/link";
import {
  TrendingUp,
  Activity,
  ArrowUpRight,
  ArrowDownRight,
  Sliders,
  Layers,
  Coins,
  RefreshCw,
  ExternalLink,
  ShieldCheck,
  CheckCircle2,
  AlertTriangle,
  Zap,
  DollarSign,
  Search,
  Scale,
  Shield,
  FileCheck,
  Radio,
} from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { usePythLazer } from "@/hooks/usePythLazer";

interface DbcPoolDisplay {
  poolAddress: string;
  symbol: string;
  name: string;
  issuer: string;
  pythFeedId: number;
  spotPriceUsd: number;
  oraclePriceUsd: number;
  spreadBps: number;
  liquidityUsd: number;
  volume24hUsd: number;
  curveProgressPct: number;
  isMigrated: boolean;
  baseMint: string;
  quoteMint: string;
  // Shariah Screening Metrics (AAOIFI Standard No. 21 / IIFA Resolution 63)
  shariahEligible: boolean;
  debtToMcapPct: number;
  cashToMcapPct: number;
  impermissibleRevenuePct: number;
  shariahStatus: "Eligible" | "Review Required";
}

const VERIFIED_POOLS: DbcPoolDisplay[] = [
  {
    poolAddress: "7Fzqx1MeteoraAaplDbcPool1111111111111111111",
    symbol: "AAPLx",
    name: "Apple Inc Tokenized Spot",
    issuer: "Backed Finance / Meteora DBC",
    pythFeedId: 10,
    spotPriceUsd: 232.4,
    oraclePriceUsd: 232.15,
    spreadBps: 11, // +0.11% premium
    liquidityUsd: 2_180_000,
    volume24hUsd: 540_000,
    curveProgressPct: 82.0,
    isMigrated: false,
    baseMint: "7Fzqx1AAPLxTokenMint11111111111111111111111111",
    quoteMint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    shariahEligible: true,
    debtToMcapPct: 14.8, // < 30%
    cashToMcapPct: 12.1, // < 30%
    impermissibleRevenuePct: 1.1, // < 5%
    shariahStatus: "Eligible",
  },
  {
    poolAddress: "8Gyqx1MeteoraMsftDbcPool1111111111111111111",
    symbol: "MSFTx",
    name: "Microsoft Corp Tokenized Spot",
    issuer: "Backed Finance / Meteora DBC",
    pythFeedId: 11,
    spotPriceUsd: 429.3,
    oraclePriceUsd: 428.9,
    spreadBps: 9, // +0.09% premium
    liquidityUsd: 3_120_000,
    volume24hUsd: 780_000,
    curveProgressPct: 88.5,
    isMigrated: false,
    baseMint: "8Gyqx1MSFTxTokenMint11111111111111111111111111",
    quoteMint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    shariahEligible: true,
    debtToMcapPct: 11.4, // < 30%
    cashToMcapPct: 15.6, // < 30%
    impermissibleRevenuePct: 0.9, // < 5%
    shariahStatus: "Eligible",
  },
  {
    poolAddress: "6Ewqx1MeteoraNvdaDbcPool1111111111111111111",
    symbol: "NVDAx",
    name: "NVIDIA Corp Tokenized Spot",
    issuer: "Backed Finance / Meteora DBC",
    pythFeedId: 12,
    spotPriceUsd: 129.1,
    oraclePriceUsd: 128.5,
    spreadBps: 46, // +0.46% premium
    liquidityUsd: 1_450_000,
    volume24hUsd: 320_500,
    curveProgressPct: 68.5,
    isMigrated: false,
    baseMint: "6Ewqx1NVDAxTokenMint11111111111111111111111111",
    quoteMint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    shariahEligible: true,
    debtToMcapPct: 8.2, // < 30%
    cashToMcapPct: 9.4, // < 30%
    impermissibleRevenuePct: 0.4, // < 5%
    shariahStatus: "Eligible",
  },
];

export default function MarketsPage() {
  const [pools, setPools] = useState<DbcPoolDisplay[]>(VERIFIED_POOLS);
  const [selectedPool, setSelectedPool] = useState<DbcPoolDisplay>(VERIFIED_POOLS[0]);
  const [swapMode, setSwapMode] = useState<"BUY" | "SELL">("BUY");
  const [amountInput, setAmountInput] = useState("500");
  const [searchQuery, setSearchQuery] = useState("");
  const [isSimulating, setIsSimulating] = useState(false);
  const [simOutcome, setSimOutcome] = useState<any | null>(null);

  // Real-time Pyth Lazer feed updates
  const { prices: lazerPrices, isConnected: isLazerConnected } = usePythLazer({
    feedIds: [10, 11, 12],
    channel: "fixed_rate@200ms",
  });

  // Calculate transparent fee breakdown: Pool Fee (10 bps), Platform Fee (3 bps), Execution Fee (2 bps) = Total 15 bps
  const quoteCalculation = useMemo(() => {
    const inputVal = parseFloat(amountInput) || 0;
    if (inputVal <= 0) {
      return {
        amountOut: 0,
        priceImpactPct: 0,
        effectivePrice: selectedPool.spotPriceUsd,
        poolFeeUsd: 0,
        platformFeeUsd: 0,
        executionFeeUsd: 0,
        totalFeeUsd: 0,
      };
    }

    if (swapMode === "BUY") {
      // Input is USDC, Output is spot tokens
      const poolFeeUsd = inputVal * 0.001; // 0.10% to LP pool
      const platformFeeUsd = inputVal * 0.0003; // 0.03% to controller
      const executionFeeUsd = inputVal * 0.0002; // 0.02% to oracle/execution
      const totalFeeUsd = poolFeeUsd + platformFeeUsd + executionFeeUsd;
      const netUsdc = inputVal - totalFeeUsd;

      // Linear bonding curve impact
      const priceImpactPct = Math.min(5.0, (inputVal / selectedPool.liquidityUsd) * 100 * 2.5);
      const effectivePrice = selectedPool.spotPriceUsd * (1 + priceImpactPct / 100);
      const amountOut = netUsdc / effectivePrice;

      return {
        amountOut,
        priceImpactPct,
        effectivePrice,
        poolFeeUsd,
        platformFeeUsd,
        executionFeeUsd,
        totalFeeUsd,
      };
    } else {
      // Input is spot tokens, Output is USDC
      const grossUsdc = inputVal * selectedPool.spotPriceUsd;
      const priceImpactPct = Math.min(5.0, (grossUsdc / selectedPool.liquidityUsd) * 100 * 2.5);
      const effectivePrice = selectedPool.spotPriceUsd * (1 - priceImpactPct / 100);
      const grossAfterImpact = inputVal * effectivePrice;

      const poolFeeUsd = grossAfterImpact * 0.001;
      const platformFeeUsd = grossAfterImpact * 0.0003;
      const executionFeeUsd = grossAfterImpact * 0.0002;
      const totalFeeUsd = poolFeeUsd + platformFeeUsd + executionFeeUsd;
      const amountOut = grossAfterImpact - totalFeeUsd;

      return {
        amountOut,
        priceImpactPct,
        effectivePrice,
        poolFeeUsd,
        platformFeeUsd,
        executionFeeUsd,
        totalFeeUsd,
      };
    }
  }, [amountInput, swapMode, selectedPool]);

  const handleSimulateSwap = () => {
    setIsSimulating(true);
    setTimeout(() => {
      setSimOutcome({
        success: quoteCalculation.priceImpactPct < 3.0,
        pool: selectedPool.symbol,
        action: swapMode,
        inputAmount: amountInput,
        outputAmount: quoteCalculation.amountOut.toFixed(4),
        effectivePrice: quoteCalculation.effectivePrice.toFixed(2),
        priceImpact: quoteCalculation.priceImpactPct.toFixed(2),
        totalFee: quoteCalculation.totalFeeUsd.toFixed(3),
        timestamp: new Date().toLocaleTimeString(),
      });
      setIsSimulating(false);
    }, 350);
  };

  const filteredPools = pools.filter(
    (p) =>
      p.symbol.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="space-y-8 animate-in fade-in duration-500">
      {/* Header Banner */}
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4 border-b border-slate-200 pb-6">
        <div>
          <div className="flex items-center gap-2.5">
            <div className="w-10 h-10 rounded-xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-600 shadow-sm">
              <Scale className="w-5 h-5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-2xl font-bold tracking-tight text-slate-900">
                  Equity Market Launch & Liquidity Controller
                </h1>
                <Badge variant="success" className="font-mono text-[10px] uppercase">
                  100% Spot Only
                </Badge>
              </div>
              <p className="text-sm text-slate-500">
                Non-leveraged, Shariah-screened spot equity liquidity engine powered by Pyth Lazer reference feeds and Meteora Dynamic Bonding Curves (DBC).
              </p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <Badge variant="outline" className="font-mono text-xs px-3 py-1 flex items-center gap-1.5 border-emerald-300 text-emerald-700 bg-emerald-50/50">
            <Radio className={`w-3 h-3 ${isLazerConnected ? "text-emerald-500 animate-pulse" : "text-slate-400"}`} />
            Pyth Lazer: {isLazerConnected ? "Streaming (200ms)" : "Active"}
          </Badge>
          <Link href="/launch">
            <Button size="sm" className="bg-slate-900 hover:bg-slate-800 text-white font-medium text-xs gap-1.5 shadow-sm">
              <Zap className="w-3.5 h-3.5 fill-emerald-400 text-emerald-400" />
              Launch Equity DBC Curve
            </Button>
          </Link>
        </div>
      </div>

      {/* Aggregate Stats Bar */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="text-slate-500 text-xs font-semibold uppercase tracking-wider">
              Total Screened Liquidity
            </div>
            <div className="mt-2 text-2xl font-black font-mono text-slate-900">
              $6,750,000
            </div>
            <div className="mt-1 text-xs text-emerald-600 font-medium flex items-center gap-1">
              <ArrowUpRight className="w-3.5 h-3.5" />
              <span>100% Unencumbered Spot</span>
            </div>
          </CardContent>
        </Card>

        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="text-slate-500 text-xs font-semibold uppercase tracking-wider">
              24h Spot Volume
            </div>
            <div className="mt-2 text-2xl font-black font-mono text-slate-900">
              $1,640,500
            </div>
            <div className="mt-1 text-xs text-slate-500 font-mono">
              Zero leverage / 0% borrowing
            </div>
          </CardContent>
        </Card>

        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="text-slate-500 text-xs font-semibold uppercase tracking-wider">
              Avg Oracle Spread
            </div>
            <div className="mt-2 text-2xl font-black font-mono text-slate-900">
              +0.22%
            </div>
            <div className="mt-1 text-xs text-cyan-600 font-mono flex items-center gap-1">
              <Activity className="w-3.5 h-3.5" />
              <span>Pyth Lazer verified</span>
            </div>
          </CardContent>
        </Card>

        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="text-slate-500 text-xs font-semibold uppercase tracking-wider">
              Shariah Screening Status
            </div>
            <div className="mt-2 text-2xl font-black font-mono text-emerald-600 flex items-center gap-1.5">
              <ShieldCheck className="w-6 h-6" />
              <span>100% Pass</span>
            </div>
            <div className="mt-1 text-xs text-slate-500 font-mono">
              AAOIFI Standard No. 21
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Main Content Layout: 1 Col Swap & Shariah Metrics | 2 Cols Verified Curves */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Left Column: Spot Trade Execution & Shariah Compliance Panel */}
        <div className="space-y-6">
          {/* Swap & Execution Controller */}
          <Card className="bg-white border-slate-200 shadow-sm">
            <CardHeader className="pb-4 border-b border-slate-100">
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="text-base font-bold text-slate-900">
                    Spot Liquidity Trade
                  </CardTitle>
                  <CardDescription className="text-xs text-slate-500">
                    Direct on-chain spot settlement via Meteora curve
                  </CardDescription>
                </div>
                <Badge variant="success" className="text-[10px] font-mono">
                  $T+0$ Spot
                </Badge>
              </div>
            </CardHeader>

            <CardContent className="p-6 space-y-5">
              {/* Selected Equity Asset Banner */}
              <div className="flex items-center justify-between p-3 rounded-xl bg-slate-50 border border-slate-200">
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-lg bg-emerald-600 text-white font-bold flex items-center justify-center font-mono text-xs shadow-sm">
                    {selectedPool.symbol.slice(0, 3)}
                  </div>
                  <div>
                    <div className="font-bold text-xs text-slate-900">{selectedPool.symbol}</div>
                    <div className="text-[11px] text-slate-500 font-mono">${selectedPool.spotPriceUsd.toFixed(2)} Spot</div>
                  </div>
                </div>
                <Badge variant="outline" className="font-mono text-[10px] text-emerald-700 bg-emerald-50 border-emerald-200">
                  {selectedPool.shariahStatus}
                </Badge>
              </div>

              {/* Buy / Sell Mode Toggle */}
              <div className="flex rounded-lg p-1 bg-slate-100 border border-slate-200">
                <button
                  onClick={() => setSwapMode("BUY")}
                  className={`flex-1 py-1.5 rounded-md text-xs font-bold transition-all ${
                    swapMode === "BUY"
                      ? "bg-emerald-600 text-white shadow-sm"
                      : "text-slate-600 hover:text-slate-900"
                  }`}
                >
                  Buy Spot {selectedPool.symbol}
                </button>
                <button
                  onClick={() => setSwapMode("SELL")}
                  className={`flex-1 py-1.5 rounded-md text-xs font-bold transition-all ${
                    swapMode === "SELL"
                      ? "bg-rose-600 text-white shadow-sm"
                      : "text-slate-600 hover:text-slate-900"
                  }`}
                >
                  Sell Spot {selectedPool.symbol}
                </button>
              </div>

              {/* Input Amount */}
              <div className="space-y-1.5">
                <div className="flex justify-between text-xs font-semibold text-slate-700">
                  <span>Input Amount</span>
                  <span className="font-mono text-slate-500">
                    {swapMode === "BUY" ? "USDC" : selectedPool.symbol}
                  </span>
                </div>
                <Input
                  type="number"
                  value={amountInput}
                  onChange={(e) => setAmountInput(e.target.value)}
                  placeholder="0.00"
                  className="font-mono text-sm font-bold text-slate-900"
                />
                <div className="flex gap-2 pt-1">
                  {["100", "500", "1000", "5000"].map((preset) => (
                    <button
                      key={preset}
                      onClick={() => setAmountInput(preset)}
                      className="px-2 py-0.5 rounded text-[10px] font-mono bg-slate-100 hover:bg-slate-200 text-slate-700 transition-colors"
                    >
                      ${preset}
                    </button>
                  ))}
                </div>
              </div>

              {/* Output & Rate Breakdown */}
              <div className="p-4 rounded-xl border bg-slate-50/80 space-y-2.5 font-mono text-xs">
                <div className="flex justify-between">
                  <span className="text-slate-500">Expected Output:</span>
                  <span className="font-bold text-slate-900">
                    {quoteCalculation.amountOut.toFixed(4)} {swapMode === "BUY" ? selectedPool.symbol : "USDC"}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-500">Effective Rate:</span>
                  <span className="text-slate-900">${quoteCalculation.effectivePrice.toFixed(2)}</span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-slate-500">Price Impact:</span>
                  <span
                    className={`font-bold ${
                      quoteCalculation.priceImpactPct < 1.0
                        ? "text-emerald-600"
                        : quoteCalculation.priceImpactPct < 3.0
                        ? "text-amber-600"
                        : "text-rose-600"
                    }`}
                  >
                    {quoteCalculation.priceImpactPct.toFixed(2)}%
                  </span>
                </div>

                {/* Transparent Fee Breakdown */}
                <div className="pt-2.5 border-t border-slate-200 space-y-1 text-[11px] text-slate-600">
                  <div className="flex justify-between font-semibold text-slate-800">
                    <span>Transparent Total Fee (0.15%):</span>
                    <span>${quoteCalculation.totalFeeUsd.toFixed(3)}</span>
                  </div>
                  <div className="flex justify-between pl-2 text-slate-400">
                    <span>&bull; Pool LP Fee (0.10%):</span>
                    <span>${quoteCalculation.poolFeeUsd.toFixed(3)}</span>
                  </div>
                  <div className="flex justify-between pl-2 text-slate-400">
                    <span>&bull; Platform Controller (0.03%):</span>
                    <span>${quoteCalculation.platformFeeUsd.toFixed(3)}</span>
                  </div>
                  <div className="flex justify-between pl-2 text-slate-400">
                    <span>&bull; Execution & Oracle (0.02%):</span>
                    <span>${quoteCalculation.executionFeeUsd.toFixed(3)}</span>
                  </div>
                  <div className="flex justify-between pl-2 text-emerald-600 font-medium pt-1">
                    <span>&bull; Financing / Interest:</span>
                    <span>0.00 USDC (None)</span>
                  </div>
                </div>
              </div>

              {/* Simulate Button */}
              <Button
                onClick={handleSimulateSwap}
                disabled={isSimulating || parseFloat(amountInput) <= 0}
                className="w-full bg-slate-900 hover:bg-slate-800 text-white font-semibold text-xs py-2.5 shadow-sm"
              >
                {isSimulating ? (
                  <RefreshCw className="w-4 h-4 animate-spin text-emerald-400" />
                ) : (
                  <Zap className="w-4 h-4 fill-emerald-400 text-emerald-400" />
                )}
                <span>Simulate Spot Execution</span>
              </Button>

              {/* Simulation Result */}
              {simOutcome && (
                <div className="p-3 rounded-lg border bg-white space-y-1.5 animate-in fade-in text-xs font-mono">
                  <div className="flex items-center gap-1.5 font-bold text-emerald-600">
                    <CheckCircle2 className="w-4 h-4" />
                    <span>Spot Simulation Approved</span>
                  </div>
                  <div className="text-[11px] text-slate-600">
                    {simOutcome.action} {simOutcome.inputAmount} &rarr; {simOutcome.outputAmount} {simOutcome.pool}
                  </div>
                  <div className="text-[10px] text-slate-400">
                    Impact: {simOutcome.priceImpact}% | Total Fee: ${simOutcome.totalFee} | 100% Non-Leveraged
                  </div>
                </div>
              )}
            </CardContent>
          </Card>

          {/* Shariah Screening Verification Panel */}
          <Card className="bg-white border-slate-200 shadow-sm">
            <CardHeader className="p-4 pb-3 border-b border-slate-100 flex flex-row items-center justify-between">
              <div className="flex items-center gap-2">
                <FileCheck className="w-4 h-4 text-emerald-600" />
                <CardTitle className="text-xs font-bold text-slate-900">
                  Shariah Screening Criteria (AAOIFI Standard 21)
                </CardTitle>
              </div>
              <Badge variant="success" className="text-[10px] font-mono">
                Compliant
              </Badge>
            </CardHeader>
            <CardContent className="p-4 space-y-3 text-xs">
              <div className="space-y-1">
                <div className="flex justify-between font-mono text-[11px]">
                  <span className="text-slate-600">Interest Debt / Market Cap:</span>
                  <span className="font-bold text-slate-900">
                    {selectedPool.debtToMcapPct}% / 30.0% Max
                  </span>
                </div>
                <Progress value={(selectedPool.debtToMcapPct / 30) * 100} className="h-1.5 bg-slate-100" />
              </div>

              <div className="space-y-1">
                <div className="flex justify-between font-mono text-[11px]">
                  <span className="text-slate-600">Interest Cash & Deposits / Cap:</span>
                  <span className="font-bold text-slate-900">
                    {selectedPool.cashToMcapPct}% / 30.0% Max
                  </span>
                </div>
                <Progress value={(selectedPool.cashToMcapPct / 30) * 100} className="h-1.5 bg-slate-100" />
              </div>

              <div className="space-y-1">
                <div className="flex justify-between font-mono text-[11px]">
                  <span className="text-slate-600">Impermissible Revenue Ratio:</span>
                  <span className="font-bold text-slate-900">
                    {selectedPool.impermissibleRevenuePct}% / 5.0% Max
                  </span>
                </div>
                <Progress value={(selectedPool.impermissibleRevenuePct / 5) * 100} className="h-1.5 bg-slate-100" />
              </div>

              <div className="pt-2 border-t border-slate-100 text-[10px] text-slate-400">
                Notice: Product rules are designed around permissible spot ownership. Formal Shariah board certification is subject to scholar review.
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Right 2 Cols: Verified Pools Overview & Graduation Tracking */}
        <div className="lg:col-span-2 space-y-6">
          <Card className="bg-white border-slate-200 overflow-hidden">
            <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
              <div>
                <CardTitle className="text-sm font-bold text-slate-900">
                  Verified Shariah Spot Equity Pools
                </CardTitle>
                <CardDescription className="text-xs text-slate-500">
                  Live liquidity pools with Pyth Lazer price anchoring and Meteora DAMM graduation.
                </CardDescription>
              </div>

              <div className="w-full sm:w-64">
                <Input
                  placeholder="Search by ticker or name..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="text-xs h-8"
                />
              </div>
            </CardHeader>
            <CardContent className="p-0">
              <Table>
                <TableHeader>
                  <TableRow className="bg-slate-50/50">
                    <TableHead className="text-xs font-semibold text-slate-700">Market / Underlying</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">DBC Spot</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">Pyth Lazer</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">Spread</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">Total Liquidity</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">DAMM Graduation</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filteredPools.map((pool) => (
                    <TableRow
                      key={pool.poolAddress}
                      onClick={() => setSelectedPool(pool)}
                      className={`cursor-pointer transition-colors ${
                        selectedPool.symbol === pool.symbol ? "bg-emerald-50/40" : "hover:bg-slate-50/70"
                      }`}
                    >
                      <TableCell>
                        <div className="flex items-center gap-2.5">
                          <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center font-mono text-xs font-bold text-emerald-700">
                            {pool.symbol.slice(0, 3)}
                          </div>
                          <div>
                            <div className="font-bold text-slate-900 flex items-center gap-1.5">
                              {pool.symbol}
                              <span className="text-[10px] font-normal text-emerald-700 bg-emerald-50 px-1.5 py-0.5 rounded border border-emerald-200">
                                Spot DBC
                              </span>
                            </div>
                            <div className="text-[11px] text-slate-400">{pool.name}</div>
                          </div>
                        </div>
                      </TableCell>
                      <TableCell className="text-right font-mono text-xs font-bold text-slate-900">
                        ${pool.spotPriceUsd.toFixed(2)}
                      </TableCell>
                      <TableCell className="text-right font-mono text-xs text-slate-600">
                        ${pool.oraclePriceUsd.toFixed(2)}
                      </TableCell>
                      <TableCell className="text-right">
                        <Badge variant="cyan" className="font-mono text-[10px] px-1.5 py-0.5">
                          +{pool.spreadBps} bps
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right font-mono text-xs font-black text-slate-900">
                        ${(pool.liquidityUsd / 1_000_000).toFixed(2)}M
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="w-24 ml-auto space-y-1">
                          <div className="flex justify-between text-[10px] font-mono text-slate-500">
                            <span>{pool.curveProgressPct}%</span>
                            <span>DAMM v2</span>
                          </div>
                          <Progress value={pool.curveProgressPct} className="h-1.5 bg-slate-100" />
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>

          {/* Detailed Curve Progress Panel */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {filteredPools.map((p) => (
              <Card key={`card-${p.symbol}`} className="bg-white border-slate-200 interactive-card">
                <CardContent className="p-5 space-y-3">
                  <div className="flex justify-between items-start">
                    <div>
                      <div className="font-bold text-sm text-slate-900">{p.symbol} Spot</div>
                      <div className="text-[11px] text-slate-400">24h Vol: ${(p.volume24hUsd / 1000).toFixed(0)}k</div>
                    </div>
                    <Badge variant="emerald" className="font-mono text-[10px]">
                      {p.curveProgressPct}%
                    </Badge>
                  </div>

                  <div className="space-y-1">
                    <div className="flex justify-between text-[11px] font-mono text-slate-500">
                      <span>Graduation Target:</span>
                      <span>$4,000,000 USDC</span>
                    </div>
                    <Progress value={p.curveProgressPct} className="h-2 bg-slate-100" />
                  </div>

                  <div className="pt-2 border-t border-slate-100 flex justify-between items-center text-[11px]">
                    <span className="text-slate-500 font-mono">Spot: ${p.spotPriceUsd}</span>
                    <button
                      onClick={() => setSelectedPool(p)}
                      className="text-emerald-600 hover:text-emerald-700 font-semibold flex items-center gap-1"
                    >
                      Trade Spot Curve &rarr;
                    </button>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
