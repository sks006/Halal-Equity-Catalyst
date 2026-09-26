"use client";

import React, { useMemo, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { PublicKey } from "@solana/web3.js";
import { useWallet } from "@solana/wallet-adapter-react";
import {
  ArrowLeft,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  HelpCircle,
  PlusCircle,
  RefreshCw,
  ShieldAlert,
  ShieldCheck,
} from "lucide-react";
import { findPolicyPda, findVaultPda, VaultModel } from "@equity-catalyst/sdk";

import { useSolanaTx } from "../../../hooks/useSolanaTx";
import { TransactionStatus } from "../../../components/TransactionStatus";
import { getSdkClient } from "../../../lib/sdk";
import { useAppDispatch } from "../../../store/hooks";
import { addVault } from "../../../store/vaultsSlice";
import { Card, CardContent, CardHeader, CardTitle } from "../../../components/ui/card";
import { Button } from "../../../components/ui/button";
import { Input } from "../../../components/ui/input";
import { Slider } from "../../../components/ui/slider";

const DEPOSIT_ASSET_PRESETS = [
  {
    name: "USDC (USD Coin)",
    symbol: "USDC",
    mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  },
  {
    name: "Wrapped SOL",
    symbol: "SOL",
    mint: "So11111111111111111111111111111111111111112",
  },
  {
    name: "USDT (Tether USD)",
    symbol: "USDT",
    mint: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
  },
];

export default function NewVaultPage() {
  const router = useRouter();
  const dispatch = useAppDispatch();
  const { publicKey, connected } = useWallet();
  const { status: txStatus, executeTx, reset: resetTx } = useSolanaTx();

  // Step 1: Vault Name & Symbol
  const [name, setName] = useState<string>("My Portfolio Vault");
  const [symbol, setSymbol] = useState<string>("MPV");

  // Step 2: Deposit Asset
  const [selectedPresetMint, setSelectedPresetMint] = useState<string>(DEPOSIT_ASSET_PRESETS[0].mint);
  const [isCustomMint, setIsCustomMint] = useState<boolean>(false);
  const [customMint, setCustomMint] = useState<string>("");

  // Step 3: Basic Risk Preferences (Plain Language)
  const [minCashPct, setMinCashPct] = useState<number>(10);
  const [maxPositionPct, setMaxPositionPct] = useState<number>(25);

  // Advanced section collapsible
  const [showAdvancedDetails, setShowAdvancedDetails] = useState<boolean>(false);

  // Submitting state
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [errorFeedback, setErrorFeedback] = useState<string | null>(null);

  const effectiveMint = isCustomMint ? customMint.trim() : selectedPresetMint;

  // Selected preset object for display
  const selectedPreset = DEPOSIT_ASSET_PRESETS.find((p) => p.mint === effectiveMint) || {
    name: "Custom Asset",
    symbol: "TOKEN",
    mint: effectiveMint,
  };

  // Live PDA Derivation (preserves underlying cryptographic and transaction logic)
  const derivedPdas = useMemo(() => {
    if (!publicKey || !name.trim()) return null;
    try {
      const [vaultPda, vaultBump] = findVaultPda(publicKey, name.trim());
      const [policyPda, policyBump] = findPolicyPda(vaultPda);
      return {
        vaultPda: vaultPda.toBase58(),
        vaultBump,
        policyPda: policyPda.toBase58(),
        policyBump,
      };
    } catch {
      return null;
    }
  }, [publicKey, name]);

  const isValid = useMemo(() => {
    if (!connected || !publicKey) return false;
    if (!name.trim() || name.length > 32) return false;
    if (!symbol.trim() || symbol.length > 8) return false;
    try {
      new PublicKey(effectiveMint);
    } catch {
      return false;
    }
    return minCashPct >= 5 && minCashPct <= 50 && maxPositionPct >= 5 && maxPositionPct <= 50;
  }, [connected, publicKey, name, symbol, effectiveMint, minCashPct, maxPositionPct]);

  const handleCreateVault = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!isValid || !publicKey || !derivedPdas) return;

    setErrorFeedback(null);
    setIsSubmitting(true);

    try {
      const sdk = getSdkClient();
      const assetMintPubkey = new PublicKey(effectiveMint);

      // 1. Build initialize_vault instruction
      const ix = sdk.vaults.buildInitializeVaultIx({
        authority: publicKey,
        name: name.trim(),
        symbol: symbol.trim().toUpperCase(),
        assetMint: assetMintPubkey,
        minCashBps: Math.round(minCashPct * 100),
        maxPositionBps: Math.round(maxPositionPct * 100),
      });

      // 2. Execute on-chain transaction
      await executeTx(ix);

      // 3. Register with Redux store & API
      const newVaultModel: VaultModel = {
        vault_address: derivedPdas.vaultPda,
        authority: publicKey.toBase58(),
        name: name.trim(),
        symbol: symbol.trim().toUpperCase(),
        deposit_mint: effectiveMint,
        vault_token_account: "",
        total_shares: 0,
        total_deposits: 0,
        is_paused: false,
        bump: derivedPdas.vaultBump,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      dispatch(addVault(newVaultModel));

      if (sdk.apiUrl) {
        try {
          await fetch(`${sdk.apiUrl}/vaults`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              vault_address: derivedPdas.vaultPda,
              authority: publicKey.toBase58(),
              name: name.trim(),
              symbol: symbol.trim().toUpperCase(),
              deposit_mint: effectiveMint,
            }),
          });
        } catch (apiErr) {
          console.warn("Backend API registration skipped or failed:", apiErr);
        }
      }

      // Success! Redirect to newly created vault page
      setTimeout(() => {
        router.push(`/vault/${derivedPdas.vaultPda}`);
      }, 1500);
    } catch (err: any) {
      setErrorFeedback(err?.message || "Failed to create vault. Please verify your connection.");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Link href="/dashboard">
          <Button variant="outline" size="icon" className="h-9 w-9 rounded-lg">
            <ArrowLeft className="w-4 h-4" />
          </Button>
        </Link>
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">
            Create Vault
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Set up a portfolio vault with automated risk protection and target asset allocations.
          </p>
        </div>
      </div>

      {!connected && (
        <div className="p-4 rounded-xl border border-amber-200 bg-amber-50 flex items-center justify-between text-xs">
          <div className="flex items-center gap-2 text-amber-800 font-medium">
            <HelpCircle className="w-4 h-4 shrink-0" />
            <span>Connect your wallet to become the owner of this vault.</span>
          </div>
        </div>
      )}

      <form onSubmit={handleCreateVault} className="space-y-6">
        {/* STEP 1: VAULT NAME */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardHeader className="px-6 py-4 border-b border-slate-100">
            <CardTitle className="text-sm font-bold text-slate-900 flex items-center gap-2">
              <span className="w-5 h-5 rounded-full bg-emerald-100 text-emerald-800 flex items-center justify-center text-xs font-bold">1</span>
              <span>Vault Name</span>
            </CardTitle>
          </CardHeader>

          <CardContent className="p-6 space-y-4">
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700">What do you want to call this vault?</label>
              <Input
                type="text"
                required
                maxLength={32}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. My Growth Vault"
                className="h-10 text-sm bg-slate-50 border-slate-200 focus:bg-white"
              />
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700">Short symbol</label>
              <Input
                type="text"
                required
                maxLength={8}
                value={symbol}
                onChange={(e) => setSymbol(e.target.value.toUpperCase())}
                placeholder="e.g. MGV"
                className="h-10 font-mono uppercase text-sm bg-slate-50 border-slate-200 focus:bg-white max-w-xs"
              />
              <p className="text-[11px] text-slate-400">Short identifier for your vault shares.</p>
            </div>
          </CardContent>
        </Card>

        {/* STEP 2: DEPOSIT ASSET */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardHeader className="px-6 py-4 border-b border-slate-100">
            <CardTitle className="text-sm font-bold text-slate-900 flex items-center gap-2">
              <span className="w-5 h-5 rounded-full bg-emerald-100 text-emerald-800 flex items-center justify-center text-xs font-bold">2</span>
              <span>Deposit Asset</span>
            </CardTitle>
          </CardHeader>

          <CardContent className="p-6 space-y-4">
            <label className="text-xs font-semibold text-slate-700 block">Choose the base asset accepted for deposits:</label>

            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
              {DEPOSIT_ASSET_PRESETS.map((preset) => {
                const isSelected = !isCustomMint && selectedPresetMint === preset.mint;
                return (
                  <button
                    key={preset.mint}
                    type="button"
                    onClick={() => {
                      setIsCustomMint(false);
                      setSelectedPresetMint(preset.mint);
                    }}
                    className={`p-3.5 rounded-xl border text-left transition-all ${
                      isSelected
                        ? "bg-emerald-50 border-emerald-500 text-emerald-950 shadow-sm"
                        : "bg-white border-slate-200 text-slate-700 hover:border-slate-300"
                    }`}
                  >
                    <div className="font-bold text-sm">{preset.symbol}</div>
                    <div className="text-xs text-slate-500 mt-0.5">{preset.name}</div>
                  </button>
                );
              })}
            </div>

            {/* Custom token option */}
            <div className="pt-2">
              <label className="flex items-center gap-2 text-xs text-slate-600 cursor-pointer">
                <input
                  type="checkbox"
                  checked={isCustomMint}
                  onChange={(e) => setIsCustomMint(e.target.checked)}
                  className="rounded border-slate-300 text-emerald-600 focus:ring-emerald-500"
                />
                <span>Use custom token address</span>
              </label>

              {isCustomMint && (
                <div className="mt-2">
                  <Input
                    type="text"
                    placeholder="Enter token mint address..."
                    value={customMint}
                    onChange={(e) => setCustomMint(e.target.value)}
                    className="font-mono text-xs bg-slate-50 border-slate-200"
                  />
                </div>
              )}
            </div>
          </CardContent>
        </Card>

        {/* STEP 3: BASIC RISK PREFERENCES */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardHeader className="px-6 py-4 border-b border-slate-100">
            <CardTitle className="text-sm font-bold text-slate-900 flex items-center gap-2">
              <span className="w-5 h-5 rounded-full bg-emerald-100 text-emerald-800 flex items-center justify-center text-xs font-bold">3</span>
              <span>Basic Risk Preferences</span>
            </CardTitle>
          </CardHeader>

          <CardContent className="p-6 space-y-6">
            {/* Control 1: Minimum cash reserve */}
            <div className="space-y-2">
              <div className="flex justify-between items-center text-xs">
                <span className="font-semibold text-slate-800">Minimum cash reserve</span>
                <span className="font-bold text-emerald-800 bg-emerald-50 px-2 py-0.5 rounded border border-emerald-200">
                  {minCashPct}%
                </span>
              </div>
              <Slider
                min={5}
                max={50}
                step={1}
                value={[minCashPct]}
                onValueChange={(val) => setMinCashPct(val[0])}
              />
              <p className="text-xs text-slate-500">
                Cash held uninvested to protect capital and provide instant withdrawals.
              </p>
            </div>

            {/* Control 2: Maximum allocation to one asset */}
            <div className="space-y-2">
              <div className="flex justify-between items-center text-xs">
                <span className="font-semibold text-slate-800">Maximum allocation to one asset</span>
                <span className="font-bold text-emerald-800 bg-emerald-50 px-2 py-0.5 rounded border border-emerald-200">
                  {maxPositionPct}%
                </span>
              </div>
              <Slider
                min={5}
                max={50}
                step={5}
                value={[maxPositionPct]}
                onValueChange={(val) => setMaxPositionPct(val[0])}
              />
              <p className="text-xs text-slate-500">
                The largest percentage of your portfolio allowed in any single asset.
              </p>
            </div>
          </CardContent>
        </Card>

        {/* STEP 4: REVIEW */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardHeader className="px-6 py-4 border-b border-slate-100">
            <CardTitle className="text-sm font-bold text-slate-900 flex items-center gap-2">
              <span className="w-5 h-5 rounded-full bg-emerald-100 text-emerald-800 flex items-center justify-center text-xs font-bold">4</span>
              <span>Review</span>
            </CardTitle>
          </CardHeader>

          <CardContent className="p-6 space-y-4">
            <div className="p-4 rounded-xl bg-slate-50 border border-slate-200 divide-y divide-slate-200/80 text-xs">
              <div className="flex justify-between py-2">
                <span className="text-slate-500">Vault name</span>
                <span className="font-bold text-slate-900">{name} ({symbol})</span>
              </div>

              <div className="flex justify-between py-2">
                <span className="text-slate-500">Deposit asset</span>
                <span className="font-bold text-slate-900">{selectedPreset.name}</span>
              </div>

              <div className="flex justify-between py-2">
                <span className="text-slate-500">Cash reserve</span>
                <span className="font-bold text-slate-900">{minCashPct}%</span>
              </div>

              <div className="flex justify-between py-2">
                <span className="text-slate-500">Maximum asset allocation</span>
                <span className="font-bold text-slate-900">{maxPositionPct}%</span>
              </div>
            </div>

            {/* Error Feedback */}
            {errorFeedback && (
              <div className="p-3 rounded-lg bg-rose-50 border border-rose-200 text-rose-800 text-xs flex items-center gap-2">
                <ShieldAlert className="w-4 h-4 shrink-0" />
                <span>{errorFeedback}</span>
              </div>
            )}

            {/* Primary Action Button: Create Vault */}
            <Button
              type="submit"
              disabled={!isValid || isSubmitting}
              className="w-full h-11 text-sm font-semibold bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg shadow-sm"
            >
              {isSubmitting ? (
                <>
                  <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                  <span>Creating Vault...</span>
                </>
              ) : (
                <span>Create Vault</span>
              )}
            </Button>
          </CardContent>
        </Card>

        {/* ADVANCED TECHNICAL DETAILS (Collapsible, closed by default) */}
        <Card className="bg-slate-50 border-slate-200 rounded-xl overflow-hidden">
          <div className="px-6 py-3.5 border-b border-slate-200">
            <button
              type="button"
              onClick={() => setShowAdvancedDetails(!showAdvancedDetails)}
              className="flex items-center justify-between w-full text-xs font-semibold text-slate-600 hover:text-slate-900 transition-colors"
            >
              <span>Advanced technical details</span>
              {showAdvancedDetails ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
            </button>
          </div>

          {showAdvancedDetails && (
            <CardContent className="p-6 space-y-3 text-xs">
              {derivedPdas ? (
                <div className="space-y-2">
                  <div className="p-3 rounded-lg bg-white border border-slate-200">
                    <span className="text-xs text-slate-500 block">Vault PDA Address:</span>
                    <span className="text-slate-900 font-mono font-bold break-all">{derivedPdas.vaultPda}</span>
                  </div>

                  <div className="p-3 rounded-lg bg-white border border-slate-200">
                    <span className="text-xs text-slate-500 block">Policy PDA Address:</span>
                    <span className="text-slate-900 font-mono font-bold break-all">{derivedPdas.policyPda}</span>
                  </div>

                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div className="p-2.5 rounded bg-white border border-slate-200">
                      <span className="text-slate-500 block text-xs">Vault Bump:</span>
                      <span className="text-slate-800 font-mono">{derivedPdas.vaultBump}</span>
                    </div>

                    <div className="p-2.5 rounded bg-white border border-slate-200">
                      <span className="text-slate-500 block text-xs">Policy Bump:</span>
                      <span className="text-slate-800 font-mono">{derivedPdas.policyBump}</span>
                    </div>
                  </div>

                  <div className="p-3 rounded-lg bg-white border border-slate-200">
                    <span className="text-xs text-slate-500 block">Seed Derivation Details:</span>
                    <span className="text-slate-700 text-xs">
                      Vault: <code>[b"vault", authority, name]</code> &bull; Policy: <code>[b"policy", vault]</code>
                    </span>
                  </div>

                  <div className="p-3 rounded-lg bg-white border border-slate-200">
                    <span className="text-xs text-slate-500 block">Deposit Mint Public Key:</span>
                    <span className="text-slate-700 font-mono break-all text-xs">{effectiveMint}</span>
                  </div>
                </div>
              ) : (
                <div className="p-4 text-center text-slate-400">
                  Connect wallet to compute derived account parameters.
                </div>
              )}
            </CardContent>
          )}
        </Card>
      </form>

      {/* Transaction Status Modal */}
      <TransactionStatus status={txStatus} onClose={resetTx} />
    </div>
  );
}
