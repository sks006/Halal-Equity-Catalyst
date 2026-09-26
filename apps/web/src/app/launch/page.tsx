"use client";

import React, { useState, useEffect } from "react";
import Link from "next/link";
import {
  AlertCircle,
  ArrowUpRight,
  BarChart3,
  CheckCircle2,
  Clock,
  Layers,
  Plus,
  RefreshCw,
  Rocket,
  ShieldCheck,
  TrendingUp,
  Zap,
} from "lucide-react";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../components/ui/card";
import { Progress } from "../../components/ui/progress";
import { getApiClient, DbcPoolModel } from "../../lib/api-client";

export default function LaunchpadPage() {
  const [pools, setPools] = useState<DbcPoolModel[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const fetchPools = async () => {
    setLoading(true);
    setError(null);
    try {
      const client = getApiClient();
      const livePools = await client.listDbcPools();
      setPools(livePools || []);
    } catch (err: any) {
      // Fail closed: never display synthetic/sample pools
      setError("Unable to load live launchpad pools from network.");
      setPools([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchPools();
  }, []);

  const totalLiquidityUsd = pools.reduce(
    (acc, p) => acc + (p.current_price_usd ? p.current_price_usd * 10000 : 0),
    0
  );
  const graduatedCount = pools.filter((p) => p.is_migrated).length;

  return (
    <div className="space-y-8 pb-12">
      {/* Hero Header */}
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-6 pb-6 border-b border-slate-200">
        <div>
          <div className="flex items-center gap-2 mb-2">
            <Badge variant="emerald" className="gap-1 text-xs">
              <Zap className="w-3.5 h-3.5" />
              Meteora DBC Architecture
            </Badge>
            <Badge variant="secondary" className="gap-1 text-xs">
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
          <Link href="/launch/simulate">
            <Button variant="outline" className="border-slate-300 hover:bg-slate-50 text-slate-700 gap-2 shadow-sm">
              <BarChart3 className="w-4 h-4 text-purple-600" />
              Simulate Configurations
            </Button>
          </Link>
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
        <Card className="border-slate-200 bg-white">
          <CardHeader className="pb-2">
            <CardDescription className="text-xs font-semibold text-slate-500">
              Active DBC Pools
            </CardDescription>
            <CardTitle className="text-2xl font-bold text-slate-900">
              {loading ? "—" : error ? "Unavailable" : `${pools.length} Pools`}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-xs text-emerald-600 font-medium flex items-center gap-1">
              <TrendingUp className="w-3.5 h-3.5" /> 100% On-Chain Meteora DBC
            </span>
          </CardContent>
        </Card>

        <Card className="border-slate-200 bg-white">
          <CardHeader className="pb-2">
            <CardDescription className="text-xs font-semibold text-slate-500">
              Total Liquidity Locked
            </CardDescription>
            <CardTitle className="text-2xl font-bold text-slate-900">
              {loading ? "—" : error ? "Unavailable" : `$${totalLiquidityUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-xs text-slate-600 font-medium">
              Denominated in USDC Quote Reserves
            </span>
          </CardContent>
        </Card>

        <Card className="border-slate-200 bg-white">
          <CardHeader className="pb-2">
            <CardDescription className="text-xs font-semibold text-slate-500">
              DAMM v2 Migrations
            </CardDescription>
            <CardTitle className="text-2xl font-bold text-slate-900">
              {loading ? "—" : error ? "Unavailable" : `${graduatedCount} Pools`}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-xs text-purple-600 font-medium flex items-center gap-1">
              <CheckCircle2 className="w-3.5 h-3.5" /> Seamless Auto-Graduation
            </span>
          </CardContent>
        </Card>

        <Card className="border-slate-200 bg-white">
          <CardHeader className="pb-2">
            <CardDescription className="text-xs font-semibold text-slate-500">
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

        {loading ? (
          <Card className="bg-white p-12 text-center text-slate-400 text-xs">
            <RefreshCw className="w-6 h-6 animate-spin mx-auto mb-2 text-emerald-600" />
            Loading on-chain discovery pools...
          </Card>
        ) : error ? (
          <Card className="bg-white p-8 text-center border-amber-200">
            <div className="flex items-center justify-center gap-2 text-slate-600 text-sm">
              <AlertCircle className="w-5 h-5 text-amber-500" />
              <span>{error}</span>
            </div>
          </Card>
        ) : pools.length === 0 ? (
          <Card className="bg-white p-12 text-center text-slate-400 text-xs">
            No active launchpad pools found on-chain. Configure a new launch to initialize a bonding curve.
          </Card>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            {pools.map((pool) => (
              <Card key={pool.pool_address} className="border-slate-200 hover:border-slate-300 transition-all shadow-sm">
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <Badge variant="outline" className="text-xs">
                      Tokenized Equity
                    </Badge>
                    <Badge
                      variant={pool.is_migrated ? "emerald" : "cyan"}
                      className="text-xs font-semibold"
                    >
                      {pool.is_migrated ? "Graduated" : "Active Curve"}
                    </Badge>
                  </div>
                  <CardTitle className="text-lg font-bold text-slate-900 mt-2 flex items-center justify-between">
                    <span>{pool.token_symbol}</span>
                    <span className="text-xs font-mono text-slate-500">{pool.quote_mint.slice(0, 4)}...</span>
                  </CardTitle>
                  <CardDescription className="text-xs font-mono text-slate-400 truncate">
                    {pool.pool_address}
                  </CardDescription>
                </CardHeader>

                <CardContent className="space-y-4">
                  <div className="grid grid-cols-2 gap-2 p-3 bg-slate-50 rounded-lg text-xs">
                    <div>
                      <span className="text-slate-500 block">Current Price</span>
                      <span className="font-bold text-slate-900">
                        {pool.current_price_usd > 0 ? `$${pool.current_price_usd.toFixed(2)}` : "Price unavailable"}
                      </span>
                    </div>
                    <div>
                      <span className="text-slate-500 block">Status</span>
                      <span className="font-bold text-emerald-600">
                        {pool.is_migrated ? "Migrated DAMM" : "Bonding"}
                      </span>
                    </div>
                  </div>

                  <div className="space-y-1.5">
                    <div className="flex justify-between text-xs font-medium">
                      <span className="text-slate-500">Graduation Progress</span>
                      <span className="text-slate-900 font-mono">{pool.curve_progress_pct.toFixed(1)}%</span>
                    </div>
                    <Progress value={pool.curve_progress_pct} className="h-2 bg-slate-100" />
                  </div>

                  <div className="pt-2 border-t border-slate-100 flex items-center justify-between text-xs">
                    <span className="text-slate-500 flex items-center gap-1">
                      <Clock className="w-3.5 h-3.5" /> Active Discovery
                    </span>
                    <Link
                      href={`/launch/new?anchorPrice=${pool.current_price_usd}&asset=${encodeURIComponent(pool.token_symbol)}`}
                      className="text-emerald-600 hover:text-emerald-700 font-semibold flex items-center gap-1"
                    >
                      Inspect Curve <ArrowUpRight className="w-3.5 h-3.5" />
                    </Link>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
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
