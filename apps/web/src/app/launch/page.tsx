"use client";

import Link from "next/link";
import {
  ArrowUpRight,
  BarChart3,
  CheckCircle2,
  Clock,
  Layers,
  Plus,
  Rocket,
  ShieldCheck,
  TrendingUp,
  Zap,
} from "lucide-react";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../components/ui/card";
import { Progress } from "../../components/ui/progress";

interface LaunchpadPool {
  id: string;
  name: string;
  symbol: string;
  assetType: string;
  anchorPrice: number;
  currentPrice: number;
  quoteToken: string;
  quoteCollected: number;
  graduationThreshold: number;
  progressPct: number;
  regime: "Regime A" | "Regime B" | "Regime C" | "Graduated";
  poolAddress: string;
}

const SAMPLE_DBC_POOLS: LaunchpadPool[] = [
  {
    id: "pool-1",
    name: "NVIDIA Synthetic xStock",
    symbol: "NVDA-x",
    assetType: "Tokenized Stock",
    anchorPrice: 120.0,
    currentPrice: 124.5,
    quoteToken: "USDC",
    quoteCollected: 580000,
    graduationThreshold: 750000,
    progressPct: 77.3,
    regime: "Regime B",
    poolAddress: "DBCnvd99xStock111111111111111111111111111111",
  },
  {
    id: "pool-2",
    name: "Apple Fractional Equity",
    symbol: "AAPL-x",
    assetType: "Tokenized Stock",
    anchorPrice: 220.0,
    currentPrice: 228.1,
    quoteToken: "USDC",
    quoteCollected: 685000,
    graduationThreshold: 750000,
    progressPct: 91.3,
    regime: "Regime C",
    poolAddress: "DBCaap11xStock222222222222222222222222222222",
  },
  {
    id: "pool-3",
    name: "Space Exploration Pre-IPO",
    symbol: "SPCX-p",
    assetType: "Pre-IPO Equity",
    anchorPrice: 100.0,
    currentPrice: 101.2,
    quoteToken: "USDC",
    quoteCollected: 210000,
    graduationThreshold: 750000,
    progressPct: 28.0,
    regime: "Regime A",
    poolAddress: "DBCspcx33PreIpo33333333333333333333333333333",
  },
];

