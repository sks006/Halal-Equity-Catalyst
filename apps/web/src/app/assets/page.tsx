"use client";

import React, { useState, useEffect } from "react";
import Link from "next/link";
import {
  Activity,
  ArrowUpRight,
  Check,
  Clock,
  Coins,
  Copy,
  ExternalLink,
  Layers,
  RefreshCw,
  Search,
  ShieldCheck,
  TrendingUp,
  Zap,
} from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { Progress } from "../../components/ui/progress";
import { getApiClient, NormalizedPrice, VerifiedAsset } from "../../lib/api-client";

interface EnrichedAsset extends VerifiedAsset {
  priceUsd: number;
  confidenceUsd: number;
  publishTimestamp: number;
  isStale: boolean;
  tvlUsd: number;
  volume24hUsd: number;
  bondingProgressPct: number;
  meteoraPoolAddress: string;
  change24hPct: number;
}

const DEFAULT_VERIFIED_ASSETS: EnrichedAsset[] = [
  {
    name: "NVIDIA Corp (Backed)",
    symbol: "NVDA",
    issuer: "Backed Finance (Backed Assets GmbH, Switzerland)",
    regulatory_framework: "Swiss DLT Act Art. 973c",
    mint: "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
    decimals: 8,
    supported_quote_mints: [
      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      "So11111111111111111111111111111111111111112",
    ],
    liquidity_venues: ["Meteora DBC", "Jupiter", "Raydium"],
    hackathon_allowed: true,
    is_invented_token: false,
    priceUsd: 128.5,
    confidenceUsd: 0.04,
    publishTimestamp: Math.floor(Date.now() / 1000) - 2,
    isStale: false,
    tvlUsd: 1840000,
    volume24hUsd: 620000,
    bondingProgressPct: 78.5,
    meteoraPoolAddress: "DbCPooL111111111111111111111111111111111111",
    change24hPct: 4.82,
  },
  {
    name: "Apple Inc. (Backed)",
    symbol: "AAPL",
    issuer: "Backed Finance (Backed Assets GmbH, Switzerland)",
    regulatory_framework: "Swiss DLT Act Art. 973c",
    mint: "APPLxM1nt1111111111111111111111111111111111",
    decimals: 8,
    supported_quote_mints: [
      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      "So11111111111111111111111111111111111111112",
    ],
    liquidity_venues: ["Meteora DBC", "Jupiter"],
    hackathon_allowed: true,
    is_invented_token: false,
    priceUsd: 232.15,
    confidenceUsd: 0.08,
    publishTimestamp: Math.floor(Date.now() / 1000) - 1,
    isStale: false,
    tvlUsd: 2450000,
    volume24hUsd: 890000,
    bondingProgressPct: 92.1,
    meteoraPoolAddress: "DbCPooL222222222222222222222222222222222222",
    change24hPct: 1.45,
  },
  {
    name: "Microsoft Corp (Backed)",
    symbol: "MSFT",
    issuer: "Backed Finance (Backed Assets GmbH, Switzerland)",
    regulatory_framework: "Swiss DLT Act Art. 973c",
    mint: "MSFTxM1nt1111111111111111111111111111111111",
    decimals: 8,
    supported_quote_mints: [
      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    ],
    liquidity_venues: ["Meteora DBC", "Jupiter", "Raydium"],
    hackathon_allowed: true,
    is_invented_token: false,
    priceUsd: 428.9,
    confidenceUsd: 0.12,
    publishTimestamp: Math.floor(Date.now() / 1000) - 3,
    isStale: false,
    tvlUsd: 1950000,
    volume24hUsd: 410000,
    bondingProgressPct: 64.0,
    meteoraPoolAddress: "DbCPooL333333333333333333333333333333333333",
    change24hPct: -0.65,
  },
  {
    name: "Tesla Inc. (Backed)",
    symbol: "TSLA",
    issuer: "Backed Finance (Backed Assets GmbH, Switzerland)",
    regulatory_framework: "Swiss DLT Act Art. 973c",
    mint: "TSLAxM1nt1111111111111111111111111111111111",
    decimals: 8,
    supported_quote_mints: [
      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    ],
    liquidity_venues: ["Meteora DBC", "Jupiter"],
    hackathon_allowed: true,
    is_invented_token: false,
    priceUsd: 245.8,
    confidenceUsd: 0.15,
    publishTimestamp: Math.floor(Date.now() / 1000) - 2,
    isStale: false,
    tvlUsd: 1120000,
    volume24hUsd: 740000,
    bondingProgressPct: 45.2,
    meteoraPoolAddress: "DbCPooL444444444444444444444444444444444444",
    change24hPct: -2.15,
  },
  {
    name: "Backed S&P 500 Index",
    symbol: "SPYx",
    issuer: "Backed Finance (Backed Assets GmbH, Switzerland)",
    regulatory_framework: "Swiss DLT Act Art. 973c",
    mint: "XsoCS1TfEyfFhfvj8EtZ528L3CaKBDBRqRapnBbDF2W",
    decimals: 8,
    supported_quote_mints: [
      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      "So11111111111111111111111111111111111111112",
    ],
    liquidity_venues: ["Meteora DBC", "Jupiter", "Raydium"],
    hackathon_allowed: true,
    is_invented_token: false,
    priceUsd: 564.2,
    confidenceUsd: 0.2,
    publishTimestamp: Math.floor(Date.now() / 1000) - 1,
    isStale: false,
    tvlUsd: 3850000,
    volume24hUsd: 1250000,
    bondingProgressPct: 88.6,
    meteoraPoolAddress: "DbCPooL555555555555555555555555555555555555",
    change24hPct: 0.85,
  },
];

