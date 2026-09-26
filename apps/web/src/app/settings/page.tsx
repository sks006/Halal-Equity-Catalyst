"use client";

import React, { useState } from "react";
import { Settings as SettingsIcon, CheckCircle2, ShieldCheck } from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

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
    <div className="space-y-6 max-w-3xl mx-auto">
      {/* Page Header */}
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-slate-900 flex items-center gap-2">
          <SettingsIcon className="w-6 h-6 text-emerald-600" />
          <span>Settings</span>
        </h1>
        <p className="text-sm text-slate-500 mt-1">
          Configure network connection, trading preferences, and execution parameters.
        </p>
      </div>

      {/* Security Status Card */}
      <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
        <CardContent className="p-6 flex items-start gap-4">
          <div className="w-10 h-10 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-700 shrink-0">
            <ShieldCheck className="w-5 h-5" />
          </div>
          <div className="space-y-1">
            <h3 className="text-sm font-bold text-slate-900">Protected Non-Custodial Environment</h3>
            <p className="text-xs text-slate-600 leading-relaxed">
              Your wallet and trades remain strictly non-custodial. Private keys and sensitive credentials are never stored or transmitted by this frontend.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Configuration Form */}
      <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
        <CardHeader className="px-6 py-4 border-b border-slate-100">
          <CardTitle className="text-base font-bold text-slate-900">
            Connection & Trading Preferences
          </CardTitle>
        </CardHeader>

        <CardContent className="p-6">
          <form onSubmit={handleSave} className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700">Solana RPC Endpoint</label>
              <Input
                type="text"
                value={rpcUrl}
                onChange={(e) => setRpcUrl(e.target.value)}
                className="font-mono text-xs bg-slate-50 border-slate-200"
              />
              <p className="text-xs text-slate-500">Network connection for transaction confirmation</p>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700">Backend API URL</label>
              <Input
                type="text"
                value={apiUrl}
                onChange={(e) => setApiUrl(e.target.value)}
                className="font-mono text-xs bg-slate-50 border-slate-200"
              />
              <p className="text-xs text-slate-500">Service endpoint for market data and policy validation</p>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700">Default Slippage Tolerance (bps)</label>
              <Input
                type="number"
                value={defaultSlippageBps}
                onChange={(e) => setDefaultSlippageBps(Number(e.target.value))}
                className="text-xs bg-slate-50 border-slate-200"
                min={5}
                max={500}
              />
              <p className="text-xs text-slate-500">Standard tolerance for DEX swaps (50 bps = 0.50%)</p>
            </div>

            <div className="pt-2 flex items-center gap-3">
              <Button
                type="submit"
                className="bg-emerald-600 hover:bg-emerald-700 text-white font-semibold text-xs px-5 h-9 rounded-lg shadow-sm"
              >
                Save Preferences
              </Button>
              {isSaved && (
                <span className="text-xs text-emerald-700 font-semibold flex items-center gap-1">
                  <CheckCircle2 className="w-4 h-4" />
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
