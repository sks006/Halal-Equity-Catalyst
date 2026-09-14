"use client";

import React, { useMemo, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { PublicKey } from "@solana/web3.js";
import { useWallet } from "@solana/wallet-adapter-react";
import {
  ArrowLeft,
  CheckCircle2,
  HelpCircle,
  Key,
  Layers,
  PlusCircle,
  RefreshCw,
  ShieldAlert,
  ShieldCheck,
  Zap,
} from "lucide-react";
import { findPolicyPda, findVaultPda, VaultModel } from "@equity-catalyst/sdk";

import { useSolanaTx } from "../../../hooks/useSolanaTx";
import { TransactionStatus } from "../../../components/TransactionStatus";
import { getSdkClient } from "../../../lib/sdk";
import { useAppDispatch } from "../../../store/hooks";
import { addVault } from "../../../store/vaultsSlice";
import { Card, CardContent, CardHeader, CardTitle, CardFooter } from "../../../components/ui/card";
import { Button } from "../../../components/ui/button";
import { Input } from "../../../components/ui/input";
import { Slider } from "../../../components/ui/slider";
import { Badge } from "../../../components/ui/badge";

const MINT_PRESETS = [
  {
    name: "USDC (USD Coin)",
    symbol: "USDC",
    mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  },
  {
    name: "Wrapped SOL",
    symbol: "WSOL",
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

  const [name, setName] = useState<string>("Alpha Liquidity Vault");
  const [symbol, setSymbol] = useState<string>("ALV");
  const [selectedPreset, setSelectedPreset] = useState<string>(MINT_PRESETS[0].mint);
  const [customMint, setCustomMint] = useState<string>("");
  const [isCustomMint, setIsCustomMint] = useState<boolean>(false);

  const [maxLtvPct, setMaxLtvPct] = useState<number>(65);
  const [maxPositionPct, setMaxPositionPct] = useState<number>(25);
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [errorFeedback, setErrorFeedback] = useState<string | null>(null);

  const effectiveMint = isCustomMint ? customMint.trim() : selectedPreset;

  // Live PDA Derivation
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
    return maxLtvPct > 0 && maxLtvPct <= 90 && maxPositionPct > 0 && maxPositionPct <= 100;
  }, [connected, publicKey, name, symbol, effectiveMint, maxLtvPct, maxPositionPct]);

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
        maxLtvBps: Math.round(maxLtvPct * 100),
        maxPositionBps: Math.round(maxPositionPct * 100),
      });

      // 2. Execute on-chain transaction
      const signature = await executeTx(ix);

      // 3. Register with Redux store & optional API
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
      setErrorFeedback(err?.message || "Failed to initialize vault on Solana");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="max-w-3xl mx-auto space-y-6">
      {/* Navigation Header */}
      <div className="flex items-center gap-3">
        <Link href="/dashboard">
          <Button variant="outline" size="icon" className="h-9 w-9">
            <ArrowLeft className="w-4 h-4" />
          </Button>
        </Link>
        <div>
          <h1 className="text-xl font-bold text-slate-900 flex items-center gap-2">
            <PlusCircle className="w-5 h-5 text-emerald-600" />
            <span>Initialize Autonomous Strategy Vault</span>
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Deploy an Anchor PDA-governed vault equipped with automated risk defense and Redux state tracking.
          </p>
        </div>
      </div>

      {!connected && (
        <div className="p-4 rounded-xl border border-amber-200 bg-amber-50 flex items-center justify-between text-xs">
          <div className="flex items-center gap-2 text-amber-800 font-medium">
            <HelpCircle className="w-4 h-4 shrink-0" />
            <span>Connect your Solana wallet to initialize and become authority of this vault.</span>
          </div>
        </div>
      )}

      {/* Main Creation Form */}
      <form onSubmit={handleCreateVault} className="space-y-6">
        <Card className="bg-white">
          <CardHeader className="p-6 pb-3 border-b border-slate-100">
            <div className="flex items-center gap-2">
              <Layers className="w-4 h-4 text-emerald-600" />
              <CardTitle className="text-sm font-bold text-slate-900">
                Vault Identity & Asset Standard
              </CardTitle>
            </div>
          </CardHeader>

          <CardContent className="p-6 space-y-5">
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
              {/* Vault Name */}
              <div className="sm:col-span-2 space-y-1.5">
                <label className="text-xs font-semibold text-slate-700">Vault Name</label>
                <Input
                  type="text"
                  required
                  maxLength={32}
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g. Solana Liquid Growth"
                />
                <p className="text-[10px] text-slate-400">Seed string for PDA derivation (max 32 chars).</p>
              </div>

              {/* Vault Symbol */}
              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700">Share Symbol</label>
                <Input
                  type="text"
                  required
                  maxLength={8}
                  value={symbol}
                  onChange={(e) => setSymbol(e.target.value.toUpperCase())}
                  placeholder="e.g. SLG"
                  className="font-mono uppercase"
                />
                <p className="text-[10px] text-slate-400">Ticker for share tokens.</p>
              </div>
            </div>

            {/* Collateral Asset Mint */}
            <div className="space-y-3 pt-2">
              <label className="text-xs font-semibold text-slate-700">Collateral Deposit Mint</label>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
                {MINT_PRESETS.map((preset) => {
                  const isSelected = !isCustomMint && selectedPreset === preset.mint;
                  return (
                    <button
                      key={preset.mint}
                      type="button"
                      onClick={() => {
                        setIsCustomMint(false);
                        setSelectedPreset(preset.mint);
                      }}
                      className={`p-3 rounded-lg border text-left transition-all ${
                        isSelected
                          ? "bg-emerald-50 border-emerald-500 text-emerald-900 shadow-sm"
                          : "bg-white border-slate-200 text-slate-600 hover:border-slate-300"
                      }`}
                    >
                      <div className="font-bold text-xs">{preset.symbol}</div>
                      <div className="text-[10px] text-slate-400 mt-0.5 truncate">{preset.name}</div>
                    </button>
                  );
                })}
              </div>

              {/* Custom Mint Toggle */}
              <div className="pt-2">
                <label className="flex items-center gap-2 text-xs text-slate-600 cursor-pointer mb-2">
                  <input
                    type="checkbox"
                    checked={isCustomMint}
                    onChange={(e) => setIsCustomMint(e.target.checked)}
                    className="rounded border-slate-300 text-emerald-600 focus:ring-emerald-500"
                  />
                  <span>Use custom SPL token mint address</span>
                </label>

                {isCustomMint && (
                  <Input
                    type="text"
                    placeholder="Enter 32-byte Base58 token mint public key..."
                    value={customMint}
                    onChange={(e) => setCustomMint(e.target.value)}
                    className="font-mono text-xs"
                  />
                )}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Initial Guardrails */}
        <Card className="bg-white">
          <CardHeader className="p-6 pb-3 border-b border-slate-100">
            <div className="flex items-center gap-2">
              <ShieldCheck className="w-4 h-4 text-cyan-600" />
              <CardTitle className="text-sm font-bold text-slate-900">
                Initial Risk Defense Ceilings
              </CardTitle>
            </div>
          </CardHeader>

          <CardContent className="p-6 space-y-6">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
              <div className="space-y-3">
                <div className="flex justify-between items-center text-xs">
                  <span className="font-medium text-slate-700">Max Loan-to-Value (LTV)</span>
                  <Badge variant="cyan" className="font-mono font-bold">{maxLtvPct}%</Badge>
                </div>
                <Slider
                  min={10}
                  max={85}
                  step={5}
                  value={[maxLtvPct]}
                  onValueChange={(val) => setMaxLtvPct(val[0])}
                />
                <p className="text-[11px] text-slate-400">Maximum allowable borrowing against collateral.</p>
              </div>

              <div className="space-y-3">
                <div className="flex justify-between items-center text-xs">
                  <span className="font-medium text-slate-700">Max Position Concentration</span>
                  <Badge variant="success" className="font-mono font-bold">{maxPositionPct}%</Badge>
                </div>
                <Slider
                  min={5}
                  max={50}
                  step={5}
                  value={[maxPositionPct]}
                  onValueChange={(val) => setMaxPositionPct(val[0])}
                />
                <p className="text-[11px] text-slate-400">Maximum allocation allowed in any single asset.</p>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Live PDA Preview Card */}
        <Card className="bg-slate-50 border-slate-200">
          <CardContent className="p-5 space-y-3">
            <div className="flex items-center justify-between text-xs">
              <div className="flex items-center gap-2 text-slate-800 font-bold">
                <Key className="w-4 h-4 text-purple-600" />
                <span>Real-Time Program Derived Addresses (PDAs)</span>
              </div>
              <span className="text-[10px] font-mono text-slate-400">Deterministic On-Chain Seeds</span>
            </div>

            {derivedPdas ? (
              <div className="space-y-2 font-mono text-xs">
                <div className="p-3 rounded-lg bg-white border border-slate-200 flex flex-col sm:flex-row sm:items-center justify-between gap-2 shadow-sm">
                  <div>
                    <span className="text-[10px] uppercase tracking-wider text-slate-400 block font-semibold">
                      Vault PDA (`[b"vault", authority, name]`)
                    </span>
                    <span className="text-emerald-700 font-bold break-all">{derivedPdas.vaultPda}</span>
                  </div>
                  <Badge variant="outline" className="text-[10px] shrink-0 self-start sm:self-center font-mono">
                    Bump: {derivedPdas.vaultBump}
                  </Badge>
                </div>

                <div className="p-3 rounded-lg bg-white border border-slate-200 flex flex-col sm:flex-row sm:items-center justify-between gap-2 shadow-sm">
                  <div>
                    <span className="text-[10px] uppercase tracking-wider text-slate-400 block font-semibold">
                      Policy PDA (`[b"policy", vault]`)
                    </span>
                    <span className="text-cyan-700 font-bold break-all">{derivedPdas.policyPda}</span>
                  </div>
                  <Badge variant="outline" className="text-[10px] shrink-0 self-start sm:self-center font-mono">
                    Bump: {derivedPdas.policyBump}
                  </Badge>
                </div>
              </div>
            ) : (
              <div className="p-4 rounded-lg bg-white border border-slate-200 text-center text-xs text-slate-400">
                Connect wallet and specify vault name to preview derived PDAs.
              </div>
            )}
          </CardContent>
        </Card>

        {/* Error Feedback */}
        {errorFeedback && (
          <div className="p-3 rounded-lg bg-rose-50 border border-rose-200 text-rose-800 text-xs flex items-center gap-2">
            <ShieldAlert className="w-4 h-4 shrink-0" />
            <span>{errorFeedback}</span>
          </div>
        )}

        {/* Submit Actions */}
        <div className="flex items-center justify-end gap-3 pt-2">
          <Link href="/dashboard">
            <Button variant="outline" size="sm">
              Cancel
            </Button>
          </Link>
          <Button
            type="submit"
            variant="emerald"
            disabled={!isValid || isSubmitting}
            className="flex items-center gap-2 font-bold"
          >
            {isSubmitting ? (
              <>
                <RefreshCw className="w-4 h-4 animate-spin" />
                <span>Broadcasting to Solana...</span>
              </>
            ) : (
              <>
                <Zap className="w-4 h-4 fill-white" />
                <span>Initialize Vault on Solana</span>
              </>
            )}
          </Button>
        </div>
      </form>

      {/* Transaction Status Modal */}
      <TransactionStatus status={txStatus} onClose={resetTx} />
    </div>
  );
}
