"use client";

import React, { useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { useWallet } from "@solana/wallet-adapter-react";
import {
  AlertTriangle,
  Check,
  CheckCircle2,
  Lock,
  Power,
  RefreshCw,
  Save,
  Send,
  Shield,
  Sliders,
  Sparkles,
} from "lucide-react";
import { PolicyModel } from "@equity-catalyst/sdk";

import { useSolanaTx } from "../../hooks/useSolanaTx";
import { TransactionStatus } from "../../components/TransactionStatus";
import { getSdkClient } from "../../lib/sdk";

interface Props {
  vaultAddress: string;
  initialPolicy?: PolicyModel | null;
  onPolicyUpdated?: (newPolicy: PolicyModel) => void;
}

export function PolicyBuilder({
  vaultAddress,
  initialPolicy,
  onPolicyUpdated,
}: Props) {
  const { publicKey } = useWallet();
  const { status: txStatus, executeTx, reset: resetTx } = useSolanaTx();

  // Policy Form State (in percentages for intuitive UI)
  const [maxPositionPct, setMaxPositionPct] = useState<number>(
    initialPolicy ? initialPolicy.max_position_bps / 100 : 25
  );
  const [maxLtvPct, setMaxLtvPct] = useState<number>(
    initialPolicy ? initialPolicy.max_ltv_bps / 100 : 65
  );
  const [stopLossPct, setStopLossPct] = useState<number>(
    initialPolicy ? initialPolicy.stop_loss_bps / 100 : 8
  );
  const [takeProfitPct, setTakeProfitPct] = useState<number>(
    initialPolicy ? initialPolicy.take_profit_bps / 100 : 20
  );
  const [rebalanceThresholdPct, setRebalanceThresholdPct] = useState<number>(
    initialPolicy ? initialPolicy.rebalance_threshold_bps / 100 : 1.5
  );
  const [isActive, setIsActive] = useState<boolean>(
    initialPolicy ? initialPolicy.is_active : true
  );

  const [syncToBackend, setSyncToBackend] = useState<boolean>(true);
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [apiFeedback, setApiFeedback] = useState<{
    type: "success" | "error";
    message: string;
  } | null>(null);

  // Derived validation rules
  const validationErrors: string[] = [];
  if (maxPositionPct <= 0 || maxPositionPct > 100) {
    validationErrors.push("Max position exposure must be between 1% and 100%");
  }
  if (maxLtvPct <= 0 || maxLtvPct > 90) {
    validationErrors.push("Max LTV must be between 1% and 90%");
  }
  if (stopLossPct <= 0 || stopLossPct > 50) {
    validationErrors.push("Stop loss must be between 0.1% and 50%");
  }
  if (takeProfitPct <= stopLossPct) {
    validationErrors.push("Take profit must be greater than stop loss percentage");
  }

  const isValid = validationErrors.length === 0;

  // Preset Risk Profiles
  const applyPreset = (tier: "conservative" | "balanced" | "aggressive") => {
    if (tier === "conservative") {
      setMaxPositionPct(15);
      setMaxLtvPct(45);
      setStopLossPct(5);
      setTakeProfitPct(15);
      setRebalanceThresholdPct(1.0);
    } else if (tier === "balanced") {
      setMaxPositionPct(25);
      setMaxLtvPct(65);
      setStopLossPct(8);
      setTakeProfitPct(25);
      setRebalanceThresholdPct(1.5);
    } else if (tier === "aggressive") {
      setMaxPositionPct(40);
      setMaxLtvPct(80);
      setStopLossPct(12);
      setTakeProfitPct(40);
      setRebalanceThresholdPct(2.5);
    }
  };

  const handleSavePolicy = async () => {
    if (!isValid) return;
    setApiFeedback(null);
    setIsSubmitting(true);

    const maxLtvBps = Math.round(maxLtvPct * 100);
    const maxPositionBps = Math.round(maxPositionPct * 100);
    const stopLossBps = Math.round(stopLossPct * 100);
    const takeProfitBps = Math.round(takeProfitPct * 100);
    const rebalanceThresholdBps = Math.round(rebalanceThresholdPct * 100);

    try {
      const sdk = getSdkClient();

      // 1. If wallet connected, submit on-chain Anchor update_policy instruction
      if (publicKey) {
        const vaultPubkey = new PublicKey(vaultAddress);
        const ix = sdk.policies.buildUpdatePolicyIx({
          vault: vaultPubkey,
          authority: publicKey,
          maxLtvBps,
          maxPositionBps,
          stopLossBps,
          takeProfitBps,
          rebalanceThresholdBps,
          isActive,
        });

        await executeTx(ix);
      }

      // 2. Optionally sync with backend REST API
      if (syncToBackend && sdk.apiUrl) {
        try {
          const resp = await fetch(`${sdk.apiUrl}/vaults/${vaultAddress}/policy`, {
            method: "PUT",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              max_ltv_bps: maxLtvBps,
              max_position_bps: maxPositionBps,
              stop_loss_bps: stopLossBps,
              take_profit_bps: takeProfitBps,
              rebalance_threshold_bps: rebalanceThresholdBps,
              is_active: isActive,
            }),
          });
          if (!resp.ok) {
            console.warn("API policy sync returned non-200, continuing with local state");
          }
        } catch (apiErr) {
          console.warn("Could not reach API server, policy updated locally/on-chain:", apiErr);
        }
      }

      const updatedPolicy: PolicyModel = {
        policy_id: initialPolicy?.policy_id || `pol-${Date.now()}`,
        vault_address: vaultAddress,
        max_ltv_bps: maxLtvBps,
        max_position_bps: maxPositionBps,
        stop_loss_bps: stopLossBps,
        take_profit_bps: takeProfitBps,
        rebalance_threshold_bps: rebalanceThresholdBps,
        is_active: isActive,
        updated_at: new Date().toISOString(),
      };

      if (onPolicyUpdated) {
        onPolicyUpdated(updatedPolicy);
      }

      setApiFeedback({
        type: "success",
        message: "Policy successfully updated and enforced!",
      });
    } catch (err: any) {
      setApiFeedback({
        type: "error",
        message: err?.message || "Failed to commit policy changes",
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="glass-panel rounded-xl p-6 border border-slate-800 space-y-6">
      {/* Header & Risk Tier Selector */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-5 border-b border-slate-800">
        <div>
          <div className="flex items-center gap-2">
            <Sliders className="w-5 h-5 text-emerald-400" />
            <h2 className="text-base font-bold text-slate-100">
              Autonomous Policy Engine Configuration
            </h2>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Programmatic guardrails executed by the risk defense engine and enforced on-chain.
          </p>
        </div>

        {/* Preset Buttons */}
        <div className="flex items-center gap-1.5 p-1 rounded-lg bg-slate-950/60 border border-slate-800">
          <button
            type="button"
            onClick={() => applyPreset("conservative")}
            className="px-2.5 py-1 text-[11px] font-medium rounded text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 transition-colors"
          >
            Conservative
          </button>
          <button
            type="button"
            onClick={() => applyPreset("balanced")}
            className="px-2.5 py-1 text-[11px] font-medium rounded text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 transition-colors"
          >
            Balanced
          </button>
          <button
            type="button"
            onClick={() => applyPreset("aggressive")}
            className="px-2.5 py-1 text-[11px] font-medium rounded text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 transition-colors"
          >
            Aggressive
          </button>
        </div>
      </div>

      {/* Main Parameters Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* 1. Max Position Exposure */}
        <div className="space-y-2 p-4 rounded-xl bg-slate-950/40 border border-slate-800/80">
          <div className="flex justify-between items-center text-xs">
            <span className="font-semibold text-slate-300">Max Single Position Exposure</span>
            <span className="font-mono font-bold text-emerald-400 bg-emerald-500/10 px-2 py-0.5 rounded border border-emerald-500/20">
              {maxPositionPct}% ({maxPositionPct * 100} bps)
            </span>
          </div>
          <input
            type="range"
            min="5"
            max="60"
            step="1"
            value={maxPositionPct}
            onChange={(e) => setMaxPositionPct(Number(e.target.value))}
            className="w-full accent-emerald-500 bg-slate-800 h-1.5 rounded cursor-pointer"
          />
          <p className="text-[11px] text-slate-500">
            Caps maximum capital concentrated into any single token asset (e.g., SOL, JUP).
          </p>
        </div>

        {/* 2. Max LTV */}
        <div className="space-y-2 p-4 rounded-xl bg-slate-950/40 border border-slate-800/80">
          <div className="flex justify-between items-center text-xs">
            <span className="font-semibold text-slate-300">Max Loan-to-Value (LTV)</span>
            <span className="font-mono font-bold text-cyan-400 bg-cyan-500/10 px-2 py-0.5 rounded border border-cyan-500/20">
              {maxLtvPct}% ({maxLtvPct * 100} bps)
            </span>
          </div>
          <input
            type="range"
            min="10"
            max="85"
            step="1"
            value={maxLtvPct}
            onChange={(e) => setMaxLtvPct(Number(e.target.value))}
            className="w-full accent-cyan-500 bg-slate-800 h-1.5 rounded cursor-pointer"
          />
          <p className="text-[11px] text-slate-500">
            Ceiling on flash-borrow and margin borrowing against vault collateral.
          </p>
        </div>

        {/* 3. Stop Loss */}
        <div className="space-y-2 p-4 rounded-xl bg-slate-950/40 border border-slate-800/80">
          <div className="flex justify-between items-center text-xs">
            <span className="font-semibold text-slate-300">Stop Loss Limit</span>
            <span className="font-mono font-bold text-rose-400 bg-rose-500/10 px-2 py-0.5 rounded border border-rose-500/20">
              -{stopLossPct}% ({stopLossPct * 100} bps)
            </span>
          </div>
          <input
            type="range"
            min="2"
            max="25"
            step="0.5"
            value={stopLossPct}
            onChange={(e) => setStopLossPct(Number(e.target.value))}
            className="w-full accent-rose-500 bg-slate-800 h-1.5 rounded cursor-pointer"
          />
          <p className="text-[11px] text-slate-500">
            Automatic position liquidation threshold trigger when oracle price declines.
          </p>
        </div>

        {/* 4. Take Profit */}
        <div className="space-y-2 p-4 rounded-xl bg-slate-950/40 border border-slate-800/80">
          <div className="flex justify-between items-center text-xs">
            <span className="font-semibold text-slate-300">Take Profit Target</span>
            <span className="font-mono font-bold text-amber-400 bg-amber-500/10 px-2 py-0.5 rounded border border-amber-500/20">
              +{takeProfitPct}% ({takeProfitPct * 100} bps)
            </span>
          </div>
          <input
            type="range"
            min="10"
            max="60"
            step="1"
            value={takeProfitPct}
            onChange={(e) => setTakeProfitPct(Number(e.target.value))}
            className="w-full accent-amber-500 bg-slate-800 h-1.5 rounded cursor-pointer"
          />
          <p className="text-[11px] text-slate-500">
            Partial or full rebalancing to stable collateral once gain threshold is realized.
          </p>
        </div>
      </div>

      {/* Rebalance Threshold & Active Toggle */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 p-4 rounded-xl bg-slate-950/30 border border-slate-800">
        {/* Rebalance drift */}
        <div className="space-y-1.5">
          <label className="text-xs font-semibold text-slate-300 flex justify-between">
            <span>Portfolio Drift Tolerance</span>
            <span className="font-mono text-slate-400 font-bold">{rebalanceThresholdPct}%</span>
          </label>
          <input
            type="number"
            step="0.1"
            min="0.5"
            max="10"
            value={rebalanceThresholdPct}
            onChange={(e) => setRebalanceThresholdPct(Number(e.target.value))}
            className="w-full px-3 py-2 rounded-lg bg-slate-900 border border-slate-700 text-xs font-mono text-slate-200 focus:outline-none focus:border-emerald-500"
          />
          <p className="text-[10px] text-slate-500">
            Weight deviation trigger for Jupiter swap rebalances.
          </p>
        </div>

        {/* Policy Active Switch */}
        <div className="flex items-center justify-between p-3 rounded-lg bg-slate-900/60 border border-slate-800">
          <div className="space-y-0.5">
            <span className="text-xs font-semibold text-slate-200 flex items-center gap-1.5">
              <Power className={`w-3.5 h-3.5 ${isActive ? "text-emerald-400" : "text-slate-500"}`} />
              Autonomous Policy Execution
            </span>
            <p className="text-[11px] text-slate-400">
              {isActive ? "Policy active and evaluating live triggers" : "Policy disabled / manual mode only"}
            </p>
          </div>

          <button
            type="button"
            onClick={() => setIsActive(!isActive)}
            className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
              isActive ? "bg-emerald-500" : "bg-slate-700"
            }`}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                isActive ? "translate-x-6" : "translate-x-1"
              }`}
            />
          </button>
        </div>
      </div>

      {/* Validation Alert */}
      {validationErrors.length > 0 && (
        <div className="p-3 rounded-lg bg-rose-500/10 border border-rose-500/30 text-rose-300 text-xs space-y-1">
          <div className="flex items-center gap-1.5 font-bold">
            <AlertTriangle className="w-3.5 h-3.5" />
            <span>Invalid Policy Configuration</span>
          </div>
          <ul className="list-disc list-inside space-y-0.5 pl-1 text-[11px]">
            {validationErrors.map((err, idx) => (
              <li key={idx}>{err}</li>
            ))}
          </ul>
        </div>
      )}

      {/* Feedback Alert */}
      {apiFeedback && (
        <div
          className={`p-3 rounded-lg text-xs flex items-center gap-2 ${
            apiFeedback.type === "success"
              ? "bg-emerald-500/10 border border-emerald-500/30 text-emerald-300"
              : "bg-rose-500/10 border border-rose-500/30 text-rose-300"
          }`}
        >
          {apiFeedback.type === "success" ? (
            <CheckCircle2 className="w-4 h-4 shrink-0" />
          ) : (
            <AlertTriangle className="w-4 h-4 shrink-0" />
          )}
          <span>{apiFeedback.message}</span>
        </div>
      )}

      {/* Footer Controls */}
      <div className="flex flex-col sm:flex-row items-center justify-between gap-4 pt-4 border-t border-slate-800">
        <label className="flex items-center gap-2 text-xs text-slate-400 cursor-pointer">
          <input
            type="checkbox"
            checked={syncToBackend}
            onChange={(e) => setSyncToBackend(e.target.checked)}
            className="rounded border-slate-700 text-emerald-500 focus:ring-0"
          />
          <span>Sync parameters with API backend & policy worker</span>
        </label>

        <div className="flex items-center gap-3 w-full sm:w-auto">
          <button
            type="button"
            disabled={!isValid || isSubmitting}
            onClick={handleSavePolicy}
            className="w-full sm:w-auto flex items-center justify-center gap-2 px-5 py-2.5 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-xs shadow-glow transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSubmitting ? (
              <>
                <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                <span>Broadcasting...</span>
              </>
            ) : (
              <>
                <Shield className="w-3.5 h-3.5" />
                <span>Enforce & Update Policy</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* On-Chain Transaction Progress Modal */}
      <TransactionStatus status={txStatus} onClose={resetTx} />
    </div>
  );
}
