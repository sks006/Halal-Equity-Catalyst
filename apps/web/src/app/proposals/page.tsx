"use client";

import React, { useState } from "react";
import Link from "next/link";
import {
  Activity,
  AlertCircle,
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  Clock,
  ExternalLink,
  Eye,
  FileCheck2,
  Filter,
  Layers,
  Play,
  RefreshCw,
  Search,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  TrendingUp,
  XCircle,
} from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";

export type ProposalLifecycleState =
  | "Proposed"
  | "Validated"
  | "Approved"
  | "Simulated"
  | "Executed"
  | "Failed";

export interface TradeProposal {
  id: string;
  vaultAddress: string;
  vaultName: string;
  state: ProposalLifecycleState;
  action: string;
  symbol: string;
  inputMintSymbol: string;
  outputMintSymbol: string;
  amountIn: string;
  amountOutExpected: string;
  minAmountOut: string;
  slippageBps: number;
  reason: string;
  failureReason?: string;
  failedAtStage?: "Validation" | "Approval" | "Simulation" | "Execution";
  txSignature?: string;
  timestamp: string;
  computeUnits?: number;
}

const STAGES: ProposalLifecycleState[] = [
  "Proposed",
  "Validated",
  "Approved",
  "Simulated",
  "Executed",
];

export default function ProposalsPage() {
  const [proposals, setProposals] = useState<TradeProposal[]>([]);
  const [selectedState, setSelectedState] = useState<string>("All");
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [inspectingProposal, setInspectingProposal] = useState<TradeProposal | null>(null);

  const filterStates: Array<"All" | ProposalLifecycleState> = [
    "All",
    "Proposed",
    "Validated",
    "Approved",
    "Simulated",
    "Executed",
    "Failed",
  ];

  const filteredProposals = proposals.filter((p) => {
    const matchesState = selectedState === "All" || p.state === selectedState;
    const matchesSearch =
      p.symbol.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.action.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.vaultName.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesState && matchesSearch;
  });

  const getStateBadge = (state: ProposalLifecycleState) => {
    switch (state) {
      case "Proposed":
        return <Badge variant="secondary" className="text-xs bg-slate-100 text-slate-700">Proposed</Badge>;
      case "Validated":
        return <Badge variant="cyan" className="text-xs">Validated</Badge>;
      case "Approved":
        return <Badge variant="secondary" className="text-xs bg-indigo-50 text-indigo-700 border-indigo-200">Approved</Badge>;
      case "Simulated":
        return <Badge variant="secondary" className="text-xs bg-purple-50 text-purple-700 border-purple-200">Simulated</Badge>;
      case "Executed":
        return <Badge variant="emerald" className="text-xs">Executed</Badge>;
      case "Failed":
        return <Badge variant="destructive" className="text-xs">Failed</Badge>;
    }
  };

  const getStageIndex = (state: ProposalLifecycleState): number => {
    switch (state) {
      case "Proposed":
        return 0;
      case "Validated":
        return 1;
      case "Approved":
        return 2;
      case "Simulated":
        return 3;
      case "Executed":
        return 4;
      case "Failed":
        return -1;
    }
  };

  return (
    <div className="space-y-8">
      {/* 1. Header & Summary Stats */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-emerald-50 text-emerald-600 flex items-center justify-center">
              <FileCheck2 className="w-4 h-4" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight text-slate-900">
              Review status
            </h1>
          </div>
          <p className="text-xs text-slate-500 mt-1">
            Deterministic state progression: Proposed &rarr; Validated &rarr; Approved &rarr; Simulated &rarr; Executed
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Badge variant="cyan" className="text-xs py-1">
            <Activity className="w-3.5 h-3.5 mr-1" />
            <span>Review status active</span>
          </Badge>
        </div>
      </div>

      {/* 2. Filter Tabs Bar */}
      <Card className="bg-white p-3 border-slate-200 shadow-sm space-y-3">
        <div className="flex flex-col sm:flex-row items-center justify-between gap-3">
          <div className="flex items-center gap-1.5 overflow-x-auto w-full sm:w-auto py-0.5">
            {filterStates.map((st) => {
              const count = st === "All" ? proposals.length : proposals.filter((p) => p.state === st).length;
              const isActive = selectedState === st;
              return (
                <button
                  key={st}
                  onClick={() => setSelectedState(st)}
                  className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-all flex items-center gap-1.5 shrink-0 ${
                    isActive
                      ? "bg-slate-900 text-white shadow-sm"
                      : "text-slate-600 hover:text-slate-900 hover:bg-slate-100"
                  }`}
                >
                  <span>{st}</span>
                  <span className={`text-xs px-1.5 py-0.2 rounded-full ${
                    isActive ? "bg-slate-800 text-emerald-400" : "bg-slate-100 text-slate-500"
                  }`}>
                    {count}
                  </span>
                </button>
              );
            })}
          </div>

          <div className="relative w-full sm:w-64">
            <Search className="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <Input
              type="text"
              placeholder="Search ticker, vault, ID..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 bg-slate-50/60 border-slate-200 text-xs h-9"
            />
          </div>
        </div>
      </Card>

      {/* 3. Main Proposals View (List & Detail Inspector) */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left Column: Proposals List */}
        <div className="lg:col-span-2 space-y-3">
          {filteredProposals.length > 0 ? (
            filteredProposals.map((prop) => {
              const isSelected = inspectingProposal?.id === prop.id;
              const currentStageIdx = getStageIndex(prop.state);

              return (
                <Card
                  key={prop.id}
                  onClick={() => setInspectingProposal(prop)}
                  className={`bg-white border transition-all cursor-pointer overflow-hidden ${
                    isSelected
                      ? "border-emerald-500 shadow-md ring-1 ring-emerald-500/20"
                      : "border-slate-200 hover:border-slate-300 shadow-sm"
                  }`}
                >
                  <CardContent className="p-5 space-y-4">
                    {/* Top line: State, Action, ID, Time */}
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        {getStateBadge(prop.state)}
                        <span className="font-semibold text-xs text-slate-900">
                          {prop.action} {prop.symbol}
                        </span>
                        <span className="text-slate-400 text-xs">•</span>
                        <span className="text-xs text-slate-500 font-medium">{prop.vaultName}</span>
                      </div>

                      <span className="text-xs text-slate-400">{prop.timestamp}</span>
                    </div>

                    {/* Mid line: Quantities & Reason */}
                    <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 text-xs bg-slate-50 p-3 rounded-lg border border-slate-100">
                      <div>
                        <span className="text-slate-400 text-xs block">Input Amount</span>
                        <span className="font-semibold text-slate-900">{prop.amountIn}</span>
                      </div>
                      <div>
                        <span className="text-slate-400 text-xs block">Expected Output</span>
                        <span className="font-semibold text-slate-900">{prop.amountOutExpected}</span>
                      </div>
                      <div>
                        <span className="text-slate-400 text-xs block">Min Out (Max Slippage)</span>
                        <span className="font-semibold text-slate-900">{prop.minAmountOut} ({prop.slippageBps} bps)</span>
                      </div>
                    </div>

                    <p className="text-xs text-slate-600 line-clamp-1 leading-relaxed">
                      {prop.reason}
                    </p>

                    {/* Stepper Pipeline Bar */}
                    <div className="pt-2 border-t border-slate-100">
                      {prop.state === "Failed" ? (
                        <div className="flex items-center justify-between bg-rose-50 text-rose-700 px-3 py-1.5 rounded text-xs border border-rose-200">
                          <div className="flex items-center gap-1.5">
                            <XCircle className="w-3.5 h-3.5 text-rose-600" />
                            <span className="font-semibold">Rejected at {prop.failedAtStage || "Execution"} Gate</span>
                          </div>
                          <span className="text-xs truncate max-w-[280px]">{prop.failureReason}</span>
                        </div>
                      ) : (
                        <div className="grid grid-cols-5 gap-1.5 text-center text-xs">
                          {STAGES.map((stage, idx) => {
                            const isDone = idx <= currentStageIdx;
                            const isCurrent = idx === currentStageIdx;
                            return (
                              <div
                                key={stage}
                                className={`py-1 rounded font-medium transition-colors ${
                                  isDone
                                    ? isCurrent && stage === "Executed"
                                      ? "bg-emerald-600 text-white font-semibold"
                                      : "bg-emerald-50 text-emerald-700 border border-emerald-200 font-semibold"
                                    : "bg-slate-100 text-slate-400"
                                }`}
                              >
                                {stage}
                              </div>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  </CardContent>
                </Card>
              );
            })
          ) : (
            <Card className="bg-white p-12 text-center text-slate-400 text-xs">
              No trade proposals in pipeline. Keeper policy engines generate proposals on market signal drift.
            </Card>
          )}
        </div>

        {/* Right Column: Detailed Proposal Inspector Drawer */}
        <div className="space-y-4">
          {inspectingProposal ? (
            <Card className="bg-white border-slate-200 shadow-sm overflow-hidden sticky top-20">
              <CardHeader className="p-5 border-b border-slate-100 bg-gradient-to-r from-slate-50 to-white">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-slate-500">
                    Proposal Inspector
                  </span>
                  {getStateBadge(inspectingProposal.state)}
                </div>
                <CardTitle className="text-base font-semibold text-slate-900 mt-2">
                  {inspectingProposal.action} {inspectingProposal.symbol}
                </CardTitle>
                <div className="text-xs text-slate-400 font-mono mt-0.5 truncate">
                  {inspectingProposal.id}
                </div>
              </CardHeader>

              <CardContent className="p-5 space-y-4 text-xs">
                <div>
                  <span className="text-slate-500 font-semibold block mb-1">Trigger Rationale</span>
                  <p className="p-3 rounded-lg bg-slate-50 border border-slate-200 text-slate-700 leading-relaxed text-xs">
                    {inspectingProposal.reason}
                  </p>
                </div>

                {inspectingProposal.failureReason && (
                  <div>
                    <span className="text-rose-600 font-semibold block mb-1 flex items-center gap-1">
                      <AlertTriangle className="w-3.5 h-3.5" />
                      <span>Rejection Diagnostic Trace</span>
                    </span>
                    <div className="p-3 rounded-lg bg-rose-50 border border-rose-200 text-rose-800 text-xs leading-relaxed">
                      {inspectingProposal.failureReason}
                    </div>
                  </div>
                )}

                <div className="space-y-2 border-t border-slate-100 pt-3">
                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Target Vault:</span>
                    <span className="text-slate-900 font-medium truncate max-w-[180px]">
                      {inspectingProposal.vaultName}
                    </span>
                  </div>

                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Input Collateral:</span>
                    <span className="font-semibold text-slate-900">{inspectingProposal.amountIn}</span>
                  </div>

                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Expected Execution:</span>
                    <span className="font-semibold text-slate-900">{inspectingProposal.amountOutExpected}</span>
                  </div>

                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Slippage Bound:</span>
                    <span className="font-semibold text-emerald-600">{inspectingProposal.slippageBps} bps</span>
                  </div>

                  {inspectingProposal.computeUnits && (
                    <div className="flex items-center justify-between">
                      <span className="text-slate-500">Compute Units:</span>
                      <span className="font-semibold text-purple-600">
                        {inspectingProposal.computeUnits.toLocaleString()} CU
                      </span>
                    </div>
                  )}

                  {inspectingProposal.txSignature && (
                    <div className="pt-2 border-t border-slate-100 space-y-1">
                      <span className="text-slate-500 block">On-Chain Transaction:</span>
                      <div className="flex items-center justify-between p-2 rounded bg-slate-50 font-mono text-xs text-slate-700">
                        <span className="truncate max-w-[200px]">{inspectingProposal.txSignature}</span>
                        <a
                          href={`https://explorer.solana.com/tx/${inspectingProposal.txSignature}?cluster=devnet`}
                          target="_blank"
                          rel="noreferrer"
                          className="text-emerald-600 hover:text-emerald-700 shrink-0 ml-1"
                        >
                          <ExternalLink className="w-3.5 h-3.5" />
                        </a>
                      </div>
                    </div>
                  )}
                </div>
              </CardContent>
            </Card>
          ) : (
            <Card className="bg-white p-8 text-center text-slate-400 text-xs">
              Select a proposal to inspect review status.
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
