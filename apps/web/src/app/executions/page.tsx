"use client";

import React, { useState, useEffect } from "react";
import {
  Activity,
  AlertCircle,
  ArrowRight,
  CheckCircle2,
  Clock,
  Copy,
  ExternalLink,
  Filter,
  History,
  Loader2,
  RefreshCw,
  Search,
  ShieldCheck,
  TrendingDown,
  TrendingUp,
  X,
  XCircle,
} from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../../components/ui/table";
import { getApiClient, ExecutionModel } from "../../lib/api-client";

interface EnrichedExecution extends ExecutionModel {
  symbol: string;
  vaultName: string;
  reconciliationDriftBps: number;
  confirmationDurationSec?: number;
}

export default function ExecutionsPage() {
  const [executions, setExecutions] = useState<EnrichedExecution[]>([]);
  const [statusFilter, setStatusFilter] = useState<string>("All");
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [copiedSig, setCopiedSig] = useState<string | null>(null);
  const [isRefreshing, setIsRefreshing] = useState<boolean>(false);
  const [selectedExecution, setSelectedExecution] = useState<EnrichedExecution | null>(null);

  const syncExecutions = async () => {
    setIsRefreshing(true);
    try {
      const client = getApiClient();
      const liveList = await client.listExecutions();
      if (liveList && liveList.length > 0) {
        const enriched: EnrichedExecution[] = liveList.map((exec) => ({
          ...exec,
          symbol: "ASSET",
          vaultName: "Anchor Vault",
          reconciliationDriftBps: 0,
        }));
        setExecutions(enriched);
        if (!selectedExecution && enriched.length > 0) {
          setSelectedExecution(enriched[0]);
        }
      } else {
        setExecutions([]);
        setSelectedExecution(null);
      }
    } catch {
      // Fail closed: do not show fake synthetic executions
      setExecutions([]);
      setSelectedExecution(null);
    } finally {
      setIsRefreshing(false);
    }
  };

  useEffect(() => {
    syncExecutions();
  }, []);

  const handleCopy = (sig: string) => {
    navigator.clipboard.writeText(sig);
    setCopiedSig(sig);
    setTimeout(() => setCopiedSig(null), 2000);
  };

  const filteredExecutions = executions.filter((exec) => {
    const matchesStatus = statusFilter === "All" || exec.status.toLowerCase() === statusFilter.toLowerCase();
    const matchesSearch =
      exec.symbol.toLowerCase().includes(searchQuery.toLowerCase()) ||
      exec.action.toLowerCase().includes(searchQuery.toLowerCase()) ||
      exec.execution_id.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (exec.tx_signature && exec.tx_signature.toLowerCase().includes(searchQuery.toLowerCase())) ||
      exec.vaultName.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesStatus && matchesSearch;
  });

  const getStatusBadge = (status: string) => {
    switch (status.toLowerCase()) {
      case "confirmed":
        return (
          <Badge variant="emerald" className="font-mono text-[10px] flex items-center gap-1">
            <CheckCircle2 className="w-3 h-3" />
            <span>CONFIRMED</span>
          </Badge>
        );
      case "submitted":
        return (
          <Badge variant="warning" className="font-mono text-[10px] flex items-center gap-1">
            <Loader2 className="w-3 h-3 animate-spin" />
            <span>SUBMITTED</span>
          </Badge>
        );
      case "simulated":
        return (
          <Badge variant="secondary" className="font-mono text-[10px] bg-purple-50 text-purple-700 border-purple-200">
            SIMULATED
          </Badge>
        );
      case "failed":
        return (
          <Badge variant="destructive" className="font-mono text-[10px] flex items-center gap-1">
            <XCircle className="w-3 h-3" />
            <span>FAILED</span>
          </Badge>
        );
      default:
        return <Badge variant="secondary" className="font-mono text-[10px]">{status.toUpperCase()}</Badge>;
    }
  };

  return (
    <div className="space-y-8">
      {/* 1. Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-emerald-50 text-emerald-600 flex items-center justify-center">
              <History className="w-4 h-4" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight text-slate-900">
              Transaction Execution Status & Ledger
            </h1>
          </div>
          <p className="text-xs text-slate-500 mt-1">
            On-chain Solana Anchor executions, cryptographic keeper signatures, and post-execution slippage reconciliation
          </p>
        </div>

        <div className="flex items-center gap-3">
          <Button
            variant="outline"
            size="sm"
            onClick={syncExecutions}
            disabled={isRefreshing}
            className="flex items-center gap-1.5 text-xs font-medium"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isRefreshing ? "animate-spin text-emerald-600" : ""}`} />
            <span>{isRefreshing ? "Syncing..." : "Sync Ledger"}</span>
          </Button>

          <Badge variant="cyan" className="font-mono text-xs py-1">
            <Activity className="w-3 h-3 mr-1 animate-pulse" />
            <span>Solana Devnet</span>
          </Badge>
        </div>
      </div>

      {/* 2. Filter & Search Controls */}
      <Card className="bg-white p-3 border-slate-200 shadow-sm space-y-3">
        <div className="flex flex-col sm:flex-row items-center justify-between gap-3">
          <div className="flex items-center gap-1.5 overflow-x-auto w-full sm:w-auto py-0.5">
            {["All", "Confirmed", "Submitted", "Failed"].map((st) => {
              const count =
                st === "All"
                  ? executions.length
                  : executions.filter((e) => e.status.toLowerCase() === st.toLowerCase()).length;
              const isActive = statusFilter === st;
              return (
                <button
                  key={st}
                  onClick={() => setStatusFilter(st)}
                  className={`px-3 py-1.5 rounded-lg text-xs font-semibold font-mono transition-all flex items-center gap-1.5 shrink-0 ${
                    isActive
                      ? "bg-slate-900 text-white shadow-sm"
                      : "text-slate-600 hover:text-slate-900 hover:bg-slate-100"
                  }`}
                >
                  <span>{st}</span>
                  <span className={`text-[10px] px-1.5 py-0.2 rounded-full ${
                    isActive ? "bg-slate-800 text-emerald-400" : "bg-slate-100 text-slate-500"
                  }`}>
                    {count}
                  </span>
                </button>
              );
            })}
          </div>

          <div className="relative w-full sm:w-72">
            <Search className="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <Input
              type="text"
              placeholder="Search signature, ticker, ID..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 bg-slate-50/60 border-slate-200 text-xs h-9"
            />
          </div>
        </div>
      </Card>

      {/* 3. Executions Table */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-3">
          <Card className="bg-white border-slate-200 shadow-sm overflow-hidden">
            {filteredExecutions.length > 0 ? (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow className="bg-slate-50/75">
                      <TableHead className="text-xs font-bold text-slate-700">Status</TableHead>
                      <TableHead className="text-xs font-bold text-slate-700">Action & Asset</TableHead>
                      <TableHead className="text-xs font-bold text-slate-700">Signature</TableHead>
                      <TableHead className="text-xs font-bold text-slate-700 text-right">Amounts</TableHead>
                      <TableHead className="text-xs font-bold text-slate-700 text-right">Reconciliation</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredExecutions.map((exec) => {
                      const isSelected = selectedExecution?.execution_id === exec.execution_id;
                      const isImprovement = exec.reconciliationDriftBps >= 0;

                      return (
                        <TableRow
                          key={exec.execution_id}
                          onClick={() => setSelectedExecution(exec)}
                          className={`cursor-pointer transition-colors ${
                            isSelected ? "bg-emerald-50/40" : "hover:bg-slate-50/50"
                          }`}
                        >
                          <TableCell>{getStatusBadge(exec.status)}</TableCell>

                          <TableCell>
                            <div>
                              <div className="font-bold text-xs text-slate-900 font-mono">
                                {exec.action} {exec.symbol}
                              </div>
                              <div className="text-[10px] text-slate-400 truncate max-w-[140px]">
                                {exec.vaultName}
                              </div>
                            </div>
                          </TableCell>

                          <TableCell>
                            {exec.tx_signature ? (
                              <div className="flex items-center gap-1.5 font-mono text-xs">
                                <span className="truncate max-w-[130px] text-slate-700">
                                  {exec.tx_signature}
                                </span>
                                <a
                                  href={`https://explorer.solana.com/tx/${exec.tx_signature}?cluster=devnet`}
                                  target="_blank"
                                  rel="noreferrer"
                                  onClick={(e) => e.stopPropagation()}
                                  className="text-emerald-600 hover:text-emerald-700"
                                >
                                  <ExternalLink className="w-3.5 h-3.5" />
                                </a>
                              </div>
                            ) : (
                              <span className="text-slate-400 font-mono text-xs">—</span>
                            )}
                          </TableCell>

                          <TableCell className="text-right text-xs font-mono">
                            <div>
                              <span className="font-semibold text-slate-900">
                                {(exec.amount_in / 1_000_000).toLocaleString(undefined, { maximumFractionDigits: 1 })} USDC
                              </span>
                              <span className="text-slate-400 block text-[10px]">
                                $\rightarrow$ {exec.amount_out_actual ? (exec.amount_out_actual / 1_000_000).toFixed(2) : (exec.amount_out_expected / 1_000_000).toFixed(2)} {exec.symbol}
                              </span>
                            </div>
                          </TableCell>

                          <TableCell className="text-right text-xs font-mono">
                            {exec.status === "confirmed" ? (
                              <span className={`font-bold inline-flex items-center gap-0.5 ${
                                isImprovement ? "text-emerald-600" : "text-amber-600"
                              }`}>
                                {isImprovement ? <TrendingUp className="w-3 h-3" /> : <TrendingDown className="w-3 h-3" />}
                                <span>{isImprovement ? "+" : ""}{exec.reconciliationDriftBps} bps</span>
                              </span>
                            ) : (
                              <span className="text-slate-400">—</span>
                            )}
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              </div>
            ) : (
              <div className="p-12 text-center text-slate-400 text-xs">
                No execution records found on-chain.
              </div>
            )}
          </Card>
        </div>

        {/* Right Column: Execution Diagnostic & Signature Inspector */}
        <div className="space-y-4">
          {selectedExecution ? (
            <Card className="bg-white border-slate-200 shadow-sm overflow-hidden sticky top-20">
              <CardHeader className="p-5 border-b border-slate-100 bg-gradient-to-r from-slate-50 to-white">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono">
                    Execution Receipt
                  </span>
                  {getStatusBadge(selectedExecution.status)}
                </div>
                <CardTitle className="text-base font-bold text-slate-900 mt-2 font-mono">
                  {selectedExecution.action} {selectedExecution.symbol}
                </CardTitle>
                <div className="text-[10px] text-slate-400 font-mono mt-0.5 truncate">
                  {selectedExecution.execution_id}
                </div>
              </CardHeader>

              <CardContent className="p-5 space-y-4 text-xs">
                {/* Signature details */}
                {selectedExecution.tx_signature && (
                  <div className="space-y-1.5">
                    <span className="text-slate-500 font-semibold block">Solana Transaction Signature</span>
                    <div className="p-3 rounded-lg bg-slate-50 border border-slate-200 font-mono text-[11px] text-slate-800 break-all relative">
                      {selectedExecution.tx_signature}
                      <div className="mt-2 pt-2 border-t border-slate-200 flex items-center justify-between">
                        <button
                          onClick={() => handleCopy(selectedExecution.tx_signature!)}
                          className="inline-flex items-center gap-1 text-slate-500 hover:text-slate-800"
                        >
                          <Copy className="w-3 h-3" />
                          <span>{copiedSig === selectedExecution.tx_signature ? "Copied!" : "Copy"}</span>
                        </button>
                        <a
                          href={`https://explorer.solana.com/tx/${selectedExecution.tx_signature}?cluster=devnet`}
                          target="_blank"
                          rel="noreferrer"
                          className="inline-flex items-center gap-1 text-emerald-600 font-bold hover:text-emerald-700"
                        >
                          <span>Solana Explorer</span>
                          <ExternalLink className="w-3 h-3" />
                        </a>
                      </div>
                    </div>
                  </div>
                )}

                {/* Error diagnostics */}
                {selectedExecution.error_message && (
                  <div>
                    <span className="text-rose-600 font-bold block mb-1 flex items-center gap-1">
                      <AlertCircle className="w-3.5 h-3.5" />
                      <span>Execution Halt Reason</span>
                    </span>
                    <div className="p-3 rounded-lg bg-rose-50 border border-rose-200 text-rose-800 font-mono text-xs leading-relaxed">
                      {selectedExecution.error_message}
                    </div>
                  </div>
                )}

                {/* Execution parameters */}
                <div className="space-y-2 border-t border-slate-100 pt-3 font-mono">
                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Vault Account:</span>
                    <span className="text-slate-900 truncate max-w-[170px]">{selectedExecution.vault_address}</span>
                  </div>

                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Input Collateral:</span>
                    <span className="font-bold text-slate-900">
                      {(selectedExecution.amount_in / 1_000_000).toLocaleString()} USDC
                    </span>
                  </div>

                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Expected Output:</span>
                    <span className="font-bold text-slate-900">
                      {(selectedExecution.amount_out_expected / 1_000_000).toFixed(4)} {selectedExecution.symbol}
                    </span>
                  </div>

                  {selectedExecution.amount_out_actual && (
                    <div className="flex items-center justify-between">
                      <span className="text-slate-500">Actual Output Received:</span>
                      <span className="font-bold text-emerald-600">
                        {(selectedExecution.amount_out_actual / 1_000_000).toFixed(4)} {selectedExecution.symbol}
                      </span>
                    </div>
                  )}

                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Slippage Tolerance:</span>
                    <span className="text-slate-700">{selectedExecution.slippage_bps} bps</span>
                  </div>

                  {selectedExecution.confirmationDurationSec && (
                    <div className="flex items-center justify-between">
                      <span className="text-slate-500">Confirmation Latency:</span>
                      <span className="text-emerald-600 font-bold">{selectedExecution.confirmationDurationSec}s</span>
                    </div>
                  )}
                </div>
              </CardContent>
            </Card>
          ) : (
            <Card className="bg-white p-8 text-center text-slate-400 text-xs">
              Select an execution record to view signature and receipt.
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
