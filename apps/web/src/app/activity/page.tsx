"use client";

import React, { useState, useEffect, useMemo } from "react";
import Link from "next/link";
import {
  History,
  RefreshCw,
  ExternalLink,
  Copy,
  Check,
  CheckCircle2,
  Clock,
  XCircle,
  ChevronDown,
  ChevronUp,
  Bot,
  ShieldCheck,
  ArrowUpRight,
} from "lucide-react";

import { getApiClient, ExecutionModel, PolicyEventModel } from "@/lib/api-client";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { ADVANCED_NAVIGATION } from "@/lib/navigation";

export type ActivityTab = "All" | "Trades" | "Rebalances" | "System";

export interface EnrichedActivity {
  id: string;
  tabType: "Trades" | "Rebalances" | "System";
  action: string;
  asset: string;
  amount: string;
  amountUsd?: number;
  status: "Completed" | "Pending" | "Failed";
  time: string;
  timestampRaw: string;
  // Trade details for click-to-expand
  expectedOutput?: string;
  actualOutput?: string;
  fees?: string;
  txSignature?: string;
  confirmationTime?: string;
  // Technical details
  inputMint?: string;
  outputMint?: string;
  vaultAddress?: string;
}

export default function ActivityPage() {
  const [activeTab, setActiveTab] = useState<ActivityTab>("All");
  const [activities, setActivities] = useState<EnrichedActivity[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [copiedSig, setCopiedSig] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const loadActivityData = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const client = getApiClient();
      const [executionsRes, eventsRes] = await Promise.allSettled([
        client.listExecutions(),
        client.listEvents(),
      ]);

      const items: EnrichedActivity[] = [];

      // Process Executions
      if (executionsRes.status === "fulfilled" && executionsRes.value) {
        executionsRes.value.forEach((exec) => {
          const isBuy = exec.action.toLowerCase().includes("buy");
          const isRebal = exec.action.toLowerCase().includes("rebalance");

          let assetName = "NVDA";
          if (exec.output_mint.includes("USDC") || exec.input_mint.includes("USDC")) {
            assetName = isBuy ? "NVDA" : "USDC";
          }

          let statusLabel: "Completed" | "Pending" | "Failed" = "Pending";
          if (exec.status === "confirmed") statusLabel = "Completed";
          else if (exec.status === "failed") statusLabel = "Failed";

          const feeAmount = (exec.amount_in * 0.0015).toFixed(2);
          const confirmationDuration = exec.confirmed_at
            ? `${(
                (new Date(exec.confirmed_at).getTime() - new Date(exec.executed_at).getTime()) /
                1000
              ).toFixed(1)}s`
            : "1.2s";

          items.push({
            id: exec.execution_id,
            tabType: isRebal ? "Rebalances" : "Trades",
            action: isBuy ? "Buy" : isRebal ? "Rebalance" : "Sell",
            asset: isRebal ? "Portfolio" : assetName,
            amount: `$${exec.amount_in.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`,
            amountUsd: exec.amount_in,
            status: statusLabel,
            time: new Date(exec.executed_at).toLocaleTimeString([], {
              hour: "2-digit",
              minute: "2-digit",
            }),
            timestampRaw: exec.executed_at,
            expectedOutput: `${exec.amount_out_expected.toFixed(4)} ${isBuy ? assetName : "USDC"}`,
            actualOutput: exec.amount_out_actual
              ? `${exec.amount_out_actual.toFixed(4)} ${isBuy ? assetName : "USDC"}`
              : `${exec.amount_out_expected.toFixed(4)} ${isBuy ? assetName : "USDC"}`,
            fees: `$${feeAmount} (0.15%)`,
            txSignature: exec.tx_signature,
            confirmationTime: confirmationDuration,
            inputMint: exec.input_mint,
            outputMint: exec.output_mint,
            vaultAddress: exec.vault_address,
          });
        });
      }

      // Process Policy Events & Proposals
      if (eventsRes.status === "fulfilled" && eventsRes.value) {
        eventsRes.value.forEach((ev) => {
          if (!items.some((item) => item.id === ev.event_id)) {
            const isRebalProposal = ev.event_type.includes("REBALANCE");

            let actionName = ev.event_type.replace(/_/g, " ");
            if (isRebalProposal) {
              actionName = ev.status === "pending" ? "Rebalance suggested" : "Rebalance";
            } else if (ev.status === "pending") {
              actionName = "Trade awaiting validation";
            }

            let statusLabel: "Completed" | "Pending" | "Failed" = "Pending";
            if (ev.status === "completed") statusLabel = "Completed";
            else if (ev.status === "failed") statusLabel = "Failed";

            items.push({
              id: ev.event_id,
              tabType: isRebalProposal ? "Rebalances" : "System",
              action: actionName,
              asset: isRebalProposal ? "Portfolio" : "System",
              amount: "—",
              status: statusLabel,
              time: new Date(ev.created_at).toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit",
              }),
              timestampRaw: ev.created_at,
              vaultAddress: ev.vault_address,
            });
          }
        });
      }

      // Sort descending by time
      items.sort(
        (a, b) => new Date(b.timestampRaw).getTime() - new Date(a.timestampRaw).getTime()
      );

      setActivities(items);
    } catch {
      setError("Unable to load live activity data");
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadActivityData();
  }, []);

  const handleCopySig = (sig: string, e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(sig);
    setCopiedSig(sig);
    setTimeout(() => setCopiedSig(null), 2000);
  };

  // Filter activities by tab
  const filteredActivities = useMemo(() => {
    if (activeTab === "All") return activities;
    return activities.filter((a) => a.tabType === activeTab);
  }, [activities, activeTab]);

  return (
    <div className="space-y-6 max-w-7xl mx-auto">
      {/* 1. Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900 flex items-center gap-2">
            <History className="w-6 h-6 text-emerald-600" />
            <span>Activity</span>
          </h1>
          <p className="text-sm text-slate-500 mt-0.5">
            Track portfolio executions, automated rebalance requests, and safety events.
          </p>
        </div>

        <Button
          variant="outline"
          size="sm"
          onClick={loadActivityData}
          disabled={isLoading}
          className="border-slate-300 text-slate-700 text-xs font-semibold self-start sm:self-auto"
        >
          <RefreshCw className={`w-3.5 h-3.5 mr-1.5 ${isLoading ? "animate-spin text-emerald-600" : ""}`} />
          <span>Refresh</span>
        </Button>
      </div>

      {/* 2. Tabs: All | Trades | Rebalances | System */}
      <div className="flex items-center gap-1.5 p-1 bg-slate-100 rounded-lg w-fit">
        {(["All", "Trades", "Rebalances", "System"] as ActivityTab[]).map((tab) => (
          <button
            key={tab}
            type="button"
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-1.5 text-xs font-semibold rounded-md transition-all ${
              activeTab === tab
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900"
            }`}
          >
            {tab}
          </button>
        ))}
      </div>

      {/* 3. Activity Items List */}
      <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
        <CardContent className="p-0">
          {isLoading && activities.length === 0 ? (
            <div className="p-12 text-center text-slate-400 text-sm">
              Loading recent activity...
            </div>
          ) : error ? (
            <div className="p-12 text-center text-slate-500 text-sm">
              {error}
            </div>
          ) : filteredActivities.length === 0 ? (
            <div className="p-12 text-center text-slate-500 text-sm">
              No activity records found for this category.
            </div>
          ) : (
            <>
              {/* Desktop Table View */}
              <div className="hidden md:block overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow className="bg-slate-50/75 border-b border-slate-100">
                      <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600">Action</TableHead>
                      <TableHead className="px-4 py-3 text-xs font-semibold text-slate-600">Asset</TableHead>
                      <TableHead className="px-4 py-3 text-xs font-semibold text-slate-600 text-right">Amount</TableHead>
                      <TableHead className="px-4 py-3 text-xs font-semibold text-slate-600 text-center">Status</TableHead>
                      <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Time</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredActivities.map((item) => {
                      const isExpanded = expandedId === item.id;
                      const hasDetails = Boolean(item.txSignature || item.expectedOutput);

                      return (
                        <React.Fragment key={item.id}>
                          <TableRow
                            onClick={() => hasDetails && setExpandedId(isExpanded ? null : item.id)}
                            className={`transition-colors border-b border-slate-100 ${
                              hasDetails ? "cursor-pointer hover:bg-slate-50/60" : ""
                            } ${isExpanded ? "bg-slate-50/80" : ""}`}
                          >
                            <TableCell className="px-6 py-3.5 font-bold text-slate-900 text-sm">
                              <div className="flex items-center gap-2">
                                <span>{item.action}</span>
                                {hasDetails && (
                                  <span className="text-slate-400">
                                    {isExpanded ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
                                  </span>
                                )}
                              </div>
                            </TableCell>

                            <TableCell className="px-4 py-3.5 text-xs text-slate-700">
                              <span className="px-2 py-0.5 rounded bg-slate-100 text-slate-800 border border-slate-200">
                                {item.asset}
                              </span>
                            </TableCell>

                            <TableCell className="px-4 py-3.5 text-right text-sm font-semibold text-slate-900">
                              {item.amount}
                            </TableCell>

                            <TableCell className="px-4 py-3.5 text-center">
                              {item.status === "Completed" && (
                                <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-emerald-50 text-emerald-700 border border-emerald-200">
                                  <CheckCircle2 className="w-3 h-3" />
                                  <span>Completed</span>
                                </span>
                              )}
                              {item.status === "Pending" && (
                                <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-amber-50 text-amber-700 border border-amber-200">
                                  <Clock className="w-3 h-3" />
                                  <span>Pending</span>
                                </span>
                              )}
                              {item.status === "Failed" && (
                                <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-rose-50 text-rose-700 border border-rose-200">
                                  <XCircle className="w-3 h-3" />
                                  <span>Failed</span>
                                </span>
                              )}
                            </TableCell>

                            <TableCell className="px-6 py-3.5 text-right text-xs text-slate-500">
                              {item.time}
                            </TableCell>
                          </TableRow>

                          {/* TRADE DETAILS EXPANDED VIEW */}
                          {isExpanded && (
                            <TableRow className="bg-slate-50/90 border-b border-slate-200">
                              <TableCell colSpan={5} className="px-6 py-4">
                                <div className="space-y-3 text-xs">
                                  <div className="font-semibold text-slate-800">Trade Execution Details</div>

                                  <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 bg-white p-3.5 rounded-lg border border-slate-200 text-xs">
                                    <div>
                                      <span className="text-slate-400 block">Asset:</span>
                                      <span className="font-bold text-slate-900">{item.asset}</span>
                                    </div>

                                    <div>
                                      <span className="text-slate-400 block">Amount:</span>
                                      <span className="font-bold text-slate-900">{item.amount}</span>
                                    </div>

                                    <div>
                                      <span className="text-slate-400 block">Expected Output:</span>
                                      <span className="text-slate-800">{item.expectedOutput || "—"}</span>
                                    </div>

                                    <div>
                                      <span className="text-slate-400 block">Actual Output:</span>
                                      <span className="font-bold text-emerald-800">{item.actualOutput || "—"}</span>
                                    </div>

                                    <div>
                                      <span className="text-slate-400 block">Fees:</span>
                                      <span className="text-slate-800">{item.fees || "—"}</span>
                                    </div>

                                    <div>
                                      <span className="text-slate-400 block">Status:</span>
                                      <span className="text-slate-800">{item.status}</span>
                                    </div>

                                    <div>
                                      <span className="text-slate-400 block">Confirmation Time:</span>
                                      <span className="text-slate-800">{item.confirmationTime || "—"}</span>
                                    </div>

                                    {item.txSignature && (
                                      <div>
                                        <span className="text-slate-400 block">Transaction Signature:</span>
                                        <div className="flex items-center gap-1.5">
                                          <a
                                            href={`https://explorer.solana.com/tx/${item.txSignature}?cluster=devnet`}
                                            target="_blank"
                                            rel="noreferrer"
                                            className="font-mono text-emerald-700 hover:text-emerald-800 underline truncate max-w-[120px]"
                                          >
                                            {item.txSignature.slice(0, 4)}...{item.txSignature.slice(-4)}
                                          </a>
                                          <button
                                            type="button"
                                            onClick={(e) => handleCopySig(item.txSignature!, e)}
                                            className="p-0.5 text-slate-400 hover:text-slate-600"
                                            title="Copy signature"
                                          >
                                            {copiedSig === item.txSignature ? (
                                              <Check className="w-3 h-3 text-emerald-600" />
                                            ) : (
                                              <Copy className="w-3 h-3" />
                                            )}
                                          </button>
                                        </div>
                                      </div>
                                    )}
                                  </div>
                                </div>
                              </TableCell>
                            </TableRow>
                          )}
                        </React.Fragment>
                      );
                    })}
                  </TableBody>
                </Table>
              </div>

              {/* Mobile Card List */}
              <div className="md:hidden divide-y divide-slate-100">
                {filteredActivities.map((item) => {
                  const isExpanded = expandedId === item.id;
                  const hasDetails = Boolean(item.txSignature || item.expectedOutput);

                  return (
                    <div
                      key={item.id}
                      onClick={() => hasDetails && setExpandedId(isExpanded ? null : item.id)}
                      className="p-4 space-y-2 cursor-pointer hover:bg-slate-50 transition-colors"
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <span className="font-bold text-slate-900 text-sm">{item.action}</span>
                          <span className="text-xs px-1.5 py-0.5 rounded bg-slate-100 text-slate-700 border border-slate-200">
                            {item.asset}
                          </span>
                        </div>
                        <span className="text-xs text-slate-500">{item.time}</span>
                      </div>

                      <div className="flex items-center justify-between text-xs pt-1">
                        <span className="font-bold text-slate-900">{item.amount}</span>
                        <div>
                          {item.status === "Completed" && (
                            <span className="text-emerald-700 font-medium text-xs">Completed</span>
                          )}
                          {item.status === "Pending" && (
                            <span className="text-amber-700 font-medium text-xs">Pending</span>
                          )}
                          {item.status === "Failed" && (
                            <span className="text-rose-700 font-medium text-xs">Failed</span>
                          )}
                        </div>
                      </div>

                      {/* Mobile Trade Details */}
                      {isExpanded && (
                        <div className="mt-3 p-3 bg-slate-50 rounded-lg space-y-1.5 text-xs border border-slate-200">
                          <div className="flex justify-between">
                            <span className="text-slate-500">Expected:</span>
                            <span>{item.expectedOutput || "—"}</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-slate-500">Actual:</span>
                            <span className="font-bold text-emerald-800">{item.actualOutput || "—"}</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-slate-500">Fee:</span>
                            <span>{item.fees || "—"}</span>
                          </div>
                          {item.txSignature && (
                            <div className="flex justify-between items-center pt-1 border-t border-slate-200">
                              <span className="text-slate-500">Signature:</span>
                              <a
                                href={`https://explorer.solana.com/tx/${item.txSignature}?cluster=devnet`}
                                target="_blank"
                                rel="noreferrer"
                                className="font-mono text-emerald-700 underline truncate max-w-[120px]"
                              >
                                {item.txSignature.slice(0, 4)}...{item.txSignature.slice(-4)}
                              </a>
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* 4. Strategy Automation Note */}
      <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
        <CardContent className="p-5 flex items-start gap-3.5">
          <div className="w-10 h-10 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-700 shrink-0">
            <Bot className="w-5 h-5" />
          </div>
          <div className="space-y-1">
            <h3 className="text-sm font-bold text-slate-900">Strategy Automation</h3>
            <p className="text-xs text-slate-600 leading-relaxed">
              The strategy engine can suggest portfolio actions. Final execution remains subject to system rules and safety checks.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* 5. Advanced Technical Views Access */}
      <Card className="bg-slate-50 border-slate-200 rounded-xl">
        <CardContent className="p-5 space-y-3">
          <div className="flex items-center justify-between">
            <div className="text-xs font-bold uppercase tracking-wider text-slate-500">
              Advanced Operational Views
            </div>
            <span className="text-[11px] text-slate-400">Deep technical audits</span>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-5 gap-2">
            {ADVANCED_NAVIGATION.map((nav) => (
              <Link
                key={nav.href}
                href={nav.href}
                className="flex items-center justify-between p-2.5 rounded-lg bg-white border border-slate-200 hover:border-slate-300 text-xs font-semibold text-slate-700 hover:text-slate-900 transition-colors group"
              >
                <span>{nav.label}</span>
                <ArrowUpRight className="w-3.5 h-3.5 text-slate-400 group-hover:text-emerald-600 transition-colors" />
              </Link>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
