"use client";

import React from "react";
import { ArrowDownRight, ArrowUpRight, RefreshCw } from "lucide-react";
import { PortfolioModel } from "@equity-catalyst/sdk";

interface Props {
  positions: PortfolioModel[];
  onRefresh?: () => void;
  isRefreshing?: boolean;
}

export function PositionTable({ positions, onRefresh, isRefreshing }: Props) {
  if (!positions || positions.length === 0) {
    return (
      <div className="glass-panel rounded-xl p-8 text-center">
        <p className="text-sm text-slate-400">No active portfolio positions found for this vault.</p>
      </div>
    );
  }

  return (
    <div className="glass-panel rounded-xl overflow-hidden border border-slate-800">
      {/* Header */}
      <div className="px-5 py-4 border-b border-slate-800 flex items-center justify-between">
        <div>
          <h3 className="text-sm font-bold text-slate-200">Portfolio Allocations & Live Oracle Valuations</h3>
          <p className="text-xs text-slate-400 mt-0.5">Asset weights benchmarked against Pyth real-time price feeds</p>
        </div>

        {onRefresh && (
          <button
            onClick={onRefresh}
            disabled={isRefreshing}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-slate-800/80 hover:bg-slate-700 text-xs font-medium text-slate-200 transition-colors"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isRefreshing ? "animate-spin text-emerald-400" : ""}`} />
            <span>{isRefreshing ? "Updating..." : "Sync Pyth Prices"}</span>
          </button>
        )}
      </div>

      {/* Table */}
      <div className="overflow-x-auto">
        <table className="w-full text-left text-xs">
          <thead className="bg-slate-950/60 text-slate-400 uppercase font-mono tracking-wider border-b border-slate-800/60">
            <tr>
              <th className="px-5 py-3">Asset</th>
              <th className="px-5 py-3">Amount</th>
              <th className="px-5 py-3">Entry Price</th>
              <th className="px-5 py-3">Current Price</th>
              <th className="px-5 py-3">Total Value</th>
              <th className="px-5 py-3">Current vs Target Weight</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/60 font-medium">
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
                <tr key={pos.portfolio_id} className="hover:bg-slate-800/30 transition-colors">
                  {/* Asset */}
                  <td className="px-5 py-3.5">
                    <div className="flex items-center gap-2">
                      <div className="w-7 h-7 rounded bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-[11px] font-bold text-emerald-400">
                        {pos.asset_symbol.slice(0, 3)}
                      </div>
                      <div>
                        <div className="font-bold text-slate-200">{pos.asset_symbol}</div>
                        <div className="text-[10px] font-mono text-slate-500 truncate max-w-[100px]">
                          {pos.asset_mint}
                        </div>
                      </div>
                    </div>
                  </td>

                  {/* Amount */}
                  <td className="px-5 py-3.5 font-mono text-slate-300">
                    {pos.amount.toLocaleString()}
                  </td>

                  {/* Entry Price */}
                  <td className="px-5 py-3.5 font-mono text-slate-400">
                    ${pos.entry_price_usd.toFixed(2)}
                  </td>

                  {/* Current Price & PnL */}
                  <td className="px-5 py-3.5 font-mono">
                    <div className="text-slate-200">${pos.current_price_usd.toFixed(2)}</div>
                    <div
                      className={`inline-flex items-center gap-0.5 text-[10px] font-semibold ${
                        isProfitable ? "text-emerald-400" : "text-rose-400"
                      }`}
                    >
                      {isProfitable ? (
                        <ArrowUpRight className="w-2.5 h-2.5" />
                      ) : (
                        <ArrowDownRight className="w-2.5 h-2.5" />
                      )}
                      <span>{Math.abs(pnlPercent).toFixed(2)}%</span>
                    </div>
                  </td>

                  {/* Total Value */}
                  <td className="px-5 py-3.5 font-mono text-slate-200 font-bold">
                    ${pos.current_value_usd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                  </td>

                  {/* Weight Progress */}
                  <td className="px-5 py-3.5">
                    <div className="space-y-1 w-36">
                      <div className="flex justify-between text-[11px] font-mono">
                        <span className="text-slate-200 font-bold">{currentWeightPct}%</span>
                        <span className="text-slate-400">Target: {targetWeightPct}%</span>
                      </div>
                      <div className="h-1.5 w-full rounded-full bg-slate-800 overflow-hidden relative">
                        <div
                          className={`h-full rounded-full transition-all duration-300 ${
                            Math.abs(driftBps) > 200 ? "bg-amber-400" : "bg-emerald-400"
                          }`}
                          style={{ width: `${Math.min(pos.current_weight_bps / 100, 100)}%` }}
                        />
                      </div>
                      <div className="text-[10px] font-mono text-slate-500 text-right">
                        Drift: {driftBps > 0 ? `+${driftBps}` : driftBps} bps
                      </div>
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
