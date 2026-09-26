"use client";

import React from "react";
import { ShieldCheck, AlertTriangle } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export interface SafetyStatusProps {
  isProtected: boolean;
  cashReservePct: number;
  minCashRequiredPct: number;
  maxPositionPct: number;
  maxPositionAllowedPct: number;
  maxPositionSymbol?: string;
  isLoading?: boolean;
}

/**
 * SafetyStatus answers one fundamental question:
 * "Is the system operating within its rules?"
 *
 * Adheres to:
 * - Clear status: "Protected" vs "Needs attention"
 * - Restrained typography and clean card layout
 * - Plain language: "Cash buffer", "Position limit", "Operating status"
 * - No technical jargon
 */
export function SafetyStatus({
  isProtected,
  cashReservePct,
  minCashRequiredPct,
  maxPositionPct,
  maxPositionAllowedPct,
  maxPositionSymbol,
  isLoading = false,
}: SafetyStatusProps) {
  const isCashOk = cashReservePct >= minCashRequiredPct;
  const isExposureOk = maxPositionPct <= maxPositionAllowedPct;
  const overallSafe = isProtected && isCashOk && isExposureOk;

  return (
    <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
      <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
        <div className="flex items-center gap-2">
          <ShieldCheck className={`w-5 h-5 ${overallSafe ? "text-emerald-600" : "text-amber-500"}`} />
          <CardTitle className="text-base font-bold text-slate-900">
            System safety
          </CardTitle>
        </div>

        <div>
          {overallSafe ? (
            <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
              Protected
            </span>
          ) : (
            <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-amber-50 text-amber-700 border border-amber-200">
              <AlertTriangle className="w-3.5 h-3.5" />
              Needs attention
            </span>
          )}
        </div>
      </CardHeader>

      <CardContent className="p-6">
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          {/* 1. Cash Buffer */}
          <div className="p-3.5 rounded-lg bg-slate-50 border border-slate-100 space-y-1">
            <div className="text-xs font-medium text-slate-500">Cash reserve</div>
            <div className="text-lg font-bold text-slate-900">
              {isLoading ? "—" : `${cashReservePct.toFixed(1)}%`}
            </div>
            <div className="text-xs text-slate-500">
              Minimum {minCashRequiredPct.toFixed(0)}% required
            </div>
          </div>

          {/* 2. Concentration / Max Position */}
          <div className="p-3.5 rounded-lg bg-slate-50 border border-slate-100 space-y-1">
            <div className="text-xs font-medium text-slate-500">Single asset exposure</div>
            <div className="text-lg font-bold text-slate-900">
              {isLoading ? "—" : `${maxPositionPct.toFixed(1)}%`}
              {maxPositionSymbol && !isLoading && (
                <span className="text-xs font-normal text-slate-500 ml-1">({maxPositionSymbol})</span>
              )}
            </div>
            <div className="text-xs text-slate-500">
              Max {maxPositionAllowedPct.toFixed(0)}% limit
            </div>
          </div>

          {/* 3. Safety Rules */}
          <div className="p-3.5 rounded-lg bg-slate-50 border border-slate-100 space-y-1">
            <div className="text-xs font-medium text-slate-500">Rule verification</div>
            <div className="text-lg font-bold text-slate-900">
              {overallSafe ? "All rules passing" : "Review limits"}
            </div>
            <div className="text-xs text-slate-500">
              Automatic risk guardrails
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
