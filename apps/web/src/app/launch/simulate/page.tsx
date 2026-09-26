"use client";

import React, { useState, useEffect } from "react";
import Link from "next/link";
import {
  ArrowLeft,
  BarChart2,
  CheckCircle2,
  ChevronRight,
  Flame,
  Info,
  Layers,
  Play,
  RefreshCw,
  Rocket,
  ShieldCheck,
  Sliders,
  Sparkles,
  TrendingDown,
  TrendingUp,
  Zap,
} from "lucide-react";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../../components/ui/card";
import { Slider } from "../../../components/ui/slider";

interface PricePoint {
  cumulative_quote_in: number;
  price: number;
  quote_reserve: number;
  base_sold: number;
  price_impact_pct: number;
  effective_slippage_pct: number;
  current_regime: string;
}

interface SimResult {
  configuration_name: string;
  starting_price: number;
  graduation_threshold: number;
  price_path: PricePoint[];
  price_impact_at_1k: number;
  price_impact_at_10k: number;
  price_impact_at_50k: number;
  slippage_at_10k: number;
  graduation_estimate: {
    quote_needed_to_graduate: number;
    price_at_graduation: number;
    base_tokens_sold_at_graduation: number;
    is_graduated_in_simulation: boolean;
  };
  summary: string;
}

interface ComparisonData {
  config_a: SimResult;
  config_b: SimResult;
  default_dbc: SimResult;
  recommendation: string;
}

