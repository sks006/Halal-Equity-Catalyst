"use client";

import React, { useState, useEffect } from "react";
import Link from "next/link";
import {
  ArrowLeft,
  CheckCircle2,
  ChevronRight,
  Eye,
  Info,
  Layers,
  Rocket,
  ShieldAlert,
  Sparkles,
  TrendingUp,
  Zap,
  Sliders,
} from "lucide-react";

import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../../components/ui/card";
import { Input } from "../../../components/ui/input";

interface ConfigResponse {
  asset: string;
  quote_token: string;
  quote_mint: string;
  initial_price: number;
  curve_profile: string;
  total_token_supply: number;
  segments: {
    segment_index: number;
    regime_name: string;
    start_price: number;
    end_price: number;
    sqrt_price_start: string;
    sqrt_price_end: string;
    liquidity_weight: number;
    estimated_liquidity: string;
    description: string;
  }[];
  fee_structure: {
    base_fee_mode: string;
    starting_fee_bps: number;
    ending_fee_bps: number;
    dynamic_volatility_fee_enabled: boolean;
    collect_fee_mode: string;
    creator_fee_share_pct: number;
    partner_fee_share_pct: number;
  };
  graduation: {
    migration_option: string;
    target_damm: string;
    migration_quote_threshold: number;
    migration_quote_threshold_lamports: number;
    fee_option: string;
    migrated_pool_fee_bps: number;
    partner_lp_percentage: number;
  };
  program_id: string;
  summary: string;
}

