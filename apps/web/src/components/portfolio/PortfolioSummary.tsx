"use client";

import React from "react";
import { ArrowUpRight, ArrowDownRight, RefreshCw, AlertCircle } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

export interface PortfolioSummaryProps {
  portfolioValueUsd: number | null;
  cashUsd: number | null;
  cashPct: number | null;
  pnlUsd: number | null;
  pnlPct: number | null;
  lastUpdated: Date | null;
  isLoading?: boolean;
  error?: string | null;
  onRefresh?: () => void;
  onOpenTrade?: (mode: "buy" | "sell" | "rebalance") => void;
}

/**
 * PortfolioSummary answers one fundamental question:
 * "How much do I have?"
 *
 * Adheres to:
 * - White/light background, slate text, restrained emerald accent
 * - Plain language: "Portfolio value", "Cash", "Last updated"
 * - Fails closed: shows "Unable to load live data" if unavailable
 */
export function PortfolioSummary({
  portfolioValueUsd,
  cashUsd,
  cashPct,
  pnlUsd,
  pnlPct,
  lastUpdated,
  isLoading = false,
  error = null,
  onRefresh,
  onOpenTrade,
}: PortfolioSummaryProps) {
  const isDataAvailable = portfolioValueUsd !== null && !error;
  const isPositivePnl = (pnlUsd ?? 0) >= 0;

  return (
    <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
      <CardContent className="p-6">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-6">
          {/* Main Financial Figure */}
          <div className="space-y-1.5">
            <div className="flex items-center gap-2">
              <span className="text-xs font-semibold text-slate-500">
                Portfolio value
              </span>
              {onRefresh && (
                <button
                  onClick={onRefresh}
                  disabled={isLoading}
                  title="Refresh portfolio value"
                  className="text-slate-400 hover:text-slate-600 transition-colors p-0.5 rounded"
                  aria-label="Refresh portfolio data"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? "animate-spin text-emerald-600" : ""}`} />
                </button>
              )}
            </div>

            {isLoading && !isDataAvailable ? (
              <div className="h-10 w-48 bg-slate-100 animate-pulse rounded-lg my-1" />
            ) : error ? (
              <div className="flex items-center gap-2 py-1 text-slate-600">
                <AlertCircle className="w-5 h-5 text-amber-500 shrink-0" />
                <span className="text-sm font-medium">Unable to load live data</span>
              </div>
            ) : (
              <div className="flex flex-wrap items-baseline gap-3">
                <h1 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-slate-900">
                  ${(portfolioValueUsd ?? 0).toLocaleString(undefined, {
                    minimumFractionDigits: 2,
                    maximumFractionDigits: 2,
                  })}
                </h1>
                {pnlUsd !== null && pnlPct !== null && (
                  <span
                    className={`inline-flex items-center text-xs font-bold px-2 py-0.5 rounded-md ${
                      isPositivePnl
                        ? "text-emerald-700 bg-emerald-50 border border-emerald-200"
                        : "text-rose-700 bg-rose-50 border border-rose-200"
                    }`}
                  >
                    {isPositivePnl ? (
                      <ArrowUpRight className="w-3.5 h-3.5 mr-0.5" />
                    ) : (
                      <ArrowDownRight className="w-3.5 h-3.5 mr-0.5" />
                    )}
                    {isPositivePnl ? "+" : ""}${Math.abs(pnlUsd).toLocaleString(undefined, {
                      minimumFractionDigits: 2,
                      maximumFractionDigits: 2,
                    })}{" "}
                    ({isPositivePnl ? "+" : ""}{pnlPct.toFixed(2)}%)
                  </span>
                )}
              </div>
            )}

            {/* Cash & Timestamp Breakdown */}
            <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-slate-500 pt-0.5">
              {cashUsd !== null && (
                <span>
                  Cash:{" "}
                  <strong className="text-slate-700">
                    ${cashUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                  </strong>
                  {cashPct !== null && ` (${cashPct.toFixed(1)}%)`}
                </span>
              )}

              {lastUpdated && (
                <>
                  <span className="text-slate-300">•</span>
                  <span>Last updated {lastUpdated.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
                </>
              )}
            </div>
          </div>

          {/* User Action Buttons */}
          <div className="flex items-center gap-3 shrink-0">
            <Button
              onClick={() => onOpenTrade?.("buy")}
              className="bg-emerald-600 hover:bg-emerald-700 text-white font-semibold px-5 py-2 rounded-lg text-sm shadow-sm transition-all"
            >
              Buy / Sell
            </Button>
            <Button
              variant="outline"
              onClick={() => onOpenTrade?.("rebalance")}
              className="border-slate-300 hover:bg-slate-50 text-slate-700 font-semibold px-4 py-2 rounded-lg text-sm transition-all"
            >
              Rebalance
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
