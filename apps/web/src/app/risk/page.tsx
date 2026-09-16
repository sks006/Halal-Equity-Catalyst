"use client";

import React, { useState } from "react";
import Link from "next/link";
import {
  Activity,
  AlertCircle,
  AlertTriangle,
  CheckCircle2,
  Clock,
  Cpu,
  Flame,
  Layers,
  Play,
  RefreshCw,
  Shield,
  ShieldAlert,
  ShieldCheck,
  Wallet,
  WifiOff,
  XCircle,
  Zap,
} from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Progress } from "../../components/ui/progress";
import { ErrorAlert, ErrorCategory } from "../../components/ErrorAlert";

export default function RiskPage() {
  const [activeErrorDemo, setActiveErrorDemo] = useState<ErrorCategory | null>("stale_market_data");
  const [isSimulatingCheck, setIsSimulatingCheck] = useState<boolean>(false);

  const triggerTestError = (cat: ErrorCategory) => {
    setActiveErrorDemo(cat);
  };

  return (
    <div className="space-y-8">
      {/* 1. Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-emerald-50 text-emerald-600 flex items-center justify-center">
              <ShieldAlert className="w-4 h-4" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight text-slate-900">
              Risk Sentinel & Error State Matrix
            </h1>
          </div>
          <p className="text-xs text-slate-500 mt-1">
            Deterministic second line of defense: Anchor CPI boundaries, preflight simulation gates, and error fail-safes
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Badge variant="emerald" className="font-mono text-xs py-1">
            <ShieldCheck className="w-3 h-3 mr-1" />
            <span>Circuit Breakers Armed</span>
          </Badge>
        </div>
      </div>

      {/* 2. Real-Time Risk Monitor Overview */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card className="bg-white border-slate-200 shadow-sm">
          <CardContent className="p-5">
            <div className="text-xs text-slate-500 font-medium">Position Concentration</div>
            <div className="mt-2 text-2xl font-extrabold font-mono text-slate-900">32.8%</div>
            <Progress value={32.8} max={40} className="h-1.5 mt-2 bg-slate-100" />
            <div className="mt-2 flex items-center justify-between text-[11px]">
              <span className="text-slate-400">Policy Ceiling:</span>
              <span className="font-mono font-bold text-slate-800">40.0% Max</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-white border-slate-200 shadow-sm">
          <CardContent className="p-5">
            <div className="text-xs text-slate-500 font-medium">Portfolio Drawdown</div>
            <div className="mt-2 text-2xl font-extrabold font-mono text-slate-900">1.85%</div>
            <Progress value={1.85} max={10} className="h-1.5 mt-2 bg-slate-100" />
            <div className="mt-2 flex items-center justify-between text-[11px]">
              <span className="text-slate-400">Circuit Breaker:</span>
              <span className="font-mono font-bold text-rose-600">10.0% Max</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-white border-slate-200 shadow-sm">
          <CardContent className="p-5">
            <div className="text-xs text-slate-500 font-medium">Collateral Cash Reserve</div>
            <div className="mt-2 text-2xl font-extrabold font-mono text-slate-900">14.3%</div>
            <Progress value={14.3} max={30} className="h-1.5 mt-2 bg-slate-100" />
            <div className="mt-2 flex items-center justify-between text-[11px]">
              <span className="text-slate-400">Mandatory Floor:</span>
              <span className="font-mono font-bold text-emerald-600">10.0% Min</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-white border-slate-200 shadow-sm">
          <CardContent className="p-5">
            <div className="text-xs text-slate-500 font-medium">Simulation Rejection Rate</div>
            <div className="mt-2 text-2xl font-extrabold font-mono text-emerald-600">0.0%</div>
            <div className="mt-2 text-xs font-mono text-slate-500">
              100% preflight simulated before RPC broadcast
            </div>
          </CardContent>
        </Card>
      </div>

      {/* 3. STEP 07.8 — ERROR STATES HANDLING SUITE */}
      <Card className="bg-white border-slate-200 shadow-sm overflow-hidden">
        <CardHeader className="p-6 border-b border-slate-100 bg-gradient-to-r from-slate-50 to-white">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <div>
              <CardTitle className="text-base font-bold text-slate-900 flex items-center gap-2">
                <AlertTriangle className="w-5 h-5 text-amber-500" />
                <span>Deterministic Error State Handling Suite (Step 07.8)</span>
              </CardTitle>
              <p className="text-xs text-slate-500 mt-1">
                The frontend intercepts, explains, and safely recovers from all 6 critical error states without crashing.
              </p>
            </div>
          </div>
        </CardHeader>

        <CardContent className="p-6 space-y-6">
          {/* Active Error Banner Preview */}
          {activeErrorDemo && (
            <div className="space-y-2">
              <span className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono">
                Active User Alert Presentation
              </span>
              <ErrorAlert
                category={activeErrorDemo}
                onRetry={() => {
                  alert(`Retry action dispatched for: ${activeErrorDemo}`);
                }}
                onDismiss={() => setActiveErrorDemo(null)}
              />
            </div>
          )}

          {/* Interactive Trigger Matrix */}
          <div>
            <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 font-mono mb-3">
              Simulate & Test Core Error Scenarios
            </h3>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {/* 1. Stale Market Data */}
              <div
                onClick={() => triggerTestError("stale_market_data")}
                className={`cursor-pointer p-4 rounded-xl border transition-all ${
                  activeErrorDemo === "stale_market_data"
                    ? "bg-amber-50/60 border-amber-400 shadow-sm ring-1 ring-amber-400/30"
                    : "bg-slate-50 hover:bg-slate-100/80 border-slate-200"
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-lg bg-amber-100 text-amber-700 flex items-center justify-center">
                    <Clock className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-xs font-bold text-slate-900 font-mono">1. Stale Market Data</h4>
                    <span className="text-[11px] text-amber-700 font-medium">Pyth Age &gt; 15s</span>
                  </div>
                </div>
                <p className="text-xs text-slate-600 mt-2.5 leading-relaxed">
                  Interprets Pyth publisher timestamps. Halts autonomous trade executions immediately when feeds exceed latency threshold.
                </p>
              </div>

              {/* 2. RPC Failure */}
              <div
                onClick={() => triggerTestError("rpc_failure")}
                className={`cursor-pointer p-4 rounded-xl border transition-all ${
                  activeErrorDemo === "rpc_failure"
                    ? "bg-rose-50/60 border-rose-400 shadow-sm ring-1 ring-rose-400/30"
                    : "bg-slate-50 hover:bg-slate-100/80 border-slate-200"
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-lg bg-rose-100 text-rose-700 flex items-center justify-center">
                    <WifiOff className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-xs font-bold text-slate-900 font-mono">2. RPC Failure</h4>
                    <span className="text-[11px] text-rose-700 font-medium">Node Drop / 429</span>
                  </div>
                </div>
                <p className="text-xs text-slate-600 mt-2.5 leading-relaxed">
                  Gracefully catches transport dropped connections and RPC rate limits, offering 1-click retry and fallback endpoints.
                </p>
              </div>

              {/* 3. API Failure */}
              <div
                onClick={() => triggerTestError("api_failure")}
                className={`cursor-pointer p-4 rounded-xl border transition-all ${
                  activeErrorDemo === "api_failure"
                    ? "bg-rose-50/60 border-rose-400 shadow-sm ring-1 ring-rose-400/30"
                    : "bg-slate-50 hover:bg-slate-100/80 border-slate-200"
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-lg bg-rose-100 text-rose-700 flex items-center justify-center">
                    <AlertCircle className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-xs font-bold text-slate-900 font-mono">3. API Failure</h4>
                    <span className="text-[11px] text-rose-700 font-medium">Backend Degradation</span>
                  </div>
                </div>
                <p className="text-xs text-slate-600 mt-2.5 leading-relaxed">
                  Catches REST API disconnections via ApiClientError, switching to cached fixtures without locking the interface.
                </p>
              </div>

              {/* 4. Wallet Failure */}
              <div
                onClick={() => triggerTestError("wallet_failure")}
                className={`cursor-pointer p-4 rounded-xl border transition-all ${
                  activeErrorDemo === "wallet_failure"
                    ? "bg-orange-50/60 border-orange-400 shadow-sm ring-1 ring-orange-400/30"
                    : "bg-slate-50 hover:bg-slate-100/80 border-slate-200"
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-lg bg-orange-100 text-orange-700 flex items-center justify-center">
                    <Wallet className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-xs font-bold text-slate-900 font-mono">4. Wallet Failure</h4>
                    <span className="text-[11px] text-orange-700 font-medium">User Rejection / Low SOL</span>
                  </div>
                </div>
                <p className="text-xs text-slate-600 mt-2.5 leading-relaxed">
                  Clear, human-readable notifications when a user rejects a signature or lacks minimum SOL rent for account creation.
                </p>
              </div>

              {/* 5. Simulation Failure */}
              <div
                onClick={() => triggerTestError("simulation_failure")}
                className={`cursor-pointer p-4 rounded-xl border transition-all ${
                  activeErrorDemo === "simulation_failure"
                    ? "bg-purple-50/60 border-purple-400 shadow-sm ring-1 ring-purple-400/30"
                    : "bg-slate-50 hover:bg-slate-100/80 border-slate-200"
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-lg bg-purple-100 text-purple-700 flex items-center justify-center">
                    <Cpu className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-xs font-bold text-slate-900 font-mono">5. Simulation Failure</h4>
                    <span className="text-[11px] text-purple-700 font-medium">Preflight Rejection</span>
                  </div>
                </div>
                <p className="text-xs text-slate-600 mt-2.5 leading-relaxed">
                  Simulates Anchor CPIs prior to broadcast. Rejects failed transactions safely, saving gas and explaining program errors.
                </p>
              </div>

              {/* 6. Policy Rejection */}
              <div
                onClick={() => triggerTestError("policy_rejection")}
                className={`cursor-pointer p-4 rounded-xl border transition-all ${
                  activeErrorDemo === "policy_rejection"
                    ? "bg-rose-50/60 border-rose-400 shadow-sm ring-1 ring-rose-400/30"
                    : "bg-slate-50 hover:bg-slate-100/80 border-slate-200"
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-lg bg-rose-100 text-rose-700 flex items-center justify-center">
                    <ShieldAlert className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-xs font-bold text-slate-900 font-mono">6. Policy Rejection</h4>
                    <span className="text-[11px] text-rose-700 font-medium">Deterministic Invariant</span>
                  </div>
                </div>
                <p className="text-xs text-slate-600 mt-2.5 leading-relaxed">
                  Enforces concentration limits, pause flags, and slippage caps. Explains exactly which rule blocked the trade.
                </p>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
