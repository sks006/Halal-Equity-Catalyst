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

const DEMO_PROPOSALS: TradeProposal[] = [
  {
    id: "prop_9a1f4b2c-8801-4d1a-bc33-01e4a5d89001",
    vaultAddress: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    vaultName: "Solana Liquid Growth Alpha",
    state: "Executed",
    action: "REBALANCE_BUY",
    symbol: "NVDA",
    inputMintSymbol: "USDC",
    outputMintSymbol: "NVDA",
    amountIn: "50,000 USDC",
    amountOutExpected: "389.10 NVDA",
    minAmountOut: "387.15 NVDA",
    slippageBps: 50,
    reason: "Datacenter revenue guidance outperformance; rebalancing allocation from cash reserves to target 32%.",
    txSignature: "5KtPn4Z8dY3aL6jK9m2Q1v8w7e6r5t4y3u2i1o0p9a8s7d6f5g4h3j2k1l0z9x8c7v6b5n4m3",
    timestamp: "12 mins ago",
    computeUnits: 42150,
  },
  {
    id: "prop_7b2c9d1a-4402-4a9b-bc22-02f5b6e90112",
    vaultAddress: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    vaultName: "Solana Liquid Growth Alpha",
    state: "Simulated",
    action: "REBALANCE_BUY",
    symbol: "AAPL",
    inputMintSymbol: "USDC",
    outputMintSymbol: "AAPL",
    amountIn: "25,000 USDC",
    amountOutExpected: "107.68 AAPL",
    minAmountOut: "107.14 AAPL",
    slippageBps: 50,
    reason: "Narrow confidence band on Pyth oracle; increasing weight towards 25.0% target allocation.",
    timestamp: "28 mins ago",
    computeUnits: 38920,
  },
  {
    id: "prop_3c8e1a5d-1103-4f8c-ba11-03a6c7d01223",
    vaultAddress: "JUP99X8c1V2b3N4EQTYv7cK89Wq3yK9u4J2b8j9Q1M6",
    vaultName: "Jupiter Delta Neutral Yield",
    state: "Approved",
    action: "REDUCE_RISK",
    symbol: "TSLA",
    inputMintSymbol: "TSLA",
    outputMintSymbol: "USDC",
    amountIn: "60.00 TSLA",
    amountOutExpected: "14,748 USDC",
    minAmountOut: "14,674 USDC",
    slippageBps: 50,
    reason: "Volatility expansion trigger hit; de-risking 200 bps back to collateral cash.",
    timestamp: "1 hour ago",
  },
  {
    id: "prop_5d4a2e9f-7704-4c7b-bc44-04b7d8e12334",
    vaultAddress: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    vaultName: "Solana Liquid Growth Alpha",
    state: "Validated",
    action: "REBALANCE_BUY",
    symbol: "SPYx",
    inputMintSymbol: "USDC",
    outputMintSymbol: "SPYx",
    amountIn: "15,000 USDC",
    amountOutExpected: "26.58 SPYx",
    minAmountOut: "26.45 SPYx",
    slippageBps: 50,
    reason: "Quarterly reweighting signal triggered by index rebalancing schedule.",
    timestamp: "2 hours ago",
  },
  {
    id: "prop_2e6b8c1a-5505-4b6a-ba55-05c8e9f23445",
    vaultAddress: "FKsxhTr6QYPc6RQFBR9XL8CsXPBhxKwEdurFwwg7zR6V",
    vaultName: "Catalyst Alpha",
    state: "Proposed",
    action: "REBALANCE_BUY",
    symbol: "MSFT",
    inputMintSymbol: "USDC",
    outputMintSymbol: "MSFT",
    amountIn: "10,000 USDC",
    amountOutExpected: "23.31 MSFT",
    minAmountOut: "23.19 MSFT",
    slippageBps: 50,
    reason: "New cash deposit detected; allocating to core tech basket benchmark.",
    timestamp: "3 hours ago",
  },
  {
    id: "prop_1f9d3b7e-9906-4e5c-ba66-06d9f0a34556",
    vaultAddress: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    vaultName: "Solana Liquid Growth Alpha",
    state: "Failed",
    action: "REBALANCE_BUY",
    symbol: "TSLA",
    inputMintSymbol: "USDC",
    outputMintSymbol: "TSLA",
    amountIn: "120,000 USDC",
    amountOutExpected: "488.20 TSLA",
    minAmountOut: "485.75 TSLA",
    slippageBps: 50,
    reason: "Agent proposed large rebalance trade following earnings announcement.",
    failureReason: "Risk limit violation: Projected position exposure (44.5%) exceeds maximum vault policy limit (40.0%). Deterministically blocked at Approval stage.",
    failedAtStage: "Approval",
    timestamp: "5 hours ago",
  },
  {
    id: "prop_8a5c2f4e-3307-4d4b-ba77-07e0a1b45667",
    vaultAddress: "JUP99X8c1V2b3N4EQTYv7cK89Wq3yK9u4J2b8j9Q1M6",
    vaultName: "Jupiter Delta Neutral Yield",
    state: "Failed",
    action: "SWAP",
    symbol: "NVDA",
    inputMintSymbol: "USDC",
    outputMintSymbol: "NVDA",
    amountIn: "40,000 USDC",
    amountOutExpected: "311.28 NVDA",
    minAmountOut: "309.72 NVDA",
    slippageBps: 50,
    reason: "Arbitrage opportunity detected between Meteora pool and external DEX.",
    failureReason: "Simulation gate failed: Insufficient liquidity on Meteora bonding curve; price impact 142 bps exceeded max tolerance (50 bps).",
    failedAtStage: "Simulation",
    timestamp: "7 hours ago",
  },
];

