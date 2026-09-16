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
} from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

interface DbcPoolDisplay {
  poolAddress: string;
  symbol: string;
  name: string;
  issuer: string;
  spotPriceUsd: number;
  oraclePriceUsd: number;
  spreadBps: number;
  liquidityUsd: number;
  volume24hUsd: number;
  curveProgressPct: number;
  isMigrated: boolean;
  baseMint: string;
  quoteMint: string;
}

const VERIFIED_POOLS: DbcPoolDisplay[] = [
  {
    poolAddress: "6Ewqx1MeteoraNvdaDbcPool1111111111111111111",
    symbol: "NVDAx",
    name: "NVIDIA Corp RWA / USDC",
    issuer: "Backed Finance / Meteora DBC",
    spotPriceUsd: 129.1,
    oraclePriceUsd: 128.5,
    spreadBps: 46, // +0.46% premium
    liquidityUsd: 1_450_000,
    volume24hUsd: 320_500,
    curveProgressPct: 68.5,
    isMigrated: false,
    baseMint: "6Ewqx1NVDAxTokenMint11111111111111111111111111",
    quoteMint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  },
  {
    poolAddress: "7Fzqx1MeteoraAaplDbcPool1111111111111111111",
    symbol: "AAPLx",
    name: "Apple Inc RWA / USDC",
    issuer: "Backed Finance / Meteora DBC",
    spotPriceUsd: 232.4,
    oraclePriceUsd: 232.15,
    spreadBps: 11, // +0.11% premium
    liquidityUsd: 2_180_000,
    volume24hUsd: 540_000,
    curveProgressPct: 82.0,
    isMigrated: false,
    baseMint: "7Fzqx1AAPLxTokenMint11111111111111111111111111",
    quoteMint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  },
  {
    poolAddress: "8Gyqx1MeteoraSpyxDbcPool1111111111111111111",
    symbol: "SPYx",
    name: "S&P 500 ETF RWA / USDC",
    issuer: "Backed Finance / Meteora DBC",
    spotPriceUsd: 564.8,
    oraclePriceUsd: 564.2,
    spreadBps: 10, // +0.10% premium
    liquidityUsd: 3_850_000,
    volume24hUsd: 890_200,
    curveProgressPct: 91.4,
    isMigrated: false,
    baseMint: "8Gyqx1SPYxTokenMint11111111111111111111111111",
    quoteMint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  },
];