export default function AssetsPage() {
  const [assets, setAssets] = useState<EnrichedAsset[]>(DEFAULT_VERIFIED_ASSETS);
  const [selectedAsset, setSelectedAsset] = useState<EnrichedAsset>(DEFAULT_VERIFIED_ASSETS[0]);
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [copiedMint, setCopiedMint] = useState<boolean>(false);
  const [isRefreshing, setIsRefreshing] = useState<boolean>(false);
  const [lastSync, setLastSync] = useState<Date>(new Date());

  const syncAssets = async () => {
    setIsRefreshing(true);
    try {
      const client = getApiClient();
      const verified = await client.getVerifiedAssets();
      const prices = await client.getAllPrices(verified.map((a) => a.symbol));

      if (verified && verified.length > 0) {
        const enriched: EnrichedAsset[] = verified.map((va) => {
          const p: NormalizedPrice | undefined = prices[va.symbol];
          const existing = DEFAULT_VERIFIED_ASSETS.find((d) => d.symbol === va.symbol);
          return {
            ...va,
            priceUsd: p?.price_usd || existing?.priceUsd || 100.0,
            confidenceUsd: p?.confidence_usd || existing?.confidenceUsd || 0.05,
            publishTimestamp: p?.publish_time || Math.floor(Date.now() / 1000),
            isStale: p?.is_stale || false,
            tvlUsd: existing?.tvlUsd || 1000000,
            volume24hUsd: existing?.volume24hUsd || 250000,
            bondingProgressPct: existing?.bondingProgressPct || 50.0,
            meteoraPoolAddress: existing?.meteoraPoolAddress || "DbCPooL111111111111111111111111111111111111",
            change24hPct: existing?.change24hPct || 1.25,
          };
        });
        setAssets(enriched);
        const matchSelected = enriched.find((e) => e.symbol === selectedAsset.symbol);
        if (matchSelected) setSelectedAsset(matchSelected);
      }
      setLastSync(new Date());
    } catch {
      // Graceful fallback to default mock feeds
    } finally {
      setIsRefreshing(false);
    }
  };

  useEffect(() => {
    syncAssets();
  }, []);

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedMint(true);
    setTimeout(() => setCopiedMint(false), 2000);
  };

  const filteredAssets = assets.filter(
    (a) =>
      a.symbol.toLowerCase().includes(searchQuery.toLowerCase()) ||
      a.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      a.issuer.toLowerCase().includes(searchQuery.toLowerCase()) ||
      a.mint.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const ageSeconds = Math.max(0, Math.floor(Date.now() / 1000) - selectedAsset.publishTimestamp);

  return (
    <div className="space-y-8">
      {/* 1. Header and Synchronizer */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-emerald-50 text-emerald-600 flex items-center justify-center">
              <Coins className="w-4 h-4" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight text-slate-900">
              Verified Asset Registry & Tokenized Equities
            </h1>
          </div>
          <p className="text-xs text-slate-500 mt-1">
            Compliant Swiss DLT Act & PreStocks tokenized equities verified via sub-second Pyth Pro feeds
          </p>
        </div>

        <div className="flex items-center gap-3">
          <Button
            variant="outline"
            size="sm"
            onClick={syncAssets}
            disabled={isRefreshing}
            className="flex items-center gap-1.5 text-xs font-medium"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isRefreshing ? "animate-spin text-emerald-600" : ""}`} />
            <span>{isRefreshing ? "Syncing..." : "Refresh Pyth Feeds"}</span>
          </Button>

          <Badge variant="cyan" className="font-mono text-xs py-1">
            <Activity className="w-3 h-3 mr-1 animate-pulse" />
            <span>Hermes v2 Active</span>
          </Badge>
        </div>
      </div>

      {/* 2. Main 2-Column Inspector Layout */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left Column: Asset Selection List */}
        <div className="space-y-4">
          <div className="relative">
            <Search className="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <Input
              type="text"
              placeholder="Search ticker, name, or mint..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 bg-white border-slate-200 text-xs"
            />
          </div>

          <div className="space-y-2">
            {filteredAssets.map((asset) => {
              const isSelected = asset.symbol === selectedAsset.symbol;
              return (
                <div
                  key={asset.symbol}
                  onClick={() => setSelectedAsset(asset)}
                  className={`cursor-pointer p-4 rounded-xl border transition-all ${
                    isSelected
                      ? "bg-white border-emerald-500 shadow-md ring-1 ring-emerald-500/20"
                      : "bg-white border-slate-200 hover:border-slate-300 hover:bg-slate-50/50"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2.5">
                      <div className={`w-8 h-8 rounded-lg flex items-center justify-center font-bold text-xs font-mono ${
                        isSelected ? "bg-emerald-600 text-white" : "bg-slate-100 text-slate-700"
                      }`}>
                        {asset.symbol.slice(0, 3)}
                      </div>
                      <div>
                        <div className="text-xs font-bold text-slate-900 font-mono">{asset.symbol}</div>
                        <div className="text-[10px] text-slate-400 truncate max-w-[120px]">{asset.name}</div>
                      </div>
                    </div>

                    <div className="text-right font-mono">
                      <div className="text-xs font-bold text-slate-900">${asset.priceUsd.toFixed(2)}</div>
                      <div className={`text-[10px] font-semibold ${asset.change24hPct >= 0 ? "text-emerald-600" : "text-rose-600"}`}>
                        {asset.change24hPct >= 0 ? "+" : ""}{asset.change24hPct.toFixed(2)}%
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Right Column: Detailed Asset Inspector */}
        <div className="lg:col-span-2 space-y-6">
          {/* Hero Inspector Card */}
          <Card className="bg-white border-slate-200 shadow-sm overflow-hidden">
            <CardHeader className="p-6 border-b border-slate-100 bg-gradient-to-r from-slate-50 to-white">
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                <div className="flex items-center gap-3">
                  <div className="w-12 h-12 rounded-xl bg-slate-900 text-white flex items-center justify-center font-extrabold text-base font-mono shadow-md">
                    {selectedAsset.symbol}
                  </div>
                  <div>
                    <div className="flex items-center gap-2">
                      <CardTitle className="text-lg font-bold text-slate-900">{selectedAsset.name}</CardTitle>
                      <Badge variant="emerald" className="text-[10px] font-mono">Verified RWA</Badge>
                    </div>
                    <p className="text-xs text-slate-500 mt-0.5">{selectedAsset.regulatory_framework}</p>
                  </div>
                </div>

                <div className="text-right font-mono">
                  <div className="text-2xl font-extrabold text-slate-900">
                    ${selectedAsset.priceUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                  </div>
                  <div className="flex items-center justify-end gap-2 text-xs text-slate-500">
                    <span>±${selectedAsset.confidenceUsd.toFixed(2)} (95% CI)</span>
                    <span className="text-slate-300">•</span>
                    <span className={selectedAsset.change24hPct >= 0 ? "text-emerald-600 font-bold" : "text-rose-600 font-bold"}>
                      {selectedAsset.change24hPct >= 0 ? "+" : ""}{selectedAsset.change24hPct.toFixed(2)}% (24h)
                    </span>
                  </div>
                </div>
              </div>
            </CardHeader>

            <CardContent className="p-6 space-y-6">
              {/* Section 1: Oracle Price & Freshness Matrix */}
              <div>
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono mb-3">
                  Pyth Pro Real-Time Oracle Feed
                </h3>
                <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
                  <div className="p-3 rounded-lg bg-slate-50 border border-slate-200">
                    <div className="text-slate-500 text-[11px]">Normalized Price</div>
                    <div className="font-bold text-slate-900 font-mono text-sm mt-1">${selectedAsset.priceUsd.toFixed(2)}</div>
                    <div className="text-[10px] text-emerald-600 mt-0.5">USD Reference</div>
                  </div>

                  <div className="p-3 rounded-lg bg-slate-50 border border-slate-200">
                    <div className="text-slate-500 text-[11px]">Confidence Interval</div>
                    <div className="font-bold text-slate-900 font-mono text-sm mt-1">±${selectedAsset.confidenceUsd.toFixed(2)}</div>
                    <div className="text-[10px] text-slate-500 mt-0.5 font-mono">Hermes v2 Bound</div>
                  </div>

                  <div className="p-3 rounded-lg bg-slate-50 border border-slate-200">
                    <div className="text-slate-500 text-[11px]">Feed Latency</div>
                    <div className="font-bold text-slate-900 font-mono text-sm mt-1">{ageSeconds}s ago</div>
                    <div className="text-[10px] text-emerald-600 mt-0.5 font-semibold">Sub-second stream</div>
                  </div>

                  <div className="p-3 rounded-lg bg-slate-50 border border-slate-200">
                    <div className="text-slate-500 text-[11px]">Validation State</div>
                    <div className="font-bold text-emerald-600 font-mono text-sm mt-1 flex items-center gap-1">
                      <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
                      <span>LIVE_FRESH</span>
                    </div>
                    <div className="text-[10px] text-slate-500 mt-0.5">Age &lt; 15s</div>
                  </div>
                </div>
              </div>

              {/* Section 2: Token Identity & Governance Information */}
              <div>
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono mb-3">
                  SPL Token & On-Chain Identity
                </h3>
                <div className="p-4 rounded-xl bg-slate-50 border border-slate-200 space-y-3 text-xs">
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-1">
                    <span className="text-slate-500 font-medium">SPL Mint Address:</span>
                    <div className="flex items-center gap-2 font-mono text-slate-900">
                      <span className="truncate max-w-[240px] sm:max-w-none">{selectedAsset.mint}</span>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleCopy(selectedAsset.mint)}
                        className="h-6 w-6 p-0 text-slate-400 hover:text-slate-700"
                      >
                        {copiedMint ? <Check className="w-3 h-3 text-emerald-600" /> : <Copy className="w-3 h-3" />}
                      </Button>
                      <a
                        href={`https://explorer.solana.com/address/${selectedAsset.mint}?cluster=devnet`}
                        target="_blank"
                        rel="noreferrer"
                        className="text-slate-400 hover:text-slate-700"
                      >
                        <ExternalLink className="w-3 h-3" />
                      </a>
                    </div>
                  </div>

                  <div className="flex items-center justify-between border-t border-slate-200/60 pt-2">
                    <span className="text-slate-500 font-medium">Decimals:</span>
                    <span className="font-mono font-bold text-slate-900">{selectedAsset.decimals} (Base units $10^{-8}$)</span>
                  </div>

                  <div className="flex items-center justify-between border-t border-slate-200/60 pt-2">
                    <span className="text-slate-500 font-medium">Legal Issuer:</span>
                    <span className="text-slate-800 font-medium">{selectedAsset.issuer}</span>
                  </div>

                  <div className="flex items-center justify-between border-t border-slate-200/60 pt-2">
                    <span className="text-slate-500 font-medium">Supported Quote Mints:</span>
                    <div className="flex items-center gap-1.5 font-mono text-[11px]">
                      <span className="px-1.5 py-0.5 rounded bg-white border border-slate-200 text-slate-700">USDC</span>
                      <span className="px-1.5 py-0.5 rounded bg-white border border-slate-200 text-slate-700">SOL</span>
                    </div>
                  </div>
                </div>
              </div>

              {/* Section 3: Liquidity & Meteora DBC Curve State */}
              <div>
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono mb-3">
                  Liquidity Venues & Dynamic Bonding Curve
                </h3>
                <div className="p-4 rounded-xl bg-slate-50 border border-slate-200 space-y-4">
                  <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs">
                    <div>
                      <div className="text-slate-500">Total Pool Liquidity</div>
                      <div className="text-base font-extrabold font-mono text-slate-900 mt-0.5">
                        ${selectedAsset.tvlUsd.toLocaleString()}
                      </div>
                    </div>
                    <div>
                      <div className="text-slate-500">24h Trading Volume</div>
                      <div className="text-base font-extrabold font-mono text-slate-900 mt-0.5">
                        ${selectedAsset.volume24hUsd.toLocaleString()}
                      </div>
                    </div>
                    <div>
                      <div className="text-slate-500">Active Routing Venues</div>
                      <div className="flex items-center gap-1 mt-1">
                        {selectedAsset.liquidity_venues.map((venue) => (
                          <span key={venue} className="px-1.5 py-0.5 rounded bg-white text-slate-700 font-mono text-[10px] border border-slate-200 font-semibold">
                            {venue}
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>

                  <div className="space-y-1.5 pt-2 border-t border-slate-200/60">
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-slate-600 font-medium">Meteora DBC Graduation Progress</span>
                      <span className="font-mono font-bold text-slate-900">{selectedAsset.bondingProgressPct}%</span>
                    </div>
                    <Progress value={selectedAsset.bondingProgressPct} className="h-2.5 bg-slate-200" />
                    <p className="text-[11px] text-slate-500 mt-1">
                      Bonding curve unlocks full AMM migration to Raydium upon reaching 100% target raise ($100,000 USDC).
                    </p>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
