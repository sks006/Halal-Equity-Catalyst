"use client";

import React from "react";
import { AlertCircle } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Button } from "@/components/ui/button";

export interface HoldingItem {
  symbol: string;
  name: string;
  units: number;
  priceUsd: number | null;
  valueUsd: number | null;
  currentWeightPct: number;
  targetWeightPct?: number;
}

export interface HoldingsTableProps {
  holdings: HoldingItem[];
  isLoading?: boolean;
  error?: string | null;
  onTrade?: (symbol: string) => void;
}

/**
 * HoldingsTable answers one fundamental question:
 * "What do I own?"
 *
 * Adheres to:
 * - Simple table for desktop, cards for mobile
 * - Restrained typography and minimal badges
 * - "Trade" action button per row
 * - No fake or invented values
 */
export function HoldingsTable({
  holdings,
  isLoading = false,
  error = null,
  onTrade,
}: HoldingsTableProps) {
  return (
    <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
      <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
        <CardTitle className="text-base font-bold text-slate-900">
          Holdings
        </CardTitle>
        <span className="text-xs text-slate-500 font-medium">
          {holdings.length} {holdings.length === 1 ? "asset" : "assets"}
        </span>
      </CardHeader>

      <CardContent className="p-0">
        {isLoading && holdings.length === 0 ? (
          <div className="p-8 text-center text-slate-400 text-sm">
            Loading your holdings...
          </div>
        ) : error ? (
          <div className="p-8 flex items-center justify-center gap-2 text-slate-600 text-sm">
            <AlertCircle className="w-5 h-5 text-amber-500 shrink-0" />
            <span>Unable to load live data</span>
          </div>
        ) : holdings.length === 0 ? (
          <div className="p-8 text-center text-slate-500 text-sm">
            No holdings found. Deposit funds or trade to build your portfolio.
          </div>
        ) : (
          <>
            {/* Desktop Table View */}
            <div className="hidden md:block overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow className="bg-slate-50/75 border-b border-slate-100">
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600">Asset</TableHead>
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Holdings</TableHead>
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Price</TableHead>
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Value</TableHead>
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Allocation</TableHead>
                    <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Action</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {holdings.map((item) => (
                    <TableRow key={item.symbol} className="hover:bg-slate-50/50 transition-colors border-b border-slate-100">
                      <TableCell className="px-6 py-3.5">
                        <div className="flex items-center gap-2.5">
                          <div className="w-8 h-8 rounded-lg bg-slate-100 border border-slate-200 flex items-center justify-center font-bold text-xs text-slate-800">
                            {item.symbol.slice(0, 3)}
                          </div>
                          <div>
                            <div className="font-semibold text-slate-900 text-sm">{item.symbol}</div>
                            <div className="text-xs text-slate-400">{item.name}</div>
                          </div>
                        </div>
                      </TableCell>
                      <TableCell className="px-6 py-3.5 text-right text-sm text-slate-700">
                        {item.units.toLocaleString(undefined, { maximumFractionDigits: 4 })}
                      </TableCell>
                      <TableCell className="px-6 py-3.5 text-right text-sm text-slate-700">
                        {item.priceUsd !== null && item.priceUsd > 0 ? (
                          `$${item.priceUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                        ) : (
                          <span className="text-slate-400">Price unavailable</span>
                        )}
                      </TableCell>
                      <TableCell className="px-6 py-3.5 text-right text-sm font-semibold text-slate-900">
                        {item.valueUsd !== null && item.valueUsd > 0 ? (
                          `$${item.valueUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                        ) : (
                          <span className="text-slate-400 font-normal">Value unavailable</span>
                        )}
                      </TableCell>
                      <TableCell className="px-6 py-3.5 text-right text-xs">
                        <span className="font-semibold text-slate-900">{item.currentWeightPct.toFixed(1)}%</span>
                        {item.targetWeightPct !== undefined && (
                          <span className="text-slate-400 ml-1">/ {item.targetWeightPct.toFixed(0)}% target</span>
                        )}
                      </TableCell>
                      <TableCell className="px-6 py-3.5 text-right">
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => onTrade?.(item.symbol)}
                          className="h-8 px-3 text-xs font-medium text-emerald-700 border-emerald-200 hover:bg-emerald-50"
                        >
                          Trade
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>

            {/* Mobile Card View */}
            <div className="md:hidden divide-y divide-slate-100">
              {holdings.map((item) => (
                <div key={item.symbol} className="p-4 space-y-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <div className="w-8 h-8 rounded-lg bg-slate-100 border border-slate-200 flex items-center justify-center font-bold text-xs text-slate-800">
                        {item.symbol.slice(0, 3)}
                      </div>
                      <div>
                        <div className="font-semibold text-slate-900 text-sm">{item.symbol}</div>
                        <div className="text-xs text-slate-400">{item.name}</div>
                      </div>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => onTrade?.(item.symbol)}
                      className="h-8 px-3 text-xs font-medium text-emerald-700 border-emerald-200 hover:bg-emerald-50"
                    >
                      Trade
                    </Button>
                  </div>

                  <div className="grid grid-cols-2 gap-2 text-xs pt-1">
                    <div className="bg-slate-50 p-2 rounded-lg">
                      <div className="text-xs text-slate-500">Holdings</div>
                      <div className="font-semibold text-slate-800 mt-0.5">
                        {item.units.toLocaleString(undefined, { maximumFractionDigits: 4 })}
                      </div>
                    </div>
                    <div className="bg-slate-50 p-2 rounded-lg">
                      <div className="text-xs text-slate-500">Price</div>
                      <div className="font-semibold text-slate-800 mt-0.5">
                        {item.priceUsd !== null && item.priceUsd > 0 ? `$${item.priceUsd.toFixed(2)}` : "Price unavailable"}
                      </div>
                    </div>
                    <div className="bg-slate-50 p-2 rounded-lg">
                      <div className="text-xs text-slate-500">Total Value</div>
                      <div className="font-bold text-slate-900 mt-0.5">
                        {item.valueUsd !== null && item.valueUsd > 0
                          ? `$${item.valueUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                          : "Value unavailable"}
                      </div>
                    </div>
                    <div className="bg-slate-50 p-2 rounded-lg">
                      <div className="text-xs text-slate-500">Allocation</div>
                      <div className="font-semibold text-slate-800 mt-0.5">
                        {item.currentWeightPct.toFixed(1)}%
                        {item.targetWeightPct !== undefined && ` (${item.targetWeightPct}% target)`}
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