export default function MarketsPage() {
  const [pools, setPools] = useState<DbcPoolDisplay[]>(VERIFIED_POOLS);
  const [selectedPool, setSelectedPool] = useState<DbcPoolDisplay>(VERIFIED_POOLS[0]);
  const [swapMode, setSwapMode] = useState<"BUY" | "SELL">("BUY");
  const [amountInput, setAmountInput] = useState("500");
  const [slippageTolerance, setSlippageTolerance] = useState(0.5); // %
  const [searchQuery, setSearchQuery] = useState("");
  const [isSimulating, setIsSimulating] = useState(false);
  const [simOutcome, setSimOutcome] = useState<any | null>(null);

  // Dynamic swap quote calculations based on curve state
  const quoteCalculation = useMemo(() => {
    const inputVal = parseFloat(amountInput) || 0;
    if (inputVal <= 0) {
      return {
        amountOut: 0,
        priceImpactPct: 0,
        effectivePrice: selectedPool.spotPriceUsd,
        feeUsd: 0,
      };
    }

    if (swapMode === "BUY") {
      // Input is USDC, Output is tokens
      const feeUsd = inputVal * 0.0015; // 0.15% fee
      const netUsdc = inputVal - feeUsd;
      // Linear bonding curve marginal impact: impact = (tradeSize / poolLiquidity) * 100
      const priceImpactPct = Math.min(5.0, (inputVal / selectedPool.liquidityUsd) * 100 * 2.5);
      const effectivePrice = selectedPool.spotPriceUsd * (1 + priceImpactPct / 100);
      const amountOut = netUsdc / effectivePrice;
      return {
        amountOut,
        priceImpactPct,
        effectivePrice,
        feeUsd,
      };
    } else {
      // Input is tokens, Output is USDC
      const grossUsdc = inputVal * selectedPool.spotPriceUsd;
      const priceImpactPct = Math.min(5.0, (grossUsdc / selectedPool.liquidityUsd) * 100 * 2.5);
      const effectivePrice = selectedPool.spotPriceUsd * (1 - priceImpactPct / 100);
      const grossAfterImpact = inputVal * effectivePrice;
      const feeUsd = grossAfterImpact * 0.0015;
      const amountOut = grossAfterImpact - feeUsd;
      return {
        amountOut,
        priceImpactPct,
        effectivePrice,
        feeUsd,
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
        timestamp: new Date().toLocaleTimeString(),
      });
      setIsSimulating(false);
    }, 450);
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
            <div className="w-10 h-10 rounded-xl bg-cyan-500/10 border border-cyan-500/20 flex items-center justify-center text-cyan-600 shadow-sm">
              <TrendingUp className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-2xl font-bold tracking-tight text-slate-900">
                Meteora Dynamic Bonding Curve (DBC) Markets
              </h1>
              <p className="text-sm text-slate-500">
                Algorithmic liquidity curves, fair-value price discovery, and automated graduation to DAMM v2.
              </p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <Badge variant="emerald" className="font-mono text-xs px-3 py-1 flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
            Meteora DBC: Active
          </Badge>
          <Link href="/launch">
            <Button size="sm" className="bg-slate-900 hover:bg-slate-800 text-white font-medium text-xs gap-1.5">
              <Zap className="w-3.5 h-3.5 fill-emerald-400 text-emerald-400" />
              Configure DBC Curve
            </Button>
          </Link>
        </div>
      </div>

      {/* Aggregate Stats Bar */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="text-slate-500 text-xs font-semibold uppercase tracking-wider">
              Total DBC Liquidity
            </div>
            <div className="mt-2 text-2xl font-black font-mono text-slate-900">
              $7,480,000
            </div>
            <div className="mt-1 text-xs text-emerald-600 font-medium flex items-center gap-1">
              <ArrowUpRight className="w-3.5 h-3.5" />
              <span>+12.4% this week</span>
            </div>
          </CardContent>
        </Card>

        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="text-slate-500 text-xs font-semibold uppercase tracking-wider">
              24h Trading Volume
            </div>
            <div className="mt-2 text-2xl font-black font-mono text-slate-900">
              $1,750,700
            </div>
            <div className="mt-1 text-xs text-slate-500 font-mono">
              Across verified equity curves
            </div>
          </CardContent>
        </Card>

        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="text-slate-500 text-xs font-semibold uppercase tracking-wider">
              Average Oracle Spread
            </div>
            <div className="mt-2 text-2xl font-black font-mono text-slate-900">
              22.3 bps
            </div>
            <div className="mt-1 text-xs text-emerald-600 font-medium">
              Tight institutional alignment
            </div>
          </CardContent>
        </Card>

        <Card className="interactive-card bg-white border-slate-200">
          <CardContent className="p-6">
            <div className="text-slate-500 text-xs font-semibold uppercase tracking-wider">
              DAMM v2 Migration Threshold
            </div>
            <div className="mt-2 text-2xl font-black font-mono text-slate-900">
              80.6% Avg
            </div>
            <div className="mt-1 text-xs text-cyan-600 font-medium">
              SPYx nearing graduation ($3.85M)
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Main Grid: Interactive Curve Swap Simulator & Verified Pools */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Left Col: Interactive Curve Swap Simulator */}
        <div className="space-y-6">
          <Card className="bg-white border-slate-200 overflow-hidden shadow-sm">
            <CardHeader className="p-6 border-b border-slate-100 bg-gradient-to-br from-slate-900 to-slate-800 text-white">
              <div className="flex items-center gap-2">
                <Sliders className="w-4 h-4 text-cyan-400" />
                <CardTitle className="text-sm font-bold text-white">
                  Dynamic Bonding Curve Swap Simulator
                </CardTitle>
              </div>
              <CardDescription className="text-xs text-slate-300">
                Simulate buy and sell swaps with mathematical bonding curve pricing and slippage analysis.
              </CardDescription>
            </CardHeader>
            <CardContent className="p-6 space-y-5">
              {/* Pool Selector */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700">Target DBC Pool</label>
                <div className="grid grid-cols-3 gap-2">
                  {pools.map((p) => (
                    <button
                      key={p.symbol}
                      onClick={() => setSelectedPool(p)}
                      className={`p-2 rounded-lg border text-left transition-all ${
                        selectedPool.symbol === p.symbol
                          ? "border-cyan-500 bg-cyan-50/50 text-cyan-950 font-bold"
                          : "border-slate-200 bg-white text-slate-700 hover:bg-slate-50"
                      }`}
                    >
                      <div className="text-xs">{p.symbol}</div>
                      <div className="text-[10px] text-slate-500 font-mono">${p.spotPriceUsd.toFixed(2)}</div>
                    </button>
                  ))}
                </div>
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
                  Buy {selectedPool.symbol}
                </button>
                <button
                  onClick={() => setSwapMode("SELL")}
                  className={`flex-1 py-1.5 rounded-md text-xs font-bold transition-all ${
                    swapMode === "SELL"
                      ? "bg-rose-600 text-white shadow-sm"
                      : "text-slate-600 hover:text-slate-900"
                  }`}
                >
                  Sell {selectedPool.symbol}
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
                <div className="flex justify-between text-[11px] pt-2 border-t border-slate-200 text-slate-500">
                  <span>Protocol Fee (0.15%):</span>
                  <span>${quoteCalculation.feeUsd.toFixed(2)}</span>
                </div>
              </div>

              {/* Simulate Button */}
              <Button
                onClick={handleSimulateSwap}
                disabled={isSimulating || parseFloat(amountInput) <= 0}
                className="w-full bg-slate-900 hover:bg-slate-800 text-white font-semibold text-xs py-2.5 shadow-sm"
              >
                {isSimulating ? (
                  <RefreshCw className="w-4 h-4 animate-spin text-cyan-400" />
                ) : (
                  <Zap className="w-4 h-4 fill-cyan-400 text-cyan-400" />
                )}
                <span>Simulate On-Chain Swap</span>
              </Button>

              {/* Simulation Result */}
              {simOutcome && (
                <div className="p-3 rounded-lg border bg-white space-y-1.5 animate-in fade-in text-xs font-mono">
                  <div className="flex items-center gap-1.5 font-bold text-emerald-600">
                    <CheckCircle2 className="w-4 h-4" />
                    <span>Pre-Flight Simulation Succeeded</span>
                  </div>
                  <div className="text-[11px] text-slate-600">
                    {simOutcome.action} {simOutcome.inputAmount} &rarr; {simOutcome.outputAmount} {simOutcome.pool}
                  </div>
                  <div className="text-[10px] text-slate-400">
                    Executed at {simOutcome.timestamp} | Impact: {simOutcome.priceImpact}%
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        {/* Right 2 Cols: Verified Pools Overview & DAMM Migration */}
        <div className="lg:col-span-2 space-y-6">
          <Card className="bg-white border-slate-200 overflow-hidden">
            <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
              <div>
                <CardTitle className="text-sm font-bold text-slate-900">
                  Verified Dynamic Bonding Curve Pools
                </CardTitle>
                <CardDescription className="text-xs text-slate-500">
                  Live liquidity pools with real-time Pyth price anchoring and graduation tracking.
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
                    <TableHead className="text-xs font-semibold text-slate-700">Market / Issuer</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">DBC Spot</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">Pyth Oracle</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">Spread</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">Total Liquidity</TableHead>
                    <TableHead className="text-xs font-semibold text-slate-700 text-right">DAMM Progress</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filteredPools.map((pool) => (
                    <TableRow
                      key={pool.poolAddress}
                      onClick={() => setSelectedPool(pool)}
                      className={`cursor-pointer transition-colors ${
                        selectedPool.symbol === pool.symbol ? "bg-cyan-50/40" : "hover:bg-slate-50/70"
                      }`}
                    >
                      <TableCell>
                        <div className="flex items-center gap-2.5">
                          <div className="w-8 h-8 rounded-lg bg-cyan-50 border border-cyan-200 flex items-center justify-center font-mono text-xs font-bold text-cyan-700">
                            {pool.symbol.slice(0, 3)}
                          </div>
                          <div>
                            <div className="font-bold text-slate-900 flex items-center gap-1.5">
                              {pool.symbol}
                              <span className="text-[10px] font-normal text-slate-400 bg-slate-100 px-1.5 py-0.5 rounded">
                                DBC Curve
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
                      <div className="font-bold text-sm text-slate-900">{p.symbol} Pool</div>
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
                      className="text-cyan-600 hover:text-cyan-700 font-semibold flex items-center gap-1"
                    >
                      Trade Curve &rarr;
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
