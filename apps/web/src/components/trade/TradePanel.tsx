"use client";

import React, { useState, useEffect } from "react";
import { X, ChevronDown, ChevronUp, AlertCircle, CheckCircle2 } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export interface TradePanelProps {
  initialMode?: "buy" | "sell" | "rebalance";
  initialSymbol?: string;
  availableAssets: Array<{
    symbol: string;
    name: string;
    priceUsd: number | null;
    balanceUnits?: number;
  }>;
  cashBalanceUsd: number;
  isOpen?: boolean;
  onClose?: () => void;
  onExecute?: (params: {
    mode: "buy" | "sell" | "rebalance";
    symbol?: string;
    amountUsd: number;
    slippagePct: number;
  }) => Promise<{ success: boolean; message?: string; txSignature?: string }>;
}

/**
 * TradePanel answers one fundamental question:
 * "What am I buying/selling?"
 *
 * Adheres to:
 * - Simple modes: "Buy", "Sell", "Rebalance"
 * - Clear pricing and balance display
 * - Collapsible "Advanced settings" for slippage
 * - Plain language: "Buy", "Sell", "Rebalance", "Cash available"
 */
export function TradePanel({
  initialMode = "buy",
  initialSymbol,
  availableAssets,
  cashBalanceUsd,
  isOpen = true,
  onClose,
  onExecute,
}: TradePanelProps) {
  const [mode, setMode] = useState<"buy" | "sell" | "rebalance">(initialMode);
  const [selectedSymbol, setSelectedSymbol] = useState<string>(
    initialSymbol || availableAssets[0]?.symbol || ""
  );
  const [amountUsd, setAmountUsd] = useState<string>("");
  const [slippagePct, setSlippagePct] = useState<number>(0.5);
  const [showAdvanced, setShowAdvanced] = useState<boolean>(false);
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [feedback, setFeedback] = useState<{ success: boolean; message: string } | null>(null);

  useEffect(() => {
    if (initialSymbol) {
      setSelectedSymbol(initialSymbol);
    }
  }, [initialSymbol]);

  useEffect(() => {
    setMode(initialMode);
  }, [initialMode]);

  const currentAsset = availableAssets.find((a) => a.symbol === selectedSymbol) || availableAssets[0];
  const price = currentAsset?.priceUsd && currentAsset.priceUsd > 0 ? currentAsset.priceUsd : 0;
  const parsedAmount = parseFloat(amountUsd) || 0;
  const estimatedUnits = price > 0 ? parsedAmount / price : 0;
  const holdingUnits = currentAsset?.balanceUnits || 0;
  const holdingValueUsd = holdingUnits * price;

  const handlePercentageClick = (pct: number) => {
    if (mode === "buy") {
      const val = (cashBalanceUsd * pct) / 100;
      setAmountUsd(val > 0 ? val.toFixed(2) : "");
    } else if (mode === "sell") {
      const val = (holdingValueUsd * pct) / 100;
      setAmountUsd(val > 0 ? val.toFixed(2) : "");
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!onExecute) return;

    if (mode !== "rebalance" && parsedAmount <= 0) return;

    setIsSubmitting(true);
    setFeedback(null);
    try {
      const res = await onExecute({
        mode,
        symbol: mode !== "rebalance" ? selectedSymbol : undefined,
        amountUsd: parsedAmount,
        slippagePct,
      });

      if (res.success) {
        setFeedback({
          success: true,
          message:
            res.message ||
            (mode === "rebalance"
              ? "Rebalance order submitted successfully."
              : `${mode === "buy" ? "Bought" : "Sold"} ${estimatedUnits.toFixed(4)} ${selectedSymbol}`),
        });
        setAmountUsd("");
      } else {
        setFeedback({
          success: false,
          message: res.message || "Failed to execute order.",
        });
      }
    } catch (err: any) {
      setFeedback({
        success: false,
        message: err?.message || "An unexpected error occurred.",
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <Card className="bg-white border-slate-200 shadow-lg rounded-xl overflow-hidden max-w-md w-full mx-auto">
      {/* Header */}
      <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
        <CardTitle className="text-base font-bold text-slate-900">
          {mode === "rebalance" ? "Rebalance Portfolio" : `${mode === "buy" ? "Buy" : "Sell"} ${selectedSymbol || "Asset"}`}
        </CardTitle>
        {onClose && (
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-600 hover:bg-slate-100 transition-colors"
            aria-label="Close trade panel"
          >
            <X className="w-5 h-5" />
          </button>
        )}
      </CardHeader>

      <CardContent className="p-6 space-y-5">
        {/* Mode Selector */}
        <div className="grid grid-cols-3 gap-1 p-1 bg-slate-100 rounded-lg">
          <button
            type="button"
            onClick={() => {
              setMode("buy");
              setFeedback(null);
            }}
            className={`py-1.5 text-xs font-semibold rounded-md transition-all ${
              mode === "buy"
                ? "bg-white text-emerald-800 shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            Buy
          </button>
          <button
            type="button"
            onClick={() => {
              setMode("sell");
              setFeedback(null);
            }}
            className={`py-1.5 text-xs font-semibold rounded-md transition-all ${
              mode === "sell"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            Sell
          </button>
          <button
            type="button"
            onClick={() => {
              setMode("rebalance");
              setFeedback(null);
            }}
            className={`py-1.5 text-xs font-semibold rounded-md transition-all ${
              mode === "rebalance"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            Rebalance
          </button>
        </div>

        {/* Feedback Alert */}
        {feedback && (
          <div
            className={`p-3 rounded-lg text-xs flex items-start gap-2 ${
              feedback.success
                ? "bg-emerald-50 text-emerald-800 border border-emerald-200"
                : "bg-rose-50 text-rose-800 border border-rose-200"
            }`}
          >
            {feedback.success ? (
              <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0 mt-0.5" />
            ) : (
              <AlertCircle className="w-4 h-4 text-rose-600 shrink-0 mt-0.5" />
            )}
            <div className="font-medium">{feedback.message}</div>
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          {mode !== "rebalance" ? (
            <>
              {/* Asset Selection */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700">Select asset</label>
                <select
                  value={selectedSymbol}
                  onChange={(e) => {
                    setSelectedSymbol(e.target.value);
                    setAmountUsd("");
                  }}
                  className="w-full h-10 px-3 rounded-lg border border-slate-200 bg-white text-sm font-medium text-slate-900 focus:outline-none focus:ring-2 focus:ring-emerald-500"
                >
                  {availableAssets.map((asset) => (
                    <option key={asset.symbol} value={asset.symbol}>
                      {asset.symbol} — {asset.name} ({asset.priceUsd && asset.priceUsd > 0 ? `$${asset.priceUsd.toFixed(2)}` : "Price unavailable"})
                    </option>
                  ))}
                </select>
              </div>

              {/* Amount Input */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="font-semibold text-slate-700">Amount (USD)</span>
                  <span className="text-slate-500">
                    {mode === "buy" ? (
                      <>Cash available: <strong className="text-slate-800">${cashBalanceUsd.toFixed(2)}</strong></>
                    ) : (
                      <>Holding value: <strong className="text-slate-800">${holdingValueUsd.toFixed(2)}</strong></>
                    )}
                  </span>
                </div>

                <div className="relative">
                  <span className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400 text-sm">$</span>
                  <Input
                    type="number"
                    step="any"
                    min="0"
                    placeholder="0.00"
                    value={amountUsd}
                    onChange={(e) => setAmountUsd(e.target.value)}
                    className="pl-7 text-sm h-10 bg-slate-50 border-slate-200 focus:bg-white"
                  />
                </div>

                {/* Percentage Shortcuts */}
                <div className="flex items-center gap-2 pt-1">
                  {[25, 50, 75, 100].map((pct) => (
                    <button
                      key={pct}
                      type="button"
                      onClick={() => handlePercentageClick(pct)}
                      className="flex-1 py-1 text-xs font-semibold text-slate-600 bg-slate-100 hover:bg-slate-200 rounded transition-colors"
                    >
                      {pct === 100 ? "Max" : `${pct}%`}
                    </button>
                  ))}
                </div>
              </div>

              {/* Order Summary */}
              <div className="p-3 bg-slate-50 rounded-lg space-y-1.5 text-xs">
                <div className="flex justify-between text-slate-600">
                  <span>Current price:</span>
                  <span className="font-semibold text-slate-900">
                    {price > 0 ? `$${price.toFixed(2)}` : "Price unavailable"}
                  </span>
                </div>
                <div className="flex justify-between text-slate-600">
                  <span>Estimated units:</span>
                  <span className="font-semibold text-slate-900">
                    {price > 0 && estimatedUnits > 0 ? estimatedUnits.toFixed(4) : "—"} {selectedSymbol}
                  </span>
                </div>
              </div>
            </>
          ) : (
            /* Rebalance Mode */
            <div className="space-y-3 p-4 bg-slate-50 rounded-lg text-xs text-slate-600">
              <p className="font-medium text-slate-800">
                Rebalance all portfolio holdings to their target weights.
              </p>
              <p>
                The system will automatically calculate the required trades to realign current weights with target allocations while preserving your cash reserve.
              </p>
            </div>
          )}

          {/* Advanced Collapsible Settings */}
          <div className="pt-1">
            <button
              type="button"
              onClick={() => setShowAdvanced(!showAdvanced)}
              className="flex items-center justify-between w-full text-xs text-slate-500 hover:text-slate-800 transition-colors py-1"
            >
              <span>Advanced details</span>
              {showAdvanced ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
            </button>

            {showAdvanced && (
              <div className="mt-2 p-3 bg-slate-50 rounded-lg space-y-2 text-xs">
                <div className="flex items-center justify-between">
                  <span className="text-slate-600">Slippage tolerance</span>
                  <div className="flex items-center gap-1">
                    <button
                      type="button"
                      onClick={() => setSlippagePct(0.1)}
                      className={`px-2 py-0.5 rounded text-xs ${slippagePct === 0.1 ? "bg-slate-800 text-white" : "bg-slate-200"}`}
                    >
                      0.1%
                    </button>
                    <button
                      type="button"
                      onClick={() => setSlippagePct(0.5)}
                      className={`px-2 py-0.5 rounded text-xs ${slippagePct === 0.5 ? "bg-slate-800 text-white" : "bg-slate-200"}`}
                    >
                      0.5%
                    </button>
                    <button
                      type="button"
                      onClick={() => setSlippagePct(1.0)}
                      className={`px-2 py-0.5 rounded text-xs ${slippagePct === 1.0 ? "bg-slate-800 text-white" : "bg-slate-200"}`}
                    >
                      1.0%
                    </button>
                  </div>
                </div>
                <div className="flex items-center justify-between text-slate-500 text-xs">
                  <span>Execution route</span>
                  <span>Jupiter DEX Direct</span>
                </div>
              </div>
            )}
          </div>

          {/* Submit Action */}
          <Button
            type="submit"
            disabled={isSubmitting || (mode !== "rebalance" && (parsedAmount <= 0 || price <= 0))}
            className="w-full h-10 font-semibold bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg shadow-sm transition-all"
          >
            {isSubmitting ? (
              "Processing..."
            ) : mode === "rebalance" ? (
              "Rebalance Portfolio"
            ) : price <= 0 ? (
              "Price unavailable"
            ) : (
              `${mode === "buy" ? "Buy" : "Sell"} ${selectedSymbol}`
            )}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
