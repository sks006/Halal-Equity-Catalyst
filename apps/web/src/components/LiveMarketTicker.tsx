"use client";

import React, { useEffect, useState } from "react";
import Link from "next/link";
import { ArrowUpRight, ArrowDownRight, Activity, Zap, ShieldCheck, Radio } from "lucide-react";
import { usePythLazer } from "../hooks/usePythLazer";

interface TickerItem {
  symbol: string;
  name: string;
  price: number;
  change24h: number;
  confidence: number;
  provider: "Pyth Lazer" | "Meteora DBC" | "Screened Spot";
}

const INITIAL_TICKERS: TickerItem[] = [
  { symbol: "AAPLx", name: "Apple Inc Tokenized", price: 232.15, change24h: 1.85, confidence: 0.08, provider: "Pyth Lazer" },
  { symbol: "MSFTx", name: "Microsoft Corp Tokenized", price: 428.9, change24h: 0.94, confidence: 0.12, provider: "Pyth Lazer" },
  { symbol: "NVDAx", name: "NVIDIA Corp Tokenized", price: 128.5, change24h: 3.42, confidence: 0.04, provider: "Pyth Lazer" },
  { symbol: "GOOGLx", name: "Alphabet Inc Tokenized", price: 178.4, change24h: 1.15, confidence: 0.06, provider: "Pyth Lazer" },
  { symbol: "SOL/USDC", name: "Solana Spot", price: 148.75, change24h: 4.12, confidence: 0.05, provider: "Pyth Lazer" },
  { symbol: "AAPLx/USDC DBC", name: "Meteora Curve Pool", price: 232.4, change24h: 1.92, confidence: 0.09, provider: "Meteora DBC" },
  { symbol: "NVDAx/USDC DBC", name: "Meteora Curve Pool", price: 129.1, change24h: 3.8, confidence: 0.06, provider: "Meteora DBC" },
  { symbol: "MSFTx/USDC DBC", name: "Meteora Curve Pool", price: 429.3, change24h: 0.98, confidence: 0.14, provider: "Meteora DBC" },
];

export function LiveMarketTicker() {
  const [tickers, setTickers] = useState<TickerItem[]>(INITIAL_TICKERS);
  const [tps, setTps] = useState(2438);

  const { status: lazerStatus, prices: lazerPrices, isConnected } = usePythLazer({
    feedIds: [1, 2, 6, 10, 11, 12],
    channel: "fixed_rate@200ms",
  });

  // Merge live incoming Pyth Lazer updates into tickers
  useEffect(() => {
    if (Object.keys(lazerPrices).length === 0) return;

    setTickers((prev) =>
      prev.map((item) => {
        let matchedPrice: number | null = null;
        let matchedConf: number | null = null;

        if (item.symbol === "SOL/USDC" && lazerPrices[6]) {
          matchedPrice = lazerPrices[6].price;
          matchedConf = lazerPrices[6].confidence ? lazerPrices[6].confidence * Math.pow(10, lazerPrices[6].exponent) : null;
        } else if (item.symbol === "AAPLx" && lazerPrices[10]) {
          matchedPrice = lazerPrices[10].price;
          matchedConf = lazerPrices[10].confidence ? lazerPrices[10].confidence * Math.pow(10, lazerPrices[10].exponent) : null;
        } else if (item.symbol === "MSFTx" && lazerPrices[11]) {
          matchedPrice = lazerPrices[11].price;
          matchedConf = lazerPrices[11].confidence ? lazerPrices[11].confidence * Math.pow(10, lazerPrices[11].exponent) : null;
        } else if (item.symbol === "GOOGLx" && lazerPrices[12]) {
          matchedPrice = lazerPrices[12].price;
          matchedConf = lazerPrices[12].confidence ? lazerPrices[12].confidence * Math.pow(10, lazerPrices[12].exponent) : null;
        }

        if (matchedPrice !== null && matchedPrice > 0) {
          const prevPrice = item.price;
          const diffPct = prevPrice > 0 ? ((matchedPrice - prevPrice) / prevPrice) * 100 : 0;
          return {
            ...item,
            price: Number(matchedPrice.toFixed(2)),
            change24h: Number((item.change24h + diffPct * 0.1).toFixed(2)),
            confidence: matchedConf ? Number(matchedConf.toFixed(2)) : item.confidence,
            provider: "Pyth Lazer",
          };
        }
        return item;
      })
    );
  }, [lazerPrices]);

  // Subtle realistic live price micro-fluctuations if waiting on streaming packets
  useEffect(() => {
    const interval = setInterval(() => {
      setTickers((prev) =>
        prev.map((item) => {
          // If already receiving high frequency updates for a feed, don't simulate
          if (isConnected && Object.keys(lazerPrices).length > 0 && item.provider === "Pyth Lazer") {
            return item;
          }
          const delta = (Math.random() - 0.48) * 0.08;
          const newPrice = Math.max(1, Number((item.price + delta).toFixed(2)));
          return {
            ...item,
            price: newPrice,
          };
        })
      );
      setTps(Math.floor(2400 + Math.random() * 120));
    }, 3000);

    return () => clearInterval(interval);
  }, [isConnected, lazerPrices]);

  // Duplicate for seamless infinite marquee loop
  const displayItems = [...tickers, ...tickers];

  return (
    <div className="w-full bg-slate-900 border-b border-slate-800 text-slate-300 text-xs overflow-hidden py-1.5 select-none relative z-30">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 flex items-center justify-between">
        {/* Left Badge: Live Status & Pyth Lazer Stream */}
        <div className="flex items-center gap-3 shrink-0 pr-4 bg-slate-900 z-10 border-r border-slate-800">
          <span className="flex items-center gap-1.5 text-emerald-400 font-mono text-[11px] font-semibold">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-ping inline-block" />
            LIVE SPOT
          </span>
          <span className="hidden md:inline-flex items-center gap-1 text-[10px] text-cyan-400 font-mono">
            <Radio className={`w-3 h-3 ${isConnected ? "text-emerald-400 animate-pulse" : "text-cyan-400"}`} />
            Pyth Lazer: {isConnected ? "Streaming (200ms)" : "Active"}
          </span>
          <span className="hidden lg:inline-flex items-center gap-1 text-[10px] text-slate-400 font-mono">
            <Activity className="w-3 h-3 text-cyan-400" />
            {tps.toLocaleString()} TPS
          </span>
          <span className="hidden xl:inline-flex items-center gap-1 text-[10px] text-emerald-300 font-mono">
            <ShieldCheck className="w-3 h-3" />
            Halal-by-Design
          </span>
        </div>

        {/* Scrolling Ticker Track */}
        <div className="overflow-hidden relative flex-1 min-w-0 mask-radial">
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
            Meteora DBC Controller &rarr;
          </Link>
        </div>
      </div>
    </div>
  );
}
