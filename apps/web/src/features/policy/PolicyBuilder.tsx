"use client";

import React, { useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { useWallet } from "@solana/wallet-adapter-react";
import {
  AlertTriangle,
  CheckCircle2,
  Power,
  RefreshCw,
  Shield,
  Sliders,
} from "lucide-react";
import { PolicyModel } from "@equity-catalyst/sdk";

import { useSolanaTx } from "../../hooks/useSolanaTx";
import { TransactionStatus } from "../../components/TransactionStatus";
import { getSdkClient } from "../../lib/sdk";
import { useAppDispatch } from "../../store/hooks";
import { savePolicy } from "../../store/policySlice";
import { Card, CardContent, CardHeader, CardTitle, CardFooter } from "../../components/ui/card";
import { Button } from "../../components/ui/button";
import { Badge } from "../../components/ui/badge";
import { Slider } from "../../components/ui/slider";
import { Input } from "../../components/ui/input";

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
  const dispatch = useAppDispatch();
  const { publicKey } = useWallet();
  const { status: txStatus, executeTx, reset: resetTx } = useSolanaTx();

  // Policy Form State (in percentages for intuitive UI)
  const [maxPositionPct, setMaxPositionPct] = useState<number>(
    initialPolicy ? initialPolicy.max_position_bps / 100 : 25
  );
  const [minCashPct, setMinCashPct] = useState<number>(
    initialPolicy ? initialPolicy.min_cash_bps / 100 : 10
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
  if (minCashPct < 5 || minCashPct > 100) {
    validationErrors.push("Minimum unencumbered cash reserve must be between 5% and 100%");
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
      setMinCashPct(25);
      setStopLossPct(5);
      setTakeProfitPct(15);
      setRebalanceThresholdPct(1.0);
    } else if (tier === "balanced") {
      setMaxPositionPct(25);
      setMinCashPct(15);
      setStopLossPct(8);
      setTakeProfitPct(25);
      setRebalanceThresholdPct(1.5);
    } else if (tier === "aggressive") {
      setMaxPositionPct(40);
      setMinCashPct(10);
      setStopLossPct(12);
      setTakeProfitPct(40);
      setRebalanceThresholdPct(2.5);
    }
  };

  const handleSavePolicy = async () => {
    if (!isValid) return;
    setApiFeedback(null);
    setIsSubmitting(true);

    const minCashBps = Math.round(minCashPct * 100);
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
          minCashBps,
          maxPositionBps,
          stopLossBps,
          takeProfitBps,
          rebalanceThresholdBps,
          isActive,
        });

        await executeTx(ix);
      }

      const updatedPolicy: PolicyModel = {
        policy_address: initialPolicy?.policy_address || `pol-${Date.now()}`,
        vault_address: vaultAddress,
        authority: publicKey ? publicKey.toBase58() : initialPolicy?.authority || "",
        min_cash_bps: minCashBps,
        max_position_bps: maxPositionBps,
        stop_loss_bps: stopLossBps,
        take_profit_bps: takeProfitBps,
        rebalance_threshold_bps: rebalanceThresholdBps,
        is_active: isActive,
        bump: initialPolicy?.bump || 255,
        created_at: initialPolicy?.created_at || new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      // 2. Dispatch savePolicy to Redux store
      if (syncToBackend) {
        await dispatch(savePolicy(updatedPolicy)).unwrap();
      }

      if (onPolicyUpdated) {
        onPolicyUpdated(updatedPolicy);
      }

      setApiFeedback({
        type: "success",
        message: "Policy successfully updated and enforced in Redux store & on-chain!",
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
    <Card className="bg-white">
      {/* Header & Risk Tier Selector */}
      <CardHeader className="p-6 border-b border-slate-100 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-emerald-50 flex items-center justify-center text-emerald-600 border border-emerald-100">
              <Sliders className="w-4 h-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <CardTitle className="text-base font-bold text-slate-900">
                  Autonomous Policy Engine Configuration
                </CardTitle>
                <Badge variant="success" className="text-[10px] font-semibold tracking-wide">
                  100% Spot Only
                </Badge>
              </div>
              <p className="text-xs text-slate-500">
                Non-leveraged Shariah-compliant guardrails enforced on-chain and synchronized via Redux.
              </p>
            </div>
          </div>
        </div>

        {/* Preset Buttons */}
        <div className="flex items-center gap-1.5 p-1 rounded-lg bg-slate-100 border border-slate-200">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => applyPreset("conservative")}
            className="text-xs font-semibold h-7 px-2.5 hover:bg-white text-slate-700"
          >
            Conservative
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => applyPreset("balanced")}
            className="text-xs font-semibold h-7 px-2.5 hover:bg-white text-slate-700"
          >
            Balanced
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => applyPreset("aggressive")}
            className="text-xs font-semibold h-7 px-2.5 hover:bg-white text-slate-700"
          >
            Aggressive
          </Button>
        </div>
      </CardHeader>

      <CardContent className="p-6 space-y-6">
        {/* Main Parameters Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* 1. Max Position Exposure */}
          <div className="space-y-3 p-4 rounded-xl bg-slate-50 border border-slate-200">
            <div className="flex justify-between items-center text-xs">
              <span className="font-semibold text-slate-700">Max Single Position Exposure</span>
              <Badge variant="success" className="font-mono text-xs font-bold">
                {maxPositionPct}% ({maxPositionPct * 100} bps)
              </Badge>
            </div>
            <Slider
              min={5}
              max={60}
              step={1}
              value={[maxPositionPct]}
              onValueChange={(val) => setMaxPositionPct(val[0])}
            />
            <p className="text-[11px] text-slate-500">
              Caps maximum capital concentrated into any single token asset.
            </p>
          </div>

          {/* 2. Minimum Cash Reserve */}
          <div className="space-y-3 p-4 rounded-xl bg-slate-50 border border-slate-200">
            <div className="flex justify-between items-center text-xs">
              <span className="font-semibold text-slate-700">Min Cash Reserve Ratio</span>
              <Badge variant="cyan" className="font-mono text-xs font-bold">
                {minCashPct}% ({minCashPct * 100} bps)
              </Badge>
            </div>
            <Slider
              min={5}
              max={50}
              step={1}
              value={[minCashPct]}
              onValueChange={(val) => setMinCashPct(val[0])}
            />
            <p className="text-[11px] text-slate-500">
              Mandatory unencumbered liquid reserve (zero borrowing, 100% non-leveraged spot only).
            </p>
          </div>

          {/* 3. Stop Loss */}
          <div className="space-y-3 p-4 rounded-xl bg-slate-50 border border-slate-200">
            <div className="flex justify-between items-center text-xs">
              <span className="font-semibold text-slate-700">Stop Loss Limit</span>
              <Badge variant="rose" className="font-mono text-xs font-bold">
                -{stopLossPct}% ({stopLossPct * 100} bps)
              </Badge>
            </div>
            <Slider
              min={2}
              max={25}
              step={0.5}
              value={[stopLossPct]}
              onValueChange={(val) => setStopLossPct(val[0])}
            />
            <p className="text-[11px] text-slate-500">
              Automatic liquidation trigger when oracle price declines.
            </p>
          </div>

          {/* 4. Take Profit */}
          <div className="space-y-3 p-4 rounded-xl bg-slate-50 border border-slate-200">
            <div className="flex justify-between items-center text-xs">
              <span className="font-semibold text-slate-700">Take Profit Target</span>
              <Badge variant="warning" className="font-mono text-xs font-bold">
                +{takeProfitPct}% ({takeProfitPct * 100} bps)
              </Badge>
            </div>
            <Slider
              min={10}
              max={60}
              step={1}
              value={[takeProfitPct]}
              onValueChange={(val) => setTakeProfitPct(val[0])}
            />
            <p className="text-[11px] text-slate-500">
              Trigger to realize gains and rebalance to stable collateral.
            </p>
          </div>
        </div>

        {/* Rebalance Threshold & Active Toggle */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 p-4 rounded-xl bg-slate-50 border border-slate-200">
          {/* Rebalance drift */}
          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-slate-700 flex justify-between">
              <span>Portfolio Drift Tolerance</span>
              <span className="font-mono text-slate-900 font-bold">{rebalanceThresholdPct}%</span>
            </label>
            <Input
              type="number"
              step="0.1"
              min="0.5"
              max="10"
              value={rebalanceThresholdPct}
              onChange={(e) => setRebalanceThresholdPct(Number(e.target.value))}
              className="bg-white"
            />
            <p className="text-[11px] text-slate-500">
              Weight deviation trigger for automated Jupiter swap rebalances.
            </p>
          </div>

          {/* Policy Active Switch */}
          <div className="flex items-center justify-between p-3 rounded-lg bg-white border border-slate-200">
            <div className="space-y-0.5">
              <span className="text-xs font-semibold text-slate-800 flex items-center gap-1.5">
                <Power className={`w-3.5 h-3.5 ${isActive ? "text-emerald-600" : "text-slate-400"}`} />
                Autonomous Policy Execution
              </span>
              <p className="text-[11px] text-slate-500">
                {isActive ? "Policy active and evaluating live triggers" : "Policy disabled / manual mode only"}
              </p>
            </div>

            <button
              type="button"
              onClick={() => setIsActive(!isActive)}
              className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                isActive ? "bg-emerald-600" : "bg-slate-300"
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
          <div className="p-3 rounded-lg bg-rose-50 border border-rose-200 text-rose-800 text-xs space-y-1">
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
                ? "bg-emerald-50 border border-emerald-200 text-emerald-800"
                : "bg-rose-50 border border-rose-200 text-rose-800"
            }`}
          >
            {apiFeedback.type === "success" ? (
              <CheckCircle2 className="w-4 h-4 shrink-0 text-emerald-600" />
            ) : (
              <AlertTriangle className="w-4 h-4 shrink-0 text-rose-600" />
            )}
            <span>{apiFeedback.message}</span>
          </div>
        )}
      </CardContent>

      {/* Footer Controls */}
      <CardFooter className="p-6 pt-3 border-t border-slate-100 flex flex-col sm:flex-row items-center justify-between gap-4">
        <label className="flex items-center gap-2 text-xs text-slate-600 cursor-pointer">
          <input
            type="checkbox"
            checked={syncToBackend}
            onChange={(e) => setSyncToBackend(e.target.checked)}
            className="rounded border-slate-300 text-emerald-600 focus:ring-emerald-500"
          />
          <span>Sync parameters with Redux store & policy worker</span>
        </label>

        <Button
          type="button"
          variant="emerald"
          disabled={!isValid || isSubmitting}
          onClick={handleSavePolicy}
          className="w-full sm:w-auto flex items-center justify-center gap-2"
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
        </Button>
      </CardFooter>

      {/* On-Chain Transaction Progress Modal */}
      <TransactionStatus status={txStatus} onClose={resetTx} />
    </Card>
  );
}