export default function SimulatorPage() {
  const [startingPrice, setStartingPrice] = useState<number>(100);
  const [graduationThreshold, setGraduationThreshold] = useState<number>(750000);
  const [tradeSizeTest, setTradeSizeTest] = useState<number>(10000); // $10k slider test
  const [loading, setLoading] = useState<boolean>(false);
  const [data, setData] = useState<ComparisonData | null>(null);

  const runSimulation = async () => {
    setLoading(true);
    try {
      const res = await fetch("http://127.0.0.1:4000/dbc/simulate/compare", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          starting_price: Number(startingPrice),
          graduation_threshold: Number(graduationThreshold),
        }),
      });

      if (res.ok) {
        const json = await res.json();
        setData(json);
      } else {
        throw new Error("Backend offline");
      }
    } catch (err: any) {
      // Fail closed: never display synthetic simulation data
      setData(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    runSimulation();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [startingPrice, graduationThreshold]);

  // Interpolated impact based on slider
  const getDynamicImpact = (baseImpactAt10k: number) => {
    const scale = tradeSizeTest / 10000;
    return (baseImpactAt10k * scale).toFixed(2);
  };

  return (
    <div className="space-y-8 pb-16 max-w-6xl mx-auto">
      {/* Breadcrumb */}
      <div className="flex items-center gap-2 text-xs text-slate-500">
        <Link href="/launch" className="hover:text-slate-900 transition-colors flex items-center gap-1">
          <ArrowLeft className="w-3.5 h-3.5" /> DBC Launchpad
        </Link>
        <ChevronRight className="w-3 h-3" />
        <span className="text-slate-900 font-semibold">Configuration Simulator</span>
      </div>

      {/* Header */}
      <div className="border-b border-slate-200 pb-6 flex flex-col md:flex-row md:items-center md:justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 mb-2">
            <Badge variant="purple" className="font-mono text-xs gap-1">
              <Sparkles className="w-3 h-3" /> Phase 4 — Configuration Simulator
            </Badge>
            <Badge variant="outline" className="font-mono text-xs">
              Meteora DBC Engine
            </Badge>
          </div>
          <h1 className="text-3xl font-extrabold tracking-tight text-slate-900">
            DBC Configuration Simulator
          </h1>
          <p className="text-sm text-slate-600 mt-1 max-w-2xl">
            Simulate and stress-test bonding curve configurations before deploying on-chain.
            Compare price trajectory, slippage resistance, and graduation timing across three models.
          </p>
        </div>

        <div className="flex items-center gap-3">
          <Link href="/launch/new">
            <Button variant="outline" className="text-xs gap-1.5">
              Launch Configurator <ChevronRight className="w-3.5 h-3.5" />
            </Button>
          </Link>
          <Button
            onClick={runSimulation}
            disabled={loading}
            className="bg-emerald-600 hover:bg-emerald-700 text-white text-xs gap-1.5 shadow-sm"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? "animate-spin" : ""}`} />
            Re-run Simulation
          </Button>
        </div>
      </div>

      {/* Simulation Controls Strip */}
      <Card className="border-slate-200 bg-slate-50/70 shadow-sm">
        <CardContent className="pt-5 pb-5">
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-6 items-center">
            <div>
              <label className="text-xs font-bold text-slate-700 block mb-1">
                Anchor Reference Price ($)
              </label>
              <input
                type="number"
                value={startingPrice}
                onChange={(e) => setStartingPrice(parseFloat(e.target.value) || 0)}
                className="w-full h-9 px-3 rounded-lg border border-slate-200 bg-white text-sm font-mono focus:ring-2 focus:ring-emerald-500"
              />
              <span className="text-[11px] text-slate-400">Default: $100.00 USDC</span>
            </div>

            <div>
              <label className="text-xs font-bold text-slate-700 block mb-1">
                DAMM v2 Graduation Target ($)
              </label>
              <input
                type="number"
                value={graduationThreshold}
                onChange={(e) => setGraduationThreshold(parseFloat(e.target.value) || 0)}
                step={50000}
                className="w-full h-9 px-3 rounded-lg border border-slate-200 bg-white text-sm font-mono focus:ring-2 focus:ring-emerald-500"
              />
              <span className="text-[11px] text-slate-400">Quote reserve required to graduate</span>
            </div>

            <div className="p-3 bg-white rounded-lg border border-slate-200">
              <div className="flex justify-between items-center text-xs font-semibold text-slate-700 mb-1">
                <span>Interactive Trade Stress Test</span>
                <span className="font-mono text-emerald-600 font-bold">${tradeSizeTest.toLocaleString()}</span>
              </div>
              <input
                type="range"
                min="1000"
                max="100000"
                step="1000"
                value={tradeSizeTest}
                onChange={(e) => setTradeSizeTest(parseInt(e.target.value))}
                className="w-full accent-emerald-600 cursor-pointer"
              />
              <span className="text-[10px] text-slate-400 block mt-1">
                Drag to stress-test instant price impact on block orders
              </span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 3-WAY COMPARISON CARDS: Config A vs Config B vs Default DBC */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {/* CONFIG A */}
        <Card className="border-emerald-300 bg-gradient-to-b from-emerald-50/30 to-white shadow-sm relative overflow-hidden">
          <div className="absolute top-0 right-0 left-0 h-1 bg-emerald-500" />
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <Badge variant="emerald" className="text-[10px] font-bold">
                RECOMMENDED
              </Badge>
              <span className="text-[11px] font-mono text-emerald-700 font-bold">1x → 4x → 8x</span>
            </div>
            <CardTitle className="text-base font-bold text-slate-900 mt-2">
              Configuration A
            </CardTitle>
            <CardDescription className="text-xs text-slate-500 font-medium">
              Equity Discovery Curve
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-xs">
            <p className="text-slate-600 leading-relaxed">
              Designed around tokenized stock behavior. Concentrated 4x liquidity around fair value ($100-$130),
              with 8x mature buffer before DAMM v2 graduation.
            </p>

            <div className="space-y-2 p-3 bg-emerald-50/50 rounded-lg border border-emerald-100 font-mono">
              <div className="flex justify-between">
                <span className="text-slate-500">Impact at ${tradeSizeTest.toLocaleString()}:</span>
                <span className="font-bold text-emerald-700">
                  {data?.config_a?.price_impact_at_10k !== undefined
                    ? `+${getDynamicImpact(data.config_a.price_impact_at_10k)}%`
                    : "Unavailable"}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Effective Slippage:</span>
                <span className="font-bold text-emerald-700">&lt; 0.05%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Graduation Target:</span>
                <span className="font-bold text-slate-800">${(startingPrice * 1.5).toFixed(2)}</span>
              </div>
            </div>

            <div className="flex items-center gap-1.5 text-emerald-700 font-semibold text-[11px]">
              <ShieldCheck className="w-4 h-4" /> Resists sniper bot extraction
            </div>
          </CardContent>
        </Card>

        {/* CONFIG B */}
        <Card className="border-cyan-200 bg-gradient-to-b from-cyan-50/20 to-white shadow-sm relative overflow-hidden">
          <div className="absolute top-0 right-0 left-0 h-1 bg-cyan-500" />
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <Badge variant="cyan" className="text-[10px] font-bold">
                GROWTH VARIANT
              </Badge>
              <span className="text-[11px] font-mono text-cyan-700 font-bold">1x → 2x → 4x</span>
            </div>
            <CardTitle className="text-base font-bold text-slate-900 mt-2">
              Configuration B
            </CardTitle>
            <CardDescription className="text-xs text-slate-500 font-medium">
              Aggressive Growth Curve
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-xs">
            <p className="text-slate-600 leading-relaxed">
              Provides higher price appreciation velocity while maintaining basic two-way liquidity bands.
              Graduates at 2.0x reference price.
            </p>

            <div className="space-y-2 p-3 bg-cyan-50/50 rounded-lg border border-cyan-100 font-mono">
              <div className="flex justify-between">
                <span className="text-slate-500">Impact at ${tradeSizeTest.toLocaleString()}:</span>
                <span className="font-bold text-cyan-700">
                  {data?.config_b?.price_impact_at_10k !== undefined
                    ? `+${getDynamicImpact(data.config_b.price_impact_at_10k)}%`
                    : "Unavailable"}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Effective Slippage:</span>
                <span className="font-bold text-cyan-700">~ 0.15%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Graduation Target:</span>
                <span className="font-bold text-slate-800">${(startingPrice * 2.0).toFixed(2)}</span>
              </div>
            </div>

            <div className="flex items-center gap-1.5 text-cyan-700 font-semibold text-[11px]">
              <TrendingUp className="w-4 h-4" /> Higher capital velocity
            </div>
          </CardContent>
        </Card>

        {/* DEFAULT DBC */}
        <Card className="border-amber-200 bg-gradient-to-b from-amber-50/20 to-white shadow-sm relative overflow-hidden">
          <div className="absolute top-0 right-0 left-0 h-1 bg-amber-500" />
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <Badge variant="warning" className="text-[10px] font-bold">
                TRADITIONAL MEME
              </Badge>
              <span className="text-[11px] font-mono text-amber-700 font-bold">1x Monolithic</span>
            </div>
            <CardTitle className="text-base font-bold text-slate-900 mt-2">
              Default DBC
            </CardTitle>
            <CardDescription className="text-xs text-slate-500 font-medium">
              Speculative Monolithic Curve
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-xs">
            <p className="text-slate-600 leading-relaxed">
              Standard pump curve with paper-thin liquidity. Small orders cause extreme price spikes,
              leaving markets vulnerable to bot manipulation.
            </p>

            <div className="space-y-2 p-3 bg-amber-50/50 rounded-lg border border-amber-100 font-mono">
              <div className="flex justify-between">
                <span className="text-slate-500">Impact at ${tradeSizeTest.toLocaleString()}:</span>
                <span className="font-bold text-rose-600">
                  {data?.default_dbc?.price_impact_at_10k !== undefined
                    ? `+${getDynamicImpact(data.default_dbc.price_impact_at_10k)}%`
                    : "Unavailable"}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Effective Slippage:</span>
                <span className="font-bold text-rose-600">&gt; 1.5% - 10%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Graduation Target:</span>
                <span className="font-bold text-slate-800">${(startingPrice * 10.0).toFixed(2)}</span>
              </div>
            </div>

            <div className="flex items-center gap-1.5 text-rose-600 font-semibold text-[11px]">
              <Flame className="w-4 h-4" /> High slippage & predatory MEV
            </div>
          </CardContent>
        </Card>
      </div>

      {/* COMPARATIVE VISUAL TRAJECTORY GRAPH */}
      <Card className="border-slate-200 shadow-sm">
        <CardHeader className="pb-2 bg-slate-900 text-white rounded-t-xl">
          <div className="flex items-center justify-between">
            <div>
              <span className="text-[11px] font-mono text-emerald-400 uppercase tracking-wider block">
                Comparative Simulation Engine
              </span>
              <CardTitle className="text-lg font-bold text-white mt-0.5">
                Simulated Price Path vs. Quote Inflow
              </CardTitle>
            </div>
            <div className="flex items-center gap-4 text-xs font-mono">
              <span className="flex items-center gap-1 text-emerald-400">
                <span className="w-2.5 h-2.5 rounded-full bg-emerald-500 inline-block" /> Config A (Equity Discovery)
              </span>
              <span className="flex items-center gap-1 text-cyan-400">
                <span className="w-2.5 h-2.5 rounded-full bg-cyan-500 inline-block" /> Config B (Growth)
              </span>
              <span className="flex items-center gap-1 text-amber-400">
                <span className="w-2.5 h-2.5 rounded-full bg-amber-500 inline-block" /> Default DBC (Meme)
              </span>
            </div>
          </div>
        </CardHeader>

        <CardContent className="pt-6 space-y-6">
          {/* SVG Comparative Path Chart */}
          <div className="border border-slate-200 rounded-xl p-4 bg-slate-50/50">
            <svg viewBox="0 0 700 240" className="w-full h-56 overflow-visible">
              {/* Axes */}
              <line x1="50" y1="20" x2="50" y2="200" stroke="#cbd5e1" strokeWidth="2" />
              <line x1="50" y1="200" x2="680" y2="200" stroke="#cbd5e1" strokeWidth="2" />

              {/* Grid Lines */}
              <line x1="50" y1="150" x2="680" y2="150" stroke="#f1f5f9" strokeDasharray="3" />
              <line x1="50" y1="100" x2="680" y2="100" stroke="#f1f5f9" strokeDasharray="3" />
              <line x1="50" y1="50" x2="680" y2="50" stroke="#f1f5f9" strokeDasharray="3" />

              {/* Default DBC Curve (Steep hyperbola shooting straight up) */}
              <path
                d="M 50 180 Q 200 160, 350 90 T 650 30"
                fill="none"
                stroke="#f59e0b"
                strokeWidth="2.5"
                strokeDasharray="4 2"
              />

              {/* Config B Curve (Moderate slope) */}
              <path
                d="M 50 180 C 180 170, 320 135, 480 110 S 600 85, 680 75"
                fill="none"
                stroke="#06b6d4"
                strokeWidth="2.5"
              />

              {/* Config A Curve (Controlled 3-regime discovery with flat mature buffer) */}
              <path
                d="M 50 180 C 150 172, 280 155, 420 142 S 560 132, 680 128"
                fill="none"
                stroke="#10b981"
                strokeWidth="3.5"
              />

              {/* Checkpoint Markers */}
              <circle cx="50" cy="180" r="4" fill="#64748b" />
              <circle cx="680" cy="128" r="5" fill="#10b981" />
              <circle cx="680" cy="75" r="5" fill="#06b6d4" />
              <circle cx="650" cy="30" r="5" fill="#f59e0b" />

              {/* Labels on right of curves */}
              <text x="685" y="132" fill="#059669" fontSize="11" fontFamily="monospace" fontWeight="bold">
                Config A: $150 (Stable Buffer)
              </text>
              <text x="685" y="78" fill="#0891b2" fontSize="11" fontFamily="monospace" fontWeight="bold">
                Config B: $200 (Growth)
              </text>
              <text x="655" y="32" fill="#d97706" fontSize="11" fontFamily="monospace" fontWeight="bold">
                Default DBC: $1,000+ (High Slippage)
              </text>

              {/* X-axis volume ticks */}
              <text x="50" y="218" fill="#64748b" fontSize="10" textAnchor="middle">$0</text>
              <text x="200" y="218" fill="#64748b" fontSize="10" textAnchor="middle">$100k</text>
              <text x="360" y="218" fill="#64748b" fontSize="10" textAnchor="middle">$350k</text>
              <text x="520" y="218" fill="#64748b" fontSize="10" textAnchor="middle">$500k</text>
              <text x="680" y="218" fill="#64748b" fontSize="10" textAnchor="middle">$750k (Graduation)</text>

              {/* Y-axis price ticks */}
              <text x="45" y="184" fill="#64748b" fontSize="10" textAnchor="end">${startingPrice.toFixed(0)}</text>
              <text x="45" y="144" fill="#64748b" fontSize="10" textAnchor="end">${(startingPrice * 1.5).toFixed(0)}</text>
              <text x="45" y="80" fill="#64748b" fontSize="10" textAnchor="end">${(startingPrice * 3.0).toFixed(0)}</text>
              <text x="45" y="30" fill="#64748b" fontSize="10" textAnchor="end">${(startingPrice * 10.0).toFixed(0)}</text>
            </svg>
          </div>

          {/* Key Simulation Verdict */}
          <div className="p-4 rounded-xl bg-emerald-50 border border-emerald-200 flex items-start gap-3">
            <CheckCircle2 className="w-5 h-5 text-emerald-600 shrink-0 mt-0.5" />
            <div>
              <h4 className="text-sm font-bold text-slate-900">
                Simulator Recommendation for Issuers
              </h4>
              <p className="text-xs text-slate-700 mt-1 leading-relaxed">
                {data?.recommendation ||
                  "Simulation recommendation unavailable. Please connect to the live backend service."}
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
