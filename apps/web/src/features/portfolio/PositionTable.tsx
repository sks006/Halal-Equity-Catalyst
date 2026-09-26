"use client";

import React from "react";
import { ArrowDownRight, ArrowUpRight, RefreshCw } from "lucide-react";
import { PortfolioModel } from "@equity-catalyst/sdk";

import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "../../components/ui/table";
import { Button } from "../../components/ui/button";
import { Badge } from "../../components/ui/badge";
import { Progress } from "../../components/ui/progress";

interface Props {
  positions: PortfolioModel[];
  onRefresh?: () => void;
  isRefreshing?: boolean;
}

export function PositionTable({ positions, onRefresh, isRefreshing }: Props) {
  if (!positions || positions.length === 0) {
    return (
      <Card className="bg-white p-8 text-center">
        <p className="text-sm text-slate-500">No active portfolio positions found for this vault.</p>
      </Card>
    );
  }

  return (
    <Card className="bg-white overflow-hidden">
      {/* Header */}
      <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
        <div>
          <CardTitle className="text-sm font-bold text-slate-900">
            Portfolio Allocations & Live Oracle Valuations
          </CardTitle>
          <p className="text-xs text-slate-500 mt-0.5">Asset weights benchmarked against Pyth real-time price feeds</p>
        </div>

        {onRefresh && (
          <Button
            variant="outline"
            size="sm"
            onClick={onRefresh}
            disabled={isRefreshing}
            className="flex items-center gap-1.5 text-xs font-medium"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isRefreshing ? "animate-spin text-emerald-600" : ""}`} />
            <span>{isRefreshing ? "Updating..." : "Sync Pyth Prices"}</span>
          </Button>
        )}
      </CardHeader>

      {/* Table */}
      <CardContent className="p-0">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="px-6 py-3 font-semibold text-slate-700">Asset</TableHead>
              <TableHead className="px-6 py-3 font-semibold text-slate-700">Amount</TableHead>
              <TableHead className="px-6 py-3 font-semibold text-slate-700">Entry Price</TableHead>
              <TableHead className="px-6 py-3 font-semibold text-slate-700">Current Price</TableHead>
              <TableHead className="px-6 py-3 font-semibold text-slate-700">Total Value</TableHead>
              <TableHead className="px-6 py-3 font-semibold text-slate-700">Target vs Actual Weight</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {positions.map((pos) => {
              const pnlPercent =
                pos.entry_price_usd > 0
                  ? ((pos.current_price_usd - pos.entry_price_usd) / pos.entry_price_usd) * 100
                  : 0;

              const isProfitable = pnlPercent >= 0;
              const currentWeightPct = (pos.current_weight_bps / 100).toFixed(2);
              const targetWeightPct = (pos.target_weight_bps / 100).toFixed(2);
              const driftBps = pos.current_weight_bps - pos.target_weight_bps;

              return (
                <TableRow key={pos.portfolio_id} className="hover:bg-slate-50/70">
                  {/* Asset */}
                  <TableCell className="px-6 py-3.5">
                    <div className="flex items-center gap-2.5">
                      <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-xs font-semibold text-emerald-700">
                        {pos.asset_symbol.slice(0, 3)}
                      </div>
                      <div>
                        <div className="font-semibold text-slate-900">{pos.asset_symbol}</div>
                        <div className="text-xs font-mono text-slate-400 truncate max-w-[120px]">
                          {pos.asset_mint}
                        </div>
                      </div>
                    </div>
                  </TableCell>

                  {/* Amount */}
                  <TableCell className="px-6 py-3.5 text-slate-700">
                    {pos.amount.toLocaleString()}
                  </TableCell>

                  {/* Entry Price */}
                  <TableCell className="px-6 py-3.5 text-slate-500">
                    {pos.entry_price_usd > 0 ? `$${pos.entry_price_usd.toFixed(2)}` : "—"}
                  </TableCell>

                  {/* Current Price & PnL */}
                  <TableCell className="px-6 py-3.5">
                    <div className="text-slate-900 font-semibold">
                      {pos.current_price_usd > 0 ? `$${pos.current_price_usd.toFixed(2)}` : <span className="text-slate-400 font-normal">Price unavailable</span>}
                    </div>
                    {pos.entry_price_usd > 0 && pos.current_price_usd > 0 ? (
                      <div
                        className={`inline-flex items-center gap-0.5 text-xs font-medium ${
                          isProfitable ? "text-emerald-600" : "text-rose-600"
                        }`}
                      >
                        {isProfitable ? (
                          <ArrowUpRight className="w-3.5 h-3.5" />
                        ) : (
                          <ArrowDownRight className="w-3.5 h-3.5" />
                        )}
                        <span>{Math.abs(pnlPercent).toFixed(2)}%</span>
                      </div>
                    ) : (
                      <div className="text-xs text-slate-400">—</div>
                    )}
                  </TableCell>

                  {/* Total Value */}
                  <TableCell className="px-6 py-3.5 text-slate-900 font-bold">
                    {pos.current_value_usd > 0 ? (
                      `$${pos.current_value_usd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                    ) : (
                      <span className="text-slate-400 font-normal">Value unavailable</span>
                    )}
                  </TableCell>

                  {/* Weight Progress */}
                  <TableCell className="px-6 py-3.5">
                    <div className="space-y-1.5 w-40">
                      <div className="flex justify-between text-xs font-medium">
                        <span className="text-slate-900 font-semibold">{currentWeightPct}%</span>
                        <span className="text-slate-500">Target: {targetWeightPct}%</span>
                      </div>
                      <Progress
                        value={Math.min(pos.current_weight_bps / 100, 100)}
                        indicatorClassName={Math.abs(driftBps) > 200 ? "bg-amber-500" : "bg-emerald-600"}
                      />
                      <div className="text-xs text-slate-500 text-right">
                        Drift: {driftBps > 0 ? `+${driftBps}` : driftBps} bps
                      </div>
                    </div>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}
