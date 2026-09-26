"use client";

import React, { useState } from "react";
import {
  ExternalLink,
  ChevronDown,
  ChevronUp,
  AlertCircle,
  Copy,
  Check,
  CheckCircle2,
  Clock,
  XCircle,
} from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

export interface ActivityItem {
  id: string;
  type: "buy" | "sell" | "rebalance" | "deposit" | "alert";
  title: string;
  description?: string;
  amountUsd?: number;
  timestamp: string | Date;
  status: "completed" | "pending" | "failed";
  txSignature?: string;
  details?: Record<string, any>;
}

export interface ActivityListProps {
  activities: ActivityItem[];
  isLoading?: boolean;
  error?: string | null;
  title?: string;
  subtitle?: string;
  maxItems?: number;
  showFilter?: boolean;
}

/**
 * ActivityList answers one fundamental question:
 * "What happened?"
 *
 * Adheres to:
 * - Plain language: "Bought $500 NVDA", "Rebalanced portfolio"
 * - Restrained status badges
 * - Clickable technical details (signatures, explorer link) inside an expandable view
 * - Simple table/timeline layout
 */
export function ActivityList({
  activities,
  isLoading = false,
  error = null,
  title = "Recent activity",
  subtitle = "Historical records of trades, rebalances, and system events",
  maxItems,
  showFilter = true,
}: ActivityListProps) {
  const [filter, setFilter] = useState<"all" | "trades" | "rebalances">("all");
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [copiedSig, setCopiedSig] = useState<string | null>(null);

  const handleCopy = (sig: string, e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(sig);
    setCopiedSig(sig);
    setTimeout(() => setCopiedSig(null), 2000);
  };

  const filtered = activities
    .filter((item) => {
      if (filter === "trades") return item.type === "buy" || item.type === "sell";
      if (filter === "rebalances") return item.type === "rebalance";
      return true;
    })
    .slice(0, maxItems || activities.length);

  const getStatusBadge = (status: ActivityItem["status"]) => {
    switch (status) {
      case "completed":
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-emerald-50 text-emerald-700 border border-emerald-200">
            <CheckCircle2 className="w-3 h-3" />
            <span>Completed</span>
          </span>
        );
      case "pending":
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-amber-50 text-amber-700 border border-amber-200">
            <Clock className="w-3 h-3" />
            <span>Pending</span>
          </span>
        );
      case "failed":
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-rose-50 text-rose-700 border border-rose-200">
            <XCircle className="w-3 h-3" />
            <span>Failed</span>
          </span>
        );
    }
  };

  return (
    <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
      <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div>
          <CardTitle className="text-base font-bold text-slate-900">{title}</CardTitle>
          {subtitle && <p className="text-xs text-slate-500 mt-0.5">{subtitle}</p>}
        </div>

        {showFilter && (
          <div className="flex items-center gap-1 bg-slate-100 p-1 rounded-lg">
            <button
              onClick={() => setFilter("all")}
              className={`px-2.5 py-1 text-xs font-semibold rounded-md transition-colors ${
                filter === "all" ? "bg-white text-slate-900 shadow-sm" : "text-slate-600 hover:text-slate-900"
              }`}
            >
              All
            </button>
            <button
              onClick={() => setFilter("trades")}
              className={`px-2.5 py-1 text-xs font-semibold rounded-md transition-colors ${
                filter === "trades" ? "bg-white text-slate-900 shadow-sm" : "text-slate-600 hover:text-slate-900"
              }`}
            >
              Trades
            </button>
            <button
              onClick={() => setFilter("rebalances")}
              className={`px-2.5 py-1 text-xs font-semibold rounded-md transition-colors ${
                filter === "rebalances" ? "bg-white text-slate-900 shadow-sm" : "text-slate-600 hover:text-slate-900"
              }`}
            >
              Rebalances
            </button>
          </div>
        )}
      </CardHeader>

      <CardContent className="p-0">
        {isLoading && activities.length === 0 ? (
          <div className="p-8 text-center text-slate-400 text-sm">
            Loading recent activity...
          </div>
        ) : error ? (
          <div className="p-8 flex items-center justify-center gap-2 text-slate-600 text-sm">
            <AlertCircle className="w-5 h-5 text-amber-500 shrink-0" />
            <span>Unable to load live data</span>
          </div>
        ) : filtered.length === 0 ? (
          <div className="p-8 text-center text-slate-500 text-sm">
            No recent activity recorded.
          </div>
        ) : (
          <div className="divide-y divide-slate-100">
            {filtered.map((item) => {
              const isExpanded = expandedId === item.id;
              const dateStr =
                item.timestamp instanceof Date
                  ? item.timestamp.toLocaleString()
                  : new Date(item.timestamp).toLocaleString();

              return (
                <div key={item.id} className="transition-colors hover:bg-slate-50/50">
                  <div
                    onClick={() => setExpandedId(isExpanded ? null : item.id)}
                    className="p-4 sm:px-6 flex items-center justify-between gap-4 cursor-pointer"
                  >
                    <div className="min-w-0 space-y-1">
                      <div className="flex items-center gap-2">
                        <span className="font-semibold text-slate-900 text-sm truncate">
                          {item.title}
                        </span>
                        {getStatusBadge(item.status)}
                      </div>
                      <div className="text-xs text-slate-500 flex items-center gap-2">
                        <span>{dateStr}</span>
                        {item.amountUsd && (
                          <>
                            <span className="text-slate-300">•</span>
                            <span className="font-medium text-slate-700">
                              ${item.amountUsd.toLocaleString(undefined, { minimumFractionDigits: 2 })}
                            </span>
                          </>
                        )}
                      </div>
                    </div>

                    <div className="flex items-center gap-2 shrink-0">
                      {item.txSignature && (
                        <button
                          type="button"
                          onClick={(e) => handleCopy(item.txSignature!, e)}
                          title="Copy transaction signature"
                          className="p-1 rounded text-slate-400 hover:text-slate-600 hover:bg-slate-100 transition-colors"
                        >
                          {copiedSig === item.txSignature ? (
                            <Check className="w-4 h-4 text-emerald-600" />
                          ) : (
                            <Copy className="w-4 h-4" />
                          )}
                        </button>
                      )}
                      <div className="text-slate-400 p-1">
                        {isExpanded ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
                      </div>
                    </div>
                  </div>

                  {/* Expandable Advanced Technical Details */}
                  {isExpanded && (
                    <div className="px-4 pb-4 sm:px-6 pt-1 bg-slate-50/80 border-t border-slate-100 text-xs space-y-2">
                      <div className="font-semibold text-slate-700 pt-1">Transaction Details</div>
                      {item.description && (
                        <p className="text-slate-600">{item.description}</p>
                      )}

                      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-1 font-mono text-[11px]">
                        <div>
                          <span className="text-slate-400 block">ID:</span>
                          <span className="text-slate-700">{item.id}</span>
                        </div>

                        {item.txSignature && (
                          <div>
                            <span className="text-slate-400 block">Solana Signature:</span>
                            <a
                              href={`https://explorer.solana.com/tx/${item.txSignature}?cluster=devnet`}
                              target="_blank"
                              rel="noreferrer"
                              className="text-emerald-700 hover:text-emerald-800 underline inline-flex items-center gap-1"
                            >
                              <span>{item.txSignature.slice(0, 8)}...{item.txSignature.slice(-8)}</span>
                              <ExternalLink className="w-3 h-3" />
                            </a>
                          </div>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
