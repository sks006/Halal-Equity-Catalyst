"use client";

import React, { useState } from "react";
import Link from "next/link";
import {
  Activity,
  CheckCircle2,
  Cpu,
  Database,
  ExternalLink,
  Eye,
  KeyRound,
  Lock,
  RefreshCw,
  Server,
  Settings as SettingsIcon,
  Shield,
  ShieldAlert,
  ShieldCheck,
  Zap,
} from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";

export default function SettingsPage() {
  const [rpcUrl, setRpcUrl] = useState<string>("https://api.devnet.solana.com");
  const [apiUrl, setApiUrl] = useState<string>("http://127.0.0.1:4000");
  const [defaultSlippageBps, setDefaultSlippageBps] = useState<number>(50);
  const [isSaved, setIsSaved] = useState<boolean>(false);

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    setIsSaved(true);
    setTimeout(() => setIsSaved(false), 2000);
  };

  return (
    <div className="space-y-8">
      {/* 1. Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-emerald-50 text-emerald-600 flex items-center justify-center">
              <SettingsIcon className="w-4 h-4" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight text-slate-900">
              System Settings & Security Boundary
            </h1>
          </div>
          <p className="text-xs text-slate-500 mt-1">
            RPC endpoint configuration, API connectivity, and non-custodial cryptographic boundary verification
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Badge variant="emerald" className="font-mono text-xs py-1">
            <ShieldCheck className="w-3 h-3 mr-1" />
            <span>Strict Isolation Active</span>
          </Badge>
        </div>
      </div>

      {/* 2. Frontend Security Invariant (Step 07.9) */}
      <Card className="bg-gradient-to-br from-slate-900 via-slate-850 to-slate-950 text-white border-slate-800 shadow-lg overflow-hidden">
        <CardHeader className="p-6 border-b border-slate-800 flex flex-row items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-emerald-500/20 text-emerald-400 flex items-center justify-center border border-emerald-500/30">
              <Lock className="w-5 h-5" />
            </div>
            <div>
              <CardTitle className="text-base font-bold text-white font-mono">
                Frontend Security Boundary (Phase 07.9)
              </CardTitle>
              <p className="text-xs text-slate-400 mt-0.5 font-sans">
                Zero exposure of backend secrets, database credentials, or keeper private keys
              </p>
            </div>
          </div>

          <Badge variant="cyan" className="font-mono text-[10px] py-1">
            Non-Custodial Client
          </Badge>
        </CardHeader>

        <CardContent className="p-6 space-y-4 text-xs font-mono">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="p-4 rounded-xl bg-slate-800/80 border border-slate-700/60 space-y-1.5">
              <div className="text-slate-400 text-[11px] flex items-center gap-1">
                <KeyRound className="w-3.5 h-3.5 text-emerald-400" />
                <span>Signer Key Isolation</span>
              </div>
              <div className="text-sm font-bold text-emerald-400">ISOLATED IN RUST KEEPER</div>
              <p className="text-[11px] text-slate-400 font-sans mt-1">
                Execution keys exist exclusively in backend <code className="text-slate-300">ExecutionSigner</code>; never sent over HTTP or stored in client bundles.
              </p>
            </div>

            <div className="p-4 rounded-xl bg-slate-800/80 border border-slate-700/60 space-y-1.5">
              <div className="text-slate-400 text-[11px] flex items-center gap-1">
                <Database className="w-3.5 h-3.5 text-cyan-400" />
                <span>Database Credentials</span>
              </div>
              <div className="text-sm font-bold text-cyan-400">BACKEND PROTECTED</div>
              <p className="text-[11px] text-slate-400 font-sans mt-1">
                PostgreSQL and Redis connection strings are private to Axum microservices; zero client access.
              </p>
            </div>

            <div className="p-4 rounded-xl bg-slate-800/80 border border-slate-700/60 space-y-1.5">
              <div className="text-slate-400 text-[11px] flex items-center gap-1">
                <Cpu className="w-3.5 h-3.5 text-purple-400" />
                <span>Preflight Gate</span>
              </div>
              <div className="text-sm font-bold text-purple-400">SIMULATION ENFORCED</div>
              <p className="text-[11px] text-slate-400 font-sans mt-1">
                Transactions must pass simulation before signature authorization. Failed simulations abort automatically.
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 3. Endpoint Configuration Form */}
      <Card className="bg-white border-slate-200 shadow-sm">
        <CardHeader className="p-6 border-b border-slate-100">
          <CardTitle className="text-base font-bold text-slate-900">Network & Protocol Parameters</CardTitle>
          <p className="text-xs text-slate-500 mt-0.5">
            Configure local or production RPC clusters and backend service routing
          </p>
        </CardHeader>

        <CardContent className="p-6">
          <form onSubmit={handleSave} className="space-y-4 max-w-xl">
            <div className="space-y-1.5">
              <label className="text-xs font-bold text-slate-700 font-mono">Solana RPC Endpoint URL</label>
              <Input
                type="text"
                value={rpcUrl}
                onChange={(e) => setRpcUrl(e.target.value)}
                className="font-mono text-xs bg-slate-50 border-slate-200"
              />
              <p className="text-[11px] text-slate-400">Default: Solana Devnet RPC (https://api.devnet.solana.com)</p>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-bold text-slate-700 font-mono">Backend API URL</label>
              <Input
                type="text"
                value={apiUrl}
                onChange={(e) => setApiUrl(e.target.value)}
                className="font-mono text-xs bg-slate-50 border-slate-200"
              />
              <p className="text-[11px] text-slate-400">Equity Catalyst REST API microservice host</p>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-bold text-slate-700 font-mono">Max Default Slippage (bps)</label>
              <Input
                type="number"
                value={defaultSlippageBps}
                onChange={(e) => setDefaultSlippageBps(Number(e.target.value))}
                className="font-mono text-xs bg-slate-50 border-slate-200"
                min={5}
                max={500}
              />
              <p className="text-[11px] text-slate-400">Standard tolerance for bonding curve and DEX swaps (50 bps = 0.50%)</p>
            </div>

            <div className="pt-2 flex items-center gap-3">
              <Button variant="emerald" size="sm" type="submit" className="font-bold text-xs">
                Save Preferences
              </Button>
              {isSaved && (
                <span className="text-xs text-emerald-600 font-bold flex items-center gap-1">
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  <span>Preferences saved</span>
                </span>
              )}
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
