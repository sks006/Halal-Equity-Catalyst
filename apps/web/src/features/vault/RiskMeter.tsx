"use client";

import React from "react";
import { AlertTriangle, CheckCircle2, Shield, ShieldAlert } from "lucide-react";

interface Props {
  currentLtvBps: number;
  maxLtvBps: number;
  currentPositionBps: number;
  maxPositionBps: number;
}

export function RiskMeter({
  currentLtvBps,
  maxLtvBps,
  currentPositionBps,
  maxPositionBps,
}: Props) {
  const ltvUsagePct = maxLtvBps > 0 ? (currentLtvBps / maxLtvBps) * 100 : 0;
  const positionUsagePct = maxPositionBps > 0 ? (currentPositionBps / maxPositionBps) * 100 : 0;

  const isLtvHigh = ltvUsagePct > 80;
  const isPosHigh = positionUsagePct > 85;

  return (
    <div className="glass-panel rounded-xl p-5 border border-slate-800 space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Shield className="w-4 h-4 text-emerald-400" />
          <h3 className="text-sm font-bold text-slate-200">On-Chain Risk Defense Line</h3>
        </div>

        {isLtvHigh || isPosHigh ? (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded badge-amber text-[10px] font-semibold">
            <AlertTriangle className="w-3 h-3" />
            Warning Limits
          </span>
        ) : (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded badge-emerald text-[10px] font-semibold">
            <CheckCircle2 className="w-3 h-3" />
            Within Policy
          </span>
        )}
      </div>

      {/* LTV Meter */}
      <div className="space-y-1.5">
        <div className="flex justify-between text-xs font-mono">
          <span className="text-slate-400">Loan-to-Value (LTV)</span>
          <span className="text-slate-200 font-bold">
            {(currentLtvBps / 100).toFixed(1)}% / {(maxLtvBps / 100).toFixed(1)}% Max
          </span>
        </div>
        <div className="h-2 w-full rounded-full bg-slate-950 overflow-hidden border border-slate-800">
          <div
            className={`h-full rounded-full transition-all duration-300 ${
              isLtvHigh ? "bg-rose-500" : ltvUsagePct > 60 ? "bg-amber-400" : "bg-emerald-400"
            }`}
            style={{ width: `${Math.min(ltvUsagePct, 100)}%` }}
          />
        </div>
      </div>

      {/* Single Position Exposure Meter */}
      <div className="space-y-1.5">
        <div className="flex justify-between text-xs font-mono">
          <span className="text-slate-400">Max Single Position Exposure</span>
          <span className="text-slate-200 font-bold">
            {(currentPositionBps / 100).toFixed(1)}% / {(maxPositionBps / 100).toFixed(1)}% Max
          </span>
        </div>
        <div className="h-2 w-full rounded-full bg-slate-950 overflow-hidden border border-slate-800">
          <div
            className={`h-full rounded-full transition-all duration-300 ${
              isPosHigh ? "bg-rose-500" : positionUsagePct > 70 ? "bg-amber-400" : "bg-cyan-400"
            }`}
            style={{ width: `${Math.min(positionUsagePct, 100)}%` }}
          />
        </div>
      </div>

      <div className="pt-2 border-t border-slate-800/80 flex items-center justify-between text-[11px] text-slate-400 font-mono">
        <span>Dual Line Defense:</span>
        <span className="text-emerald-400 font-semibold">Anchor CPI Checks Active</span>
      </div>
    </div>
  );
}
