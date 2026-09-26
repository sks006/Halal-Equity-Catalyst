"use client";

import React, { useState } from "react";
import { Search, AlertCircle, ArrowUpRight, ArrowDownRight } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

export interface MarketAsset {
  symbol: string;
  name: string;
  priceUsd: number | null;
  change24hPct?: number | null;
  isLive: boolean; // Confirmed by backend
  volume24hUsd?: number | null;
  issuer?: string;
}

export interface MarketListProps {
  assets: MarketAsset[];
  isLoading?: boolean;
  error?: string | null;
  onTrade?: (symbol: string) => void;
  title?: string;
  subtitle?: string;
  showSearch?: boolean;
}

/**
 * MarketList answers one fundamental question:
 * "What can I trade?"
 *
 * Adheres to:
 * - Simple table for desktop, cards for mobile
 * - "Live" badge ONLY when confirmed by backend
 * - Fails closed: shows "Unable to load live data" if disconnected
 * - Clean white card, slate typography, emerald accents
 */
export function MarketList({
  assets,
  isLoading = false,
  error = null,
  onTrade,
  title = "Markets",
  subtitle = "Available assets for spot trading and rebalancing",
  showSearch = true,
}: MarketListProps) {
  const [search, setSearch] = useState("");

  const filtered = assets.filter((a) => {
    if (!search.trim()) return true;
    const term = search.toLowerCase();
    return (
      a.symbol.toLowerCase().includes(term) ||
      a.name.toLowerCase().includes(term) ||
      (a.issuer && a.issuer.toLowerCase().includes(term))
    );
  });

  return (
    <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
      <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div>
          <CardTitle className="text-base font-bold text-slate-900">{title}</CardTitle>
          {subtitle && <p className="text-xs text-slate-500 mt-0.5">{subtitle}</p>}
        </div>

        {showSearch && (
          <div className="relative w-full sm:w-64">
            <Search className="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <Input
              type="text"
              placeholder="Search markets..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9 h-9 text-xs bg-slate-50 border-slate-200 focus:bg-white"
            />
          </div>
        )}
      </CardHeader>

      <CardContent className="p-0">
        {isLoading && assets.length === 0 ? (
          <div className="p-8 text-center text-slate-400 text-sm">
            Loading live markets...
          </div>
        ) : error ? (
          <div className="p-8 flex items-center justify-center gap-2 text-slate-600 text-sm">
            <AlertCircle className="w-5 h-5 text-amber-500 shrink-0" />
            <span>Unable to load live data</span>
          </div>
        ) : filtered.length === 0 ? (
          <div className="p-8 text-center text-slate-500 text-sm">
            {search ? "No assets match your search." : "No tradeable assets available."}
          </div>
        ) : (
          <>
            {/* Desktop Table View */}
            <div className="hidden md:block overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow className="bg-slate-50/75 border-b border-slate-100">
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600">Asset</TableHead>
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Price</TableHead>
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">24h Change</TableHead>
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-center">Status</TableHead>
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Action</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filtered.map((asset) => {
                    const hasPrice = asset.priceUsd !== null;
                    const isPositive = (asset.change24hPct ?? 0) >= 0;

                    return (
                      <TableRow key={asset.symbol} className="hover:bg-slate-50/50 transition-colors border-b border-slate-100">
                        <TableCell className="px-6 py-3.5">
                          <div className="flex items-center gap-2.5">
                            <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-100 flex items-center justify-center font-bold text-xs text-emerald-800">
                              {asset.symbol.slice(0, 3)}
                            </div>
                            <div>
                              <div className="font-semibold text-slate-900 text-sm">{asset.symbol}</div>
                              <div className="text-xs text-slate-400">{asset.name}</div>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="px-6 py-3.5 text-right text-sm font-semibold text-slate-900">
                          {hasPrice ? (
                            `$${asset.priceUsd!.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                          ) : (
                            <span className="text-slate-400 text-xs">Price unavailable</span>
                          )}
                        </TableCell>
                        <TableCell className="px-6 py-3.5 text-right text-xs">
                          {asset.change24hPct !== null && asset.change24hPct !== undefined ? (
                            <span
                              className={`inline-flex items-center font-semibold ${
                                isPositive ? "text-emerald-700" : "text-rose-700"
                              }`}
                            >
                              {isPositive ? (
                                <ArrowUpRight className="w-3.5 h-3.5 mr-0.5" />
                              ) : (
                                <ArrowDownRight className="w-3.5 h-3.5 mr-0.5" />
                              )}
                              {isPositive ? "+" : ""}{asset.change24hPct.toFixed(2)}%
                            </span>
                          ) : (
                            <span className="text-slate-400">—</span>
                          )}
                        </TableCell>
                        <TableCell className="px-6 py-3.5 text-center">
                          {asset.isLive ? (
                            <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium bg-emerald-50 text-emerald-700 border border-emerald-200">
                              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                              Live
                            </span>
                          ) : (
                            <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium bg-slate-100 text-slate-600 border border-slate-200">
                              Closed
                            </span>
                          )}
                        </TableCell>
                        <TableCell className="px-6 py-3.5 text-right">
                          <Button
                            size="sm"
                            onClick={() => onTrade?.(asset.symbol)}
                            className="h-8 px-3 text-xs font-semibold bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg shadow-none"
                          >
                            Buy / Sell
                          </Button>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>

            {/* Mobile Card View */}
            <div className="md:hidden divide-y divide-slate-100">
              {filtered.map((asset) => {
                const hasPrice = asset.priceUsd !== null;
                const isPositive = (asset.change24hPct ?? 0) >= 0;

                return (
                  <div key={asset.symbol} className="p-4 space-y-3">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2.5">
                        <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-100 flex items-center justify-center font-bold text-xs text-emerald-800">
                          {asset.symbol.slice(0, 3)}
                        </div>
                        <div>
                          <div className="font-semibold text-slate-900 text-sm">{asset.symbol}</div>
                          <div className="text-xs text-slate-400">{asset.name}</div>
                        </div>
                      </div>
                      <Button
                        size="sm"
                        onClick={() => onTrade?.(asset.symbol)}
                        className="h-8 px-3 text-xs font-semibold bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg shadow-none"
                      >
                        Trade
                      </Button>
                    </div>

                    <div className="flex items-center justify-between pt-1 text-xs">
                      <div>
                        <span className="text-slate-500 text-xs block">Price</span>
                        <span className="font-bold text-slate-900 text-sm">
                          {hasPrice ? (
                            `$${asset.priceUsd!.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                          ) : (
                            <span className="text-slate-400 text-xs">Price unavailable</span>
                          )}
                        </span>
                      </div>

                      <div className="text-right">
                        <span className="text-slate-500 text-xs block">24h Change</span>
                        {asset.change24hPct !== null && asset.change24hPct !== undefined ? (
                          <span
                            className={`font-semibold ${
                              isPositive ? "text-emerald-700" : "text-rose-700"
                            }`}
                          >
                            {isPositive ? "+" : ""}{asset.change24hPct.toFixed(2)}%
                          </span>
                        ) : (
                          <span className="text-slate-400">—</span>
                        )}
                      </div>

                      <div className="text-right">
                        <span className="text-slate-500 text-xs block">Feed Status</span>
                        {asset.isLive ? (
                          <span className="inline-flex items-center gap-1 text-xs font-medium text-emerald-700">
                            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                            Live
                          </span>
                        ) : (
                          <span className="text-xs font-medium text-slate-500">
                            Closed
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
