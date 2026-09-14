"use client";

import React from "react";
import Link from "next/link";
import {
  ArrowRight,
  Layers,
  ShieldCheck,
  Sparkles,
  TrendingUp,
  Zap,
} from "lucide-react";

import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Card, CardContent } from "../components/ui/card";

export default function HomePage() {
  return (
    <div className="pt-8 pb-16 space-y-16">
      {/* Hero Section */}
      <div className="max-w-4xl mx-auto text-center space-y-6">
        <Badge variant="success" className="px-3.5 py-1.5 text-xs font-semibold gap-1.5">
          <Sparkles className="w-3.5 h-3.5 text-emerald-600" />
          <span>Solana High-Performance Quantitative Vaults</span>
        </Badge>

        <h1 className="text-4xl sm:text-6xl font-extrabold tracking-tight text-slate-900 leading-tight">
          Next-Generation Alpha with{" "}
          <span className="bg-gradient-to-r from-emerald-600 via-teal-600 to-cyan-600 bg-clip-text text-transparent">
            On-Chain Risk Shields
          </span>
        </h1>

        <p className="text-base sm:text-lg text-slate-600 max-w-2xl mx-auto leading-relaxed">
          Autonomous strategy execution powered by Pyth Network real-time price feeds,
          Jupiter swap routing, and dual-layer risk defense managed with React Redux.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4 pt-4">
          <Link href="/dashboard">
            <Button variant="emerald" size="lg" className="w-full sm:w-auto flex items-center gap-2 font-bold px-8">
              <span>Launch Dashboard</span>
              <ArrowRight className="w-4 h-4" />
            </Button>
          </Link>

          <Link href="/vault/new">
            <Button variant="outline" size="lg" className="w-full sm:w-auto flex items-center gap-2 font-semibold px-8">
              <Zap className="w-4 h-4 text-emerald-600" />
              <span>Deploy Strategy Vault</span>
            </Button>
          </Link>
        </div>
      </div>

      {/* Pillar Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 max-w-6xl mx-auto">
        {/* Pillar 1 */}
        <Card className="bg-white border-slate-200 shadow-sm hover:shadow-md transition-all">
          <CardContent className="p-6 space-y-3">
            <div className="w-10 h-10 rounded-xl bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-700">
              <ShieldCheck className="w-5 h-5" />
            </div>
            <h3 className="text-base font-bold text-slate-900">Dual-Line Risk Defense</h3>
            <p className="text-xs text-slate-600 leading-relaxed">
              Every transaction is validated first off-chain by our high-frequency Rust policy engine,
              then cryptographically verified on-chain via Anchor CPI guardrails before execution.
            </p>
          </CardContent>
        </Card>

        {/* Pillar 2 */}
        <Card className="bg-white border-slate-200 shadow-sm hover:shadow-md transition-all">
          <CardContent className="p-6 space-y-3">
            <div className="w-10 h-10 rounded-xl bg-cyan-50 border border-cyan-200 flex items-center justify-center text-cyan-700">
              <TrendingUp className="w-5 h-5" />
            </div>
            <h3 className="text-base font-bold text-slate-900">Pyth Real-Time Pricing</h3>
            <p className="text-xs text-slate-600 leading-relaxed">
              Zero-stale valuation. Portfolio weights and liquidation stop-losses are computed
              using sub-second Pyth Network price feeds normalized into exact domain representations.
            </p>
          </CardContent>
        </Card>

        {/* Pillar 3 */}
        <Card className="bg-white border-slate-200 shadow-sm hover:shadow-md transition-all">
          <CardContent className="p-6 space-y-3">
            <div className="w-10 h-10 rounded-xl bg-purple-50 border border-purple-200 flex items-center justify-center text-purple-700">
              <Layers className="w-5 h-5" />
            </div>
            <h3 className="text-base font-bold text-slate-900">Jupiter Liquidity Routing</h3>
            <p className="text-xs text-slate-600 leading-relaxed">
              Autonomous portfolio rebalancing routes across all Solana DEX liquidity pools
              via Jupiter swap quotes, strictly constrained by price impact thresholds.
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