const STAGES: ProposalLifecycleState[] = [
  "Proposed",
  "Validated",
  "Approved",
  "Simulated",
  "Executed",
];

export default function ProposalsPage() {
  const [proposals, setProposals] = useState<TradeProposal[]>(DEMO_PROPOSALS);
  const [selectedState, setSelectedState] = useState<string>("All");
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [inspectingProposal, setInspectingProposal] = useState<TradeProposal | null>(DEMO_PROPOSALS[0]);

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
        return <Badge variant="secondary" className="font-mono text-[10px] bg-slate-100 text-slate-700">Proposed</Badge>;
      case "Validated":
        return <Badge variant="cyan" className="font-mono text-[10px]">Validated</Badge>;
      case "Approved":
        return <Badge variant="secondary" className="font-mono text-[10px] bg-indigo-50 text-indigo-700 border-indigo-200">Approved</Badge>;
      case "Simulated":
        return <Badge variant="secondary" className="font-mono text-[10px] bg-purple-50 text-purple-700 border-purple-200">Simulated</Badge>;
      case "Executed":
        return <Badge variant="emerald" className="font-mono text-[10px]">Executed</Badge>;
      case "Failed":
        return <Badge variant="destructive" className="font-mono text-[10px]">Failed</Badge>;
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
              Trade Proposal Lifecycle Pipeline
            </h1>
          </div>
          <p className="text-xs text-slate-500 mt-1">
            Deterministic state progression: Proposed $\rightarrow$ Validated $\rightarrow$ Approved $\rightarrow$ Simulated $\rightarrow$ Executed
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Badge variant="cyan" className="font-mono text-xs py-1">
            <Activity className="w-3 h-3 mr-1 animate-pulse" />
            <span>Keeper Pipeline Active</span>
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
          {filteredProposals.map((prop) => {
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
                      <span className="font-bold text-xs font-mono text-slate-900">
                        {prop.action} {prop.symbol}
                      </span>
                      <span className="text-slate-400 text-xs">•</span>
                      <span className="text-xs text-slate-500 font-medium">{prop.vaultName}</span>
                    </div>

                    <span className="text-[10px] text-slate-400 font-mono">{prop.timestamp}</span>
                  </div>

                  {/* Mid line: Quantities & Reason */}
                  <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 text-xs bg-slate-50 p-3 rounded-lg border border-slate-100 font-mono">
                    <div>
                      <span className="text-slate-400 text-[10px] block">Input Amount</span>
                      <span className="font-bold text-slate-900">{prop.amountIn}</span>
                    </div>
                    <div>
                      <span className="text-slate-400 text-[10px] block">Expected Output</span>
                      <span className="font-bold text-slate-900">{prop.amountOutExpected}</span>
                    </div>
                    <div>
                      <span className="text-slate-400 text-[10px] block">Min Out (Max Slippage)</span>
                      <span className="font-bold text-slate-900">{prop.minAmountOut} ({prop.slippageBps} bps)</span>
                    </div>
                  </div>

                  <p className="text-xs text-slate-600 line-clamp-1 leading-relaxed">
                    {prop.reason}
                  </p>

                  {/* Stepper Pipeline Bar */}
                  <div className="pt-2 border-t border-slate-100">
                    {prop.state === "Failed" ? (
                      <div className="flex items-center justify-between bg-rose-50 text-rose-700 px-3 py-1.5 rounded text-xs font-mono border border-rose-200">
                        <div className="flex items-center gap-1.5">
                          <XCircle className="w-3.5 h-3.5 text-rose-600" />
                          <span className="font-bold">Rejected at {prop.failedAtStage || "Execution"} Gate</span>
                        </div>
                        <span className="text-[11px] truncate max-w-[280px]">{prop.failureReason}</span>
                      </div>
                    ) : (
                      <div className="grid grid-cols-5 gap-1.5 text-center text-[10px] font-mono">
                        {STAGES.map((stage, idx) => {
                          const isDone = idx <= currentStageIdx;
                          const isCurrent = idx === currentStageIdx;
                          return (
                            <div
                              key={stage}
                              className={`py-1 rounded font-semibold transition-colors ${
                                isDone
                                  ? isCurrent && stage === "Executed"
                                    ? "bg-emerald-600 text-white font-bold"
                                    : "bg-emerald-50 text-emerald-700 border border-emerald-200 font-bold"
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
          })}
        </div>

        {/* Right Column: Detailed Proposal Inspector Drawer */}
        <div className="space-y-4">
          {inspectingProposal ? (
            <Card className="bg-white border-slate-200 shadow-sm overflow-hidden sticky top-20">
              <CardHeader className="p-5 border-b border-slate-100 bg-gradient-to-r from-slate-50 to-white">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono">
                    Proposal Inspector
                  </span>
                  {getStateBadge(inspectingProposal.state)}
                </div>
                <CardTitle className="text-base font-bold text-slate-900 mt-2 font-mono">
                  {inspectingProposal.action} {inspectingProposal.symbol}
                </CardTitle>
                <div className="text-[10px] text-slate-400 font-mono mt-0.5 truncate">
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
                    <span className="text-rose-600 font-bold block mb-1 flex items-center gap-1">
                      <AlertTriangle className="w-3.5 h-3.5" />
                      <span>Rejection Diagnostic Trace</span>
                    </span>
                    <div className="p-3 rounded-lg bg-rose-50 border border-rose-200 text-rose-800 text-xs font-mono leading-relaxed">
                      {inspectingProposal.failureReason}
                    </div>
                  </div>
                )}

                <div className="space-y-2 border-t border-slate-100 pt-3">
                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Target Vault:</span>
                    <span className="font-mono text-slate-900 font-medium truncate max-w-[180px]">
                      {inspectingProposal.vaultName}
                    </span>
                  </div>

                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Input Collateral:</span>
                    <span className="font-mono font-bold text-slate-900">{inspectingProposal.amountIn}</span>
                  </div>

                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Expected Execution:</span>
                    <span className="font-mono font-bold text-slate-900">{inspectingProposal.amountOutExpected}</span>
                  </div>

                  <div className="flex items-center justify-between">
                    <span className="text-slate-500">Slippage Bound:</span>
                    <span className="font-mono font-bold text-emerald-600">{inspectingProposal.slippageBps} bps</span>
                  </div>

                  {inspectingProposal.computeUnits && (
                    <div className="flex items-center justify-between">
                      <span className="text-slate-500">Compute Units:</span>
                      <span className="font-mono font-bold text-purple-600">
                        {inspectingProposal.computeUnits.toLocaleString()} CU
                      </span>
                    </div>
                  )}

                  {inspectingProposal.txSignature && (
                    <div className="pt-2 border-t border-slate-100 space-y-1">
                      <span className="text-slate-500 block">On-Chain Transaction:</span>
                      <div className="flex items-center justify-between p-2 rounded bg-slate-50 font-mono text-[11px] text-slate-700">
                        <span className="truncate max-w-[200px]">{inspectingProposal.txSignature}</span>
                        <a
                          href={`https://explorer.solana.com/tx/${inspectingProposal.txSignature}?cluster=devnet`}
                          target="_blank"
                          rel="noreferrer"
                          className="text-emerald-600 hover:text-emerald-700 shrink-0 ml-1"
                        >
                          <ExternalLink className="w-3 h-3" />
                        </a>
                      </div>
                    </div>
                  )}
                </div>
              </CardContent>
            </Card>
          ) : (
            <Card className="bg-white p-8 text-center text-slate-400 text-xs">
              Select a proposal to inspect detailed lifecycle parameters.
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