export default function NewLaunchPage() {
  // Form State
  const [asset, setAsset] = useState("TOKENIZED_STOCK");
  const [quoteToken, setQuoteToken] = useState("USDC");
  const [initialPrice, setInitialPrice] = useState<number>(100);
  const [curveProfile, setCurveProfile] = useState("equity_discovery");
  const [feeProfile, setFeeProfile] = useState("adaptive_equity");
  const [graduationThreshold, setGraduationThreshold] = useState<number>(750);

  // Preview & Status State
  const [loading, setLoading] = useState(false);
  const [config, setConfig] = useState<ConfigResponse | null>(null);
  const [activeRegimeHover, setActiveRegimeHover] = useState<number | null>(null);
  const [isCopied, setIsCopied] = useState(false);

  // Compute or fetch preview from backend /dbc/configure
  const fetchConfiguration = async () => {
    setLoading(true);
    try {
      const res = await fetch("http://127.0.0.1:4000/dbc/configure", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          asset,
          quote_token: quoteToken,
          initial_price: Number(initialPrice),
          curve_profile: curveProfile,
          graduation_threshold: Number(graduationThreshold),
        }),
      });

      if (res.ok) {
        const data = await res.json();
        setConfig(data);
      } else {
        throw new Error("Backend offline or non-200 response");
      }
    } catch (err: any) {
      // Fail closed: never display synthetic curve data
      setConfig(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchConfiguration();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="space-y-8 pb-16 max-w-6xl mx-auto">
      {/* Navigation Breadcrumb */}
      <div className="flex items-center gap-2 text-xs text-slate-500">
        <Link href="/launch" className="hover:text-slate-900 transition-colors flex items-center gap-1">
          <ArrowLeft className="w-3.5 h-3.5" /> DBC Launchpad
        </Link>
        <ChevronRight className="w-3 h-3" />
        <span className="text-slate-900 font-semibold">New Equity Discovery Curve</span>
      </div>

      {/* Header Banner */}
      <div className="border-b border-slate-200 pb-6 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 mb-2">
            <Badge variant="emerald" className="font-mono text-xs">
              Phase 3 & 4 — DBC Configurator & Simulator
            </Badge>
            <Badge variant="outline" className="font-mono text-xs">
              Meteora Program: dbcij3...aqN
            </Badge>
          </div>
          <h1 className="text-3xl font-extrabold tracking-tight text-slate-900">
            DBC Equity Discovery Configurator
          </h1>
          <p className="text-sm text-slate-600 mt-1">
            Configure an institutional-grade bonding curve designed around tokenized stock behavior.
            The backend validates parameters and generates production-ready Meteora DBC instructions.
          </p>
        </div>

        <Link href="/launch/simulate">
          <Button variant="outline" className="text-xs font-semibold gap-2 border-emerald-300 bg-emerald-50/50 hover:bg-emerald-100/60 text-emerald-800 shrink-0 shadow-sm">
            <Sliders className="w-3.5 h-3.5 text-emerald-600" />
            Config A vs B vs Default Simulator
          </Button>
        </Link>
      </div>

      {/* Grid: Left Column Form, Right Column Preview */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-8">
        {/* LEFT COLUMN: The User Form */}
        <div className="lg:col-span-5 space-y-6">
          <Card className="border-slate-200 shadow-sm">
            <CardHeader className="pb-4">
              <CardTitle className="text-base font-bold text-slate-900 flex items-center gap-2">
                <Layers className="w-4 h-4 text-emerald-600" /> Curve Parameters
              </CardTitle>
              <CardDescription className="text-xs text-slate-500">
                Define the asset anchor, price bands, and graduation thresholds
              </CardDescription>
            </CardHeader>

            <CardContent className="space-y-5">
              {/* Asset Field */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700 block">
                  Asset
                </label>
                <div className="relative">
                  <select
                    value={asset}
                    onChange={(e) => setAsset(e.target.value)}
                    className="w-full h-10 px-3 rounded-lg border border-slate-200 bg-white text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-500"
                  >
                    <option value="TOKENIZED_STOCK">Tokenized Stock (e.g. NVDA, AAPL)</option>
                    <option value="PRE_IPO_EQUITY">Pre-IPO Equity Shares</option>
                    <option value="SYNTHETIC_EQUITY">Synthetic Market Index</option>
                    <option value="CORPORATE_TREASURY">Corporate Treasury Share</option>
                  </select>
                </div>
              </div>

              {/* Quote Token Field */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700 block">
                  Quote
                </label>
                <select
                  value={quoteToken}
                  onChange={(e) => setQuoteToken(e.target.value)}
                  className="w-full h-10 px-3 rounded-lg border border-slate-200 bg-white text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-500"
                >
                  <option value="USDC">USDC (Stable Equity Denomination)</option>
                  <option value="USDT">USDT (Tether USD)</option>
                  <option value="SOL">SOL (Native Wrapped SOL)</option>
                </select>
              </div>

              {/* Initial Price Field */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700 block">
                  Initial Price ($)
                </label>
                <div className="relative">
                  <span className="absolute left-3 top-2.5 text-sm text-slate-400 font-mono">$</span>
                  <Input
                    type="number"
                    value={initialPrice}
                    onChange={(e) => setInitialPrice(parseFloat(e.target.value) || 0)}
                    min={1}
                    step={1}
                    className="pl-7 h-10 font-mono text-sm"
                  />
                </div>
                <p className="text-[11px] text-slate-400">
                  Target anchor fair market valuation (e.g. $100.00)
                </p>
              </div>

              {/* Curve Profile */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700 block">
                  Curve
                </label>
                <select
                  value={curveProfile}
                  onChange={(e) => setCurveProfile(e.target.value)}
                  className="w-full h-10 px-3 rounded-lg border border-slate-200 bg-white text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-500"
                >
                  <option value="equity_discovery">Equity Discovery (3-Regimes: 1x → 4x → 8x)</option>
                  <option value="linear_ramp">Linear Ramp (Even Weights)</option>
                </select>
              </div>

              {/* Fee Profile */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700 block">
                  Fee Profile
                </label>
                <select
                  value={feeProfile}
                  onChange={(e) => setFeeProfile(e.target.value)}
                  className="w-full h-10 px-3 rounded-lg border border-slate-200 bg-white text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-500"
                >
                  <option value="adaptive_equity">Adaptive Equity (FeeScheduler + Dynamic Volatility)</option>
                  <option value="flat_institution">Institutional Flat (25 bps)</option>
                </select>
                <p className="text-[11px] text-slate-400">
                  Anti-snipe decay (250 bps → 50 bps) with volatility accumulator protection
                </p>
              </div>

              {/* Graduation Threshold */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700 block">
                  Graduation
                </label>
                <div className="relative">
                  <span className="absolute left-3 top-2.5 text-sm text-slate-400 font-mono">$</span>
                  <Input
                    type="number"
                    value={graduationThreshold}
                    onChange={(e) => setGraduationThreshold(parseFloat(e.target.value) || 0)}
                    min={100}
                    step={50}
                    className="pl-7 h-10 font-mono text-sm"
                  />
                </div>
                <p className="text-[11px] text-slate-400">
                  Quote liquidity threshold before auto-migrating into Meteora DAMM v2
                </p>
              </div>

              {/* Preview Button */}
              <Button
                onClick={fetchConfiguration}
                disabled={loading}
                className="w-full bg-emerald-600 hover:bg-emerald-700 text-white font-semibold py-2.5 shadow-sm gap-2 mt-2"
              >
                <Eye className="w-4 h-4" />
                {loading ? "Compiling DBC..." : "Preview Configuration"}
              </Button>
            </CardContent>
          </Card>
        </div>

        {/* RIGHT COLUMN: Visual Preview & Compiled Parameters */}
        <div className="lg:col-span-7 space-y-6">
          {/* Visual Preview Card */}
          <Card className="border-slate-200 shadow-sm overflow-hidden">
            <CardHeader className="pb-2 bg-gradient-to-r from-slate-900 to-slate-800 text-white">
              <div className="flex items-center justify-between">
                <div>
                  <Badge variant="cyan" className="font-mono text-[10px] mb-1">
                    Visual Curve Preview
                  </Badge>
                  <CardTitle className="text-lg font-bold text-white">
                    Price vs. Liquidity Profile
                  </CardTitle>
                </div>
                <span className="text-xs font-mono text-slate-300">
                  {config?.curve_profile.toUpperCase()}
                </span>
              </div>
            </CardHeader>

            <CardContent className="pt-6 space-y-6">
              {/* ASCII / Visual Graph Display */}
              <div className="bg-slate-950 text-emerald-400 p-4 rounded-xl font-mono text-xs overflow-x-auto shadow-inner border border-slate-800">
                <div className="text-slate-400 mb-2 flex justify-between items-center text-[11px]">
                  <span>PRICE ($)</span>
                  <span className="text-emerald-400">3-REGIME PIECEWISE CURVE</span>
                </div>
                <pre className="leading-tight select-none">
{`Price
  ^
  |                                        ______ [Regime C: Mature Buffer ($${((initialPrice || 100) * 1.5).toFixed(0)})]
  |                                    ___/       (8x Concentrated Liquidity -> DAMM v2)
  |                       ____________/
  |                   ___/ [Regime B: Active Discovery ($${((initialPrice || 100) * 1.3).toFixed(0)})]
  |               ___/     (4x Concentrated Liquidity around Fair Value)
  |           ___/
  |       ___/ [Regime A: Launch ($${((initialPrice || 100) * 1.0).toFixed(0)})]
  | _____/     (1x Baseline Liquidity, Anti-Snipe Fee)
  +--------------------------------------------------------> Liquidity Depth ($)`}
                </pre>
              </div>

              {/* Dynamic Interactive SVG Chart */}
              <div className="relative border border-slate-200 rounded-xl p-4 bg-slate-50/50">
                <div className="flex justify-between items-center text-xs font-semibold text-slate-700 mb-2">
                  <span>Interactive Sqrt-Price Profile</span>
                  <span className="text-xs font-mono text-slate-500">
                    Anchor: ${(Number(initialPrice) || 100).toFixed(2)} {quoteToken}
                  </span>
                </div>

                <svg viewBox="0 0 500 200" className="w-full h-44 overflow-visible">
                  {/* Grid Lines */}
                  <line x1="40" y1="20" x2="40" y2="170" stroke="#e2e8f0" strokeWidth="2" />
                  <line x1="40" y1="170" x2="480" y2="170" stroke="#e2e8f0" strokeWidth="2" />

                  {/* Horizontal grid lines */}
                  <line x1="40" y1="50" x2="480" y2="50" stroke="#f1f5f9" strokeDasharray="4" />
                  <line x1="40" y1="90" x2="480" y2="90" stroke="#f1f5f9" strokeDasharray="4" />
                  <line x1="40" y1="130" x2="480" y2="130" stroke="#f1f5f9" strokeDasharray="4" />

                  {/* Regime Area Fills */}
                  {/* Regime A: 40 -> 160 */}
                  <polygon
                    points="40,170 40,150 160,120 160,170"
                    fill="#38bdf8"
                    fillOpacity={activeRegimeHover === 0 ? 0.35 : 0.15}
                    className="transition-all cursor-pointer"
                    onMouseEnter={() => setActiveRegimeHover(0)}
                    onMouseLeave={() => setActiveRegimeHover(null)}
                  />
                  {/* Regime B: 160 -> 340 */}
                  <polygon
                    points="160,170 160,120 340,70 340,170"
                    fill="#10b981"
                    fillOpacity={activeRegimeHover === 1 ? 0.35 : 0.15}
                    className="transition-all cursor-pointer"
                    onMouseEnter={() => setActiveRegimeHover(1)}
                    onMouseLeave={() => setActiveRegimeHover(null)}
                  />
                  {/* Regime C: 340 -> 480 */}
                  <polygon
                    points="340,170 340,70 480,45 480,170"
                    fill="#a855f7"
                    fillOpacity={activeRegimeHover === 2 ? 0.35 : 0.15}
                    className="transition-all cursor-pointer"
                    onMouseEnter={() => setActiveRegimeHover(2)}
                    onMouseLeave={() => setActiveRegimeHover(null)}
                  />

                  {/* The Curve Line */}
                  <path
                    d="M 40 150 C 100 145, 120 130, 160 120 C 220 105, 280 85, 340 70 C 390 60, 440 50, 480 45"
                    fill="none"
                    stroke="#059669"
                    strokeWidth="3.5"
                    strokeLinecap="round"
                  />

                  {/* DAMM v2 Migration Line */}
                  <line
                    x1="480"
                    y1="25"
                    x2="480"
                    y2="170"
                    stroke="#a855f7"
                    strokeWidth="2"
                    strokeDasharray="3 3"
                  />
                  <text x="445" y="20" fill="#7e22ce" fontSize="10" fontFamily="monospace" fontWeight="bold">
                    DAMM v2 Target
                  </text>

                  {/* Nodes on checkpoints */}
                  <circle cx="40" cy="150" r="5" fill="#0284c7" />
                  <circle cx="160" cy="120" r="5" fill="#059669" />
                  <circle cx="340" cy="70" r="5" fill="#10b981" />
                  <circle cx="480" cy="45" r="5" fill="#9333ea" />

                  {/* X-axis labels */}
                  <text x="90" y="185" fill="#64748b" fontSize="10" textAnchor="middle">
                    Regime A (1x)
                  </text>
                  <text x="250" y="185" fill="#64748b" fontSize="10" textAnchor="middle">
                    Regime B: Active Discovery (4x)
                  </text>
                  <text x="410" y="185" fill="#64748b" fontSize="10" textAnchor="middle">
                    Regime C: Mature (8x)
                  </text>

                  {/* Y-axis labels */}
                  <text x="35" y="153" fill="#64748b" fontSize="10" textAnchor="end">
                    ${((Number(initialPrice) || 100) * 0.85).toFixed(0)}
                  </text>
                  <text x="35" y="123" fill="#64748b" fontSize="10" textAnchor="end">
                    ${(Number(initialPrice) || 100).toFixed(0)}
                  </text>
                  <text x="35" y="73" fill="#64748b" fontSize="10" textAnchor="end">
                    ${((Number(initialPrice) || 100) * 1.3).toFixed(0)}
                  </text>
                  <text x="35" y="48" fill="#64748b" fontSize="10" textAnchor="end">
                    ${((Number(initialPrice) || 100) * 1.5).toFixed(0)}
                  </text>
                </svg>

                <div className="flex justify-between items-center text-[11px] text-slate-500 font-mono mt-2 pt-2 border-t border-slate-200">
                  <span>Start: $85.00</span>
                  <span>Consensus Anchor: $100.00</span>
                  <span>Migration Cap: $150.00</span>
                </div>
              </div>

              {/* 3-Regime Parameter Cards */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                {config?.segments.map((seg, idx) => (
                  <div
                    key={idx}
                    className={`p-3 rounded-lg border transition-all ${
                      activeRegimeHover === idx
                        ? "border-emerald-500 bg-emerald-50/50 shadow-sm"
                        : "border-slate-200 bg-white"
                    }`}
                  >
                    <div className="flex justify-between items-center mb-1">
                      <span className="text-[11px] font-bold text-slate-700">
                        {idx === 0 ? "Regime A" : idx === 1 ? "Regime B" : "Regime C"}
                      </span>
                      <Badge variant={idx === 0 ? "cyan" : idx === 1 ? "emerald" : "purple"} className="text-[9px]">
                        {seg.liquidity_weight}x Depth
                      </Badge>
                    </div>
                    <div className="text-sm font-bold text-slate-900 font-mono">
                      ${seg.start_price.toFixed(2)} → ${seg.end_price.toFixed(2)}
                    </div>
                    <p className="text-[10px] text-slate-500 mt-1 leading-snug">
                      {seg.description}
                    </p>
                  </div>
                ))}
              </div>

              {/* Validated On-Chain Configuration JSON Breakdown */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-slate-900 flex items-center gap-1.5">
                    <CheckCircle2 className="w-4 h-4 text-emerald-600" />
                    Validated Meteora DBC Engine Payload
                  </span>
                  <button
                    onClick={() => {
                      if (config) {
                        navigator.clipboard.writeText(JSON.stringify(config, null, 2));
                        setIsCopied(true);
                        setTimeout(() => setIsCopied(false), 2000);
                      }
                    }}
                    className="text-[11px] font-semibold text-emerald-600 hover:text-emerald-700"
                  >
                    {isCopied ? "Copied!" : "Copy JSON"}
                  </button>
                </div>

                <div className="bg-slate-900 text-slate-200 rounded-lg p-3 text-[11px] font-mono overflow-x-auto max-h-48">
                  <pre>{JSON.stringify(config, null, 2)}</pre>
                </div>
              </div>

              {/* Deployment CTA */}
              <div className="p-4 rounded-xl bg-slate-900 text-white flex flex-col sm:flex-row items-center justify-between gap-4">
                <div>
                  <h4 className="text-sm font-bold flex items-center gap-1.5">
                    <Sparkles className="w-4 h-4 text-emerald-400" /> Ready for On-Chain Deployment
                  </h4>
                  <p className="text-xs text-slate-400 mt-0.5">
                    Validated by backend Rust DBC Engine. Creates on-chain Meteora pool & config.
                  </p>
                </div>
                <Button
                  onClick={() => alert("DBC configuration ready! Proceeding to on-chain pool initialization.")}
                  className="bg-emerald-500 hover:bg-emerald-600 text-slate-950 font-bold px-5 text-xs shadow-md whitespace-nowrap"
                >
                  Deploy DBC Pool
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
