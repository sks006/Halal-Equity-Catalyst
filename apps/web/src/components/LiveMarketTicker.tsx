"use client";

import React, { useEffect, useState } from "react";
import Link from "next/link";
import { ArrowUpRight, ArrowDownRight, Activity, Zap, ShieldCheck } from "lucide-react";

interface TickerItem {
  symbol: string;
  name: string;
  price: number;
  change24h: number;
  confidence: number;
  provider: "Pyth Pro" | "Meteora DBC" | "Backed RWA";
}

const INITIAL_TICKERS: TickerItem[] = [
  { symbol: "NVDAx", name: "NVIDIA Corp RWA", price: 128.5, change24h: 3.42, confidence: 0.04, provider: "Pyth Pro" },
  { symbol: "AAPLx", name: "Apple Inc RWA", price: 232.15, change24h: 1.85, confidence: 0.08, provider: "Pyth Pro" },
  { symbol: "MSFTx", name: "Microsoft Corp RWA", price: 428.9, change24h: 0.94, confidence: 0.12, provider: "Pyth Pro" },
  { symbol: "SPYx", name: "S&P 500 ETF RWA", price: 564.2, change24h: 0.48, confidence: 0.2, provider: "Pyth Pro" },
  { symbol: "TSLAx", name: "Tesla Inc RWA", price: 245.8, change24h: -1.15, confidence: 0.15, provider: "Pyth Pro" },
  { symbol: "SOL/USDC", name: "Solana Native", price: 148.75, change24h: 4.12, confidence: 0.05, provider: "Pyth Pro" },
  { symbol: "NVDA/USDC DBC", name: "Meteora Curve Pool", price: 129.1, change24h: 3.8, confidence: 0.06, provider: "Meteora DBC" },
  { symbol: "AAPLx/USDC DBC", name: "Meteora Curve Pool", price: 232.4, change24h: 1.92, confidence: 0.09, provider: "Meteora DBC" },
];

export function LiveMarketTicker() {
  const [tickers, setTickers] = useState<TickerItem[]>(INITIAL_TICKERS);
  const [tps, setTps] = useState(2438);

  // Subtle realistic live price fluctuations to create a dynamic, living interface
  useEffect(() => {
    const interval = setInterval(() => {
      setTickers((prev) =>
        prev.map((item) => {
          const delta = (Math.random() - 0.48) * 0.08;
          const newPrice = Math.max(1, Number((item.price + delta).toFixed(2)));
          return {
            ...item,
            price: newPrice,
          };
        })
      );
      setTps(Math.floor(2400 + Math.random() * 120));
    }, 3500);

    return () => clearInterval(interval);
  }, []);

  // Duplicate for seamless infinite marquee loop
  const displayItems = [...tickers, ...tickers];

  return (
    <div className="w-full bg-slate-900 border-b border-slate-800 text-slate-300 text-xs overflow-hidden py-1.5 select-none relative z-30">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 flex items-center justify-between">
        {/* Left Badge: Live Status */}
        <div className="flex items-center gap-3 shrink-0 pr-4 bg-slate-900 z-10 border-r border-slate-800">
          <span className="flex items-center gap-1.5 text-emerald-400 font-mono text-[11px] font-semibold">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-ping inline-block" />
            LIVE MARKET
          </span>
          <span className="hidden md:inline-flex items-center gap-1 text-[10px] text-slate-400 font-mono">
            <Activity className="w-3 h-3 text-cyan-400" />
            {tps.toLocaleString()} TPS
          </span>
          <span className="hidden lg:inline-flex items-center gap-1 text-[10px] text-amber-400 font-mono">
            <ShieldCheck className="w-3 h-3" />
            AI Guard: Active
          </span>
        </div>

        {/* Scrolling Ticker Track */}
        <div className="overflow-hidden relative flex-1 mask-radial">
          <div className="animate-marquee flex items-center gap-6">
            {displayItems.map((item, idx) => {
              const isPositive = item.change24h >= 0;
              return (
                <Link
                  key={`${item.symbol}-${idx}`}
                  href={`/assets?symbol=${item.symbol.split(" ")[0]}`}
                  className="flex items-center gap-2 px-2 py-0.5 rounded hover:bg-slate-800/80 transition-colors shrink-0 group cursor-pointer"
                >
                  <span className="font-bold text-white font-mono tracking-tight group-hover:text-emerald-400 transition-colors">
                    {item.symbol}
                  </span>
                  <span className="font-mono text-slate-200">
                    ${item.price.toFixed(2)}
                  </span>
                  <span
                    className={`flex items-center text-[10px] font-semibold font-mono ${
                      isPositive ? "text-emerald-400" : "text-rose-400"
                    }`}
                  >
                    {isPositive ? (
                      <ArrowUpRight className="w-3 h-3" />
                    ) : (
                      <ArrowDownRight className="w-3 h-3" />
                    )}
                    {isPositive ? "+" : ""}
                    {item.change24h.toFixed(2)}%
                  </span>
                  <span className="text-[9px] text-slate-500 font-mono">
                    ±${item.confidence.toFixed(2)}
                  </span>
                </Link>
              );
            })}
          </div>
        </div>

        {/* Right CTA */}
        <div className="hidden sm:flex items-center gap-2 shrink-0 pl-4 bg-slate-900 z-10 border-l border-slate-800">
          <Link
            href="/markets"
            className="flex items-center gap-1 text-[11px] font-semibold text-emerald-400 hover:text-emerald-300 transition-colors font-mono"
          >
            <Zap className="w-3 h-3 fill-emerald-400" />
            DBC Curve Pools &rarr;
          </Link>
        </div>
      </div>
    </div>
  );
}