export default function LaunchpadPage() {
  return (
    <div className="space-y-8 pb-12">
      {/* Hero Header */}
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-6 pb-6 border-b border-slate-200">
        <div>
          <div className="flex items-center gap-2 mb-2">
            <Badge variant="emerald" className="gap-1 font-mono text-xs">
              <Zap className="w-3 h-3" />
              Meteora DBC Architecture
            </Badge>
            <Badge variant="purple" className="gap-1 font-mono text-xs">
              Program: dbcij3...aqN
            </Badge>
          </div>
          <h1 className="text-3xl font-extrabold tracking-tight text-slate-900">
            Equity Discovery Launchpad
          </h1>
          <p className="text-sm text-slate-600 mt-1 max-w-2xl">
            Customizable on-chain bonding curves engineered specifically for tokenized equity markets,
            featuring 3-regime concentrated liquidity and seamless auto-graduation to Meteora DAMM v2.
          </p>
        </div>

        <div className="flex items-center gap-3">
          <Link href="/launch/new">
            <Button className="bg-emerald-600 hover:bg-emerald-700 text-white gap-2 shadow-sm">
              <Plus className="w-4 h-4" />
              Configure New Launch
            </Button>
          </Link>
        </div>
      </div>

      {/* Highlights Metrics Banner */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card className="border-slate-200 bg-gradient-to-br from-white to-slate-50">
          <CardHeader className="pb-2">
            <CardDescription className="text-xs font-semibold uppercase tracking-wider text-slate-500">
              Active DBC Pools
            </CardDescription>
            <CardTitle className="text-2xl font-bold text-slate-900">3 Live</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-xs text-emerald-600 font-medium flex items-center gap-1">
              <TrendingUp className="w-3.5 h-3.5" /> 100% On-Chain Meteora DBC
            </span>
          </CardContent>
        </Card>

        <Card className="border-slate-200 bg-gradient-to-br from-white to-slate-50">
          <CardHeader className="pb-2">
            <CardDescription className="text-xs font-semibold uppercase tracking-wider text-slate-500">
              Total Liquidity Locked
            </CardDescription>
            <CardTitle className="text-2xl font-bold text-slate-900">$1,475,000</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-xs text-slate-600 font-medium">
              Denominated in USDC Quote Reserves
            </span>
          </CardContent>
        </Card>

        <Card className="border-slate-200 bg-gradient-to-br from-white to-slate-50">
          <CardHeader className="pb-2">
            <CardDescription className="text-xs font-semibold uppercase tracking-wider text-slate-500">
              DAMM v2 Migrations
            </CardDescription>
            <CardTitle className="text-2xl font-bold text-slate-900">1 Pool</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-xs text-purple-600 font-medium flex items-center gap-1">
              <CheckCircle2 className="w-3.5 h-3.5" /> Seamless Auto-Graduation
            </span>
          </CardContent>
        </Card>

        <Card className="border-slate-200 bg-gradient-to-br from-white to-slate-50">
          <CardHeader className="pb-2">
            <CardDescription className="text-xs font-semibold uppercase tracking-wider text-slate-500">
              Anti-Snipe Protection
            </CardDescription>
            <CardTitle className="text-2xl font-bold text-emerald-600">Active</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-xs text-slate-600 font-medium">
              Linear FeeScheduler & Dynamic Fees
            </span>
          </CardContent>
        </Card>
      </div>

      {/* Active DBC Pools */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-bold text-slate-900">Active Equity Discovery Pools</h2>
            <p className="text-xs text-slate-500">
              Bonding curve pools accumulating liquidity toward DAMM v2 migration threshold
            </p>
          </div>
          <Link href="/launch/new">
            <Button variant="outline" size="sm" className="text-xs gap-1.5">
              <Rocket className="w-3.5 h-3.5 text-emerald-600" /> Launch Token
            </Button>
          </Link>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {SAMPLE_DBC_POOLS.map((pool) => (
            <Card key={pool.id} className="border-slate-200 hover:border-slate-300 transition-all shadow-sm">
              <CardHeader className="pb-3">
                <div className="flex items-center justify-between">
                  <Badge variant="outline" className="font-mono text-[10px]">
                    {pool.assetType}
                  </Badge>
                  <Badge
                    variant={
                      pool.regime === "Regime A"
                        ? "cyan"
                        : pool.regime === "Regime B"
                        ? "emerald"
                        : "purple"
                    }
                    className="text-[10px] font-semibold"
                  >
                    {pool.regime}
                  </Badge>
                </div>
                <CardTitle className="text-lg font-bold text-slate-900 mt-2 flex items-center justify-between">
                  <span>{pool.name}</span>
                  <span className="text-sm font-mono text-slate-500">{pool.symbol}</span>
                </CardTitle>
                <CardDescription className="text-xs font-mono text-slate-400 truncate">
                  {pool.poolAddress}
                </CardDescription>
              </CardHeader>

              <CardContent className="space-y-4">
                <div className="grid grid-cols-2 gap-2 p-3 bg-slate-50 rounded-lg text-xs">
                  <div>
                    <span className="text-slate-500 block">Anchor Fair Value</span>
                    <span className="font-bold text-slate-900">${pool.anchorPrice.toFixed(2)}</span>
                  </div>
                  <div>
                    <span className="text-slate-500 block">Current Price</span>
                    <span className="font-bold text-emerald-600">${pool.currentPrice.toFixed(2)}</span>
                  </div>
                </div>

                <div className="space-y-1.5">
                  <div className="flex justify-between text-xs font-medium">
                    <span className="text-slate-500">Graduation Progress</span>
                    <span className="text-slate-900 font-mono">{pool.progressPct}%</span>
                  </div>
                  <Progress value={pool.progressPct} className="h-2 bg-slate-100" />
                  <div className="flex justify-between text-[11px] text-slate-400 font-mono pt-1">
                    <span>${pool.quoteCollected.toLocaleString()} {pool.quoteToken}</span>
                    <span>Target: ${pool.graduationThreshold.toLocaleString()}</span>
                  </div>
                </div>

                <div className="pt-2 border-t border-slate-100 flex items-center justify-between text-xs">
                  <span className="text-slate-500 flex items-center gap-1">
                    <Clock className="w-3.5 h-3.5" /> Active Discovery
                  </span>
                  <Link
                    href={`/launch/new?anchorPrice=${pool.anchorPrice}&asset=${encodeURIComponent(pool.name)}`}
                    className="text-emerald-600 hover:text-emerald-700 font-semibold flex items-center gap-1"
                  >
                    Inspect Curve <ArrowUpRight className="w-3.5 h-3.5" />
                  </Link>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>

      {/* Architectural Philosophy Section */}
      <Card className="border-emerald-100 bg-emerald-50/40">
        <CardHeader>
          <div className="flex items-center gap-2 text-emerald-700 text-xs font-bold uppercase tracking-wider">
            <ShieldCheck className="w-4 h-4" /> Architectural Philosophy
          </div>
          <CardTitle className="text-xl font-bold text-slate-900">
            Why the Equity Discovery Curve?
          </CardTitle>
          <CardDescription className="text-sm text-slate-600">
            &quot;The novelty is not that we simply changed the curve; it is that we designed the curve
            around the behavior of tokenized-stock markets rather than meme-token speculation.&quot;
          </CardDescription>
        </CardHeader>
        <CardContent className="grid grid-cols-1 md:grid-cols-3 gap-6 pt-2">
          <div className="p-4 rounded-xl bg-white border border-emerald-100 shadow-sm space-y-2">
            <div className="w-7 h-7 rounded-lg bg-emerald-100 text-emerald-700 flex items-center justify-center font-bold text-xs">
              A
            </div>
            <h3 className="text-sm font-bold text-slate-900">Regime A: Launch & Bootstrapping</h3>
            <p className="text-xs text-slate-600">
              Low/moderate slope from 85% to 100% of anchor price. Linear FeeScheduler disincentivizes
              sniper frontrunning, allowing orderly initial allocation.
            </p>
          </div>

          <div className="p-4 rounded-xl bg-white border border-emerald-100 shadow-sm space-y-2">
            <div className="w-7 h-7 rounded-lg bg-emerald-100 text-emerald-700 flex items-center justify-center font-bold text-xs">
              B
            </div>
            <h3 className="text-sm font-bold text-slate-900">Regime B: Active Discovery</h3>
            <p className="text-xs text-slate-600">
              4x concentrated liquidity centered around consensus fair value ($100-$130). Absorbs
              institutional $5k–$50k block orders with minimal slippage.
            </p>
          </div>

          <div className="p-4 rounded-xl bg-white border border-emerald-100 shadow-sm space-y-2">
            <div className="w-7 h-7 rounded-lg bg-emerald-100 text-emerald-700 flex items-center justify-center font-bold text-xs">
              C
            </div>
            <h3 className="text-sm font-bold text-slate-900">Regime C: Mature Market Buffer</h3>
            <p className="text-xs text-slate-600">
              8x concentrated depth dampens terminal volatility before graduation, eliminating the
              notorious &quot;migration dump cliff&quot; before moving into Meteora DAMM v2.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
