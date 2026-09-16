"use client";

import React from "react";
import {
  AlertCircle,
  AlertTriangle,
  Clock,
  Cpu,
  RefreshCw,
  RotateCcw,
  ShieldAlert,
  Wallet,
  WifiOff,
  X,
} from "lucide-react";

import { Button } from "./ui/button";

export type ErrorCategory =
  | "stale_market_data"
  | "rpc_failure"
  | "api_failure"
  | "wallet_failure"
  | "simulation_failure"
  | "policy_rejection";

export interface ErrorStateProps {
  category: ErrorCategory;
  title?: string;
  message?: string;
  details?: string;
  onRetry?: () => void;
  onDismiss?: () => void;
  actionLabel?: string;
  className?: string;
}

export function ErrorAlert({
  category,
  title,
  message,
  details,
  onRetry,
  onDismiss,
  actionLabel,
  className = "",
}: ErrorStateProps) {
  const getCategoryConfig = () => {
    switch (category) {
      case "stale_market_data":
        return {
          icon: Clock,
          defaultTitle: "Stale Pyth Oracle Price Detected",
          defaultMessage:
            "Market data latency exceeded the 15-second safety freshness threshold. Programmatic rebalancing and trade executions are halted until fresh oracle updates arrive.",
          bg: "bg-amber-50 border-amber-200 text-amber-900",
          iconColor: "text-amber-600",
          buttonVariant: "outline" as const,
          defaultAction: "Force Pyth Refresh",
        };
      case "rpc_failure":
        return {
          icon: WifiOff,
          defaultTitle: "Solana RPC Connection Timeout",
          defaultMessage:
            "Unable to reach the configured Solana RPC cluster node. Queries and preflight simulations may be degraded.",
          bg: "bg-rose-50 border-rose-200 text-rose-900",
          iconColor: "text-rose-600",
          buttonVariant: "outline" as const,
          defaultAction: "Retry RPC Query",
        };
      case "api_failure":
        return {
          icon: AlertCircle,
          defaultTitle: "Backend API Service Unreachable",
          defaultMessage:
            "Failed to communicate with Equity Catalyst REST API services. The application is running in local degraded mode.",
          bg: "bg-rose-50 border-rose-200 text-rose-900",
          iconColor: "text-rose-600",
          buttonVariant: "outline" as const,
          defaultAction: "Reconnect API",
        };
      case "wallet_failure":
        return {
          icon: Wallet,
          defaultTitle: "Wallet Signing / Connection Error",
          defaultMessage:
            "Wallet signature was rejected or the connected account has insufficient SOL to cover transaction network fees.",
          bg: "bg-orange-50 border-orange-200 text-orange-900",
          iconColor: "text-orange-600",
          buttonVariant: "outline" as const,
          defaultAction: "Re-prompt Wallet",
        };
      case "simulation_failure":
        return {
          icon: Cpu,
          defaultTitle: "Preflight Simulation Intercepted",
          defaultMessage:
            "Solana preflight simulation returned an error. The transaction was aborted before on-chain submission to prevent state corruption and save gas fees.",
          bg: "bg-purple-50 border-purple-200 text-purple-900",
          iconColor: "text-purple-600",
          buttonVariant: "outline" as const,
          defaultAction: "Inspect Simulation Trace",
        };
      case "policy_rejection":
        return {
          icon: ShieldAlert,
          defaultTitle: "Deterministic Policy Rejection",
          defaultMessage:
            "The proposed action violated an active on-chain Anchor constraint or backend risk engine limit. Trade blocked deterministically.",
          bg: "bg-rose-50 border-rose-200 text-rose-900",
          iconColor: "text-rose-600",
          buttonVariant: "outline" as const,
          defaultAction: "Review Policy Limits",
        };
    }
  };

  const config = getCategoryConfig();
  const Icon = config.icon;

  return (
    <div
      className={`rounded-xl border p-4 transition-all shadow-sm relative ${config.bg} ${className}`}
      role="alert"
    >
      {onDismiss && (
        <button
          onClick={onDismiss}
          className="absolute top-3 right-3 text-slate-400 hover:text-slate-600 transition-colors"
          aria-label="Dismiss error"
        >
          <X className="w-4 h-4" />
        </button>
      )}

      <div className="flex items-start gap-3">
        <div className={`p-1 rounded-md ${config.iconColor} shrink-0 mt-0.5`}>
          <Icon className="w-5 h-5" />
        </div>

        <div className="flex-1 pr-4 space-y-1">
          <h4 className="text-xs font-bold font-mono tracking-tight">
            {title || config.defaultTitle}
          </h4>

          <p className="text-xs leading-relaxed opacity-90">
            {message || config.defaultMessage}
          </p>

          {details && (
            <div className="mt-2 p-2.5 rounded bg-black/5 font-mono text-[11px] break-all border border-black/10">
              {details}
            </div>
          )}

          {onRetry && (
            <div className="pt-2">
              <Button
                variant={config.buttonVariant}
                size="sm"
                onClick={onRetry}
                className="h-7 text-xs font-semibold flex items-center gap-1.5 bg-white/80 hover:bg-white"
              >
                <RefreshCw className="w-3 h-3" />
                <span>{actionLabel || config.defaultAction}</span>
              </Button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
