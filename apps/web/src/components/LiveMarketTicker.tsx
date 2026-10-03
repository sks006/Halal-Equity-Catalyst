"use client";

import React, { useEffect, useState } from "react";
import { AlertCircle, RefreshCw } from "lucide-react";
import { usePythLazer } from "../hooks/usePythLazer";

import { getApiClient } from "@/lib/api-client";

interface TickerPrice {
  symbol: string;
  price: number;
}

/**
 * LiveMarketTicker (Cleaned Production Version)
 *
 * Adheres strictly to:
 * - NO INITIAL_TICKERS (zero static fake prices)
 * - NO random price micro-fluctuations or fake TPS
 * - NO permanent scrolling marquee on every page
 * - Shows a compact connection/error state when unavailable
 */
export function LiveMarketTicker() {
  const [prices, setPrices] = useState<TickerPrice[]>([]);

  const { prices: lazerPrices, isConnected } = usePythLazer({
    feedIds: [6, 10, 11, 12],
    channel: "fixed_rate@200ms",
  });

  useEffect(() => {
    if (lazerPrices && Object.keys(lazerPrices).length > 0) {
      const updated: TickerPrice[] = [];
      if (lazerPrices[10]?.price) updated.push({ symbol: "AAPL", price: lazerPrices[10].price });
      if (lazerPrices[11]?.price) updated.push({ symbol: "MSFT", price: lazerPrices[11].price });
      if (lazerPrices[12]?.price) updated.push({ symbol: "GOOGL", price: lazerPrices[12].price });
      if (lazerPrices[6]?.price) updated.push({ symbol: "SOL", price: lazerPrices[6].price });
      if (updated.length > 0) {
        setPrices(updated);
        return;
      }
    }

    // Fallback: Query backend oracle prices if Lazer WebSocket is idle or unentitled
    let isCancelled = false;
    const client = getApiClient();
    client
      .getAllPrices(["NVDA", "AAPL", "MSFT", "TSLA", "SOL"])
      .then((res) => {
        if (isCancelled) return;
        const updated: TickerPrice[] = [];
        if (res.NVDA?.price_usd) updated.push({ symbol: "NVDA", price: res.NVDA.price_usd });
        if (res.AAPL?.price_usd) updated.push({ symbol: "AAPL", price: res.AAPL.price_usd });
        if (res.MSFT?.price_usd) updated.push({ symbol: "MSFT", price: res.MSFT.price_usd });
        if (res.TSLA?.price_usd) updated.push({ symbol: "TSLA", price: res.TSLA.price_usd });
        if (res.SOL?.price_usd) updated.push({ symbol: "SOL", price: res.SOL.price_usd });
        if (updated.length > 0) {
          setPrices(updated);
        }
      })
      .catch(() => {});

    return () => {
      isCancelled = true;
    };
  }, [lazerPrices]);

  // If disconnected or no live prices received yet, show compact connection state
  if (!isConnected || prices.length === 0) {
    return (
      <div className="w-full bg-slate-100 border-b border-slate-200 text-slate-500 text-xs py-1.5 px-4 text-center flex items-center justify-center gap-2 font-mono">
        <AlertCircle className="w-3.5 h-3.5 text-slate-400" />
        <span>Live market feeds available on /markets.</span>
      </div>
    );
  }

  return (
    <div className="w-full bg-white border-b border-slate-200 text-slate-700 text-xs py-1.5 px-4">
      <div className="max-w-7xl mx-auto flex items-center justify-between font-mono">
        <div className="flex items-center gap-4">
          <span className="flex items-center gap-1.5 text-emerald-700 font-semibold text-[11px]">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
            Live Spot
          </span>
          <div className="flex items-center gap-4 text-xs">
            {prices.map((p) => (
              <span key={p.symbol} className="flex items-center gap-1">
                <span className="font-bold text-slate-900">{p.symbol}</span>
                <span className="text-slate-600">${p.price.toFixed(2)}</span>
              </span>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
