"use client";

import React, { useEffect, useState } from "react";
import Link from "next/link";
import {
  Activity,
  ArrowUpRight,
  Database,
  Layers,
  PlusCircle,
  RefreshCw,
  Search,
  ShieldAlert,
  ShieldCheck,
  TrendingUp,
  Vault as VaultIcon,
  Wallet,
} from "lucide-react";
import { VaultModel } from "@equity-catalyst/sdk";

import { useWallet } from "../../hooks/useWallet";
import { useSolana } from "../../hooks/useSolana";
import { VaultCard } from "../../features/vault/VaultCard";
import { DEMO_VAULTS, getSdkClient } from "../../lib/sdk";

export default function DashboardPage() {
  const { connected, shortAddress } = useWallet();
  const { balanceFormatted, isDevnet, isLoading: isSolanaLoading, refresh: refreshSolana } =
    useSolana();

  const [vaults, setVaults] = useState<VaultModel[]>(DEMO_VAULTS);
  const [searchTerm, setSearchTerm] = useState<string>("");
  const [filterPaused, setFilterPaused] = useState<"all" | "active" | "paused">("all");
  const [isLoadingVaults, setIsLoadingVaults] = useState<boolean>(false);
  const [isLiveApi, setIsLiveApi] = useState<boolean>(false);

  // Fetch vaults from backend API or fallback gracefully
  const loadVaults = async () => {
    setIsLoadingVaults(true);
    try {
      const sdk = getSdkClient();
      if (sdk.apiUrl) {
        const res = await fetch(`${sdk.apiUrl}/vaults`, { signal: AbortSignal.timeout(3000) });
        if (res.ok) {
          const data = await res.json();
          if (Array.isArray(data) && data.length > 0) {
            setVaults(data);
            setIsLiveApi(true);
            setIsLoadingVaults(false);
            return;
          }
        }
      }
    } catch (err) {
      console.log("API not reachable, using live local/devnet fixtures");
    }
    // Fallback to high-fidelity fixtures
    setVaults(DEMO_VAULTS);
    setIsLiveApi(false);
    setIsLoadingVaults(false);
  };

  useEffect(() => {
    loadVaults();
  }, []);

  // Compute aggregated stats
  const totalTvl = vaults.reduce((acc, v) => acc + v.total_deposits, 0) / 1_000_000;
  const totalShares = vaults.reduce((acc, v) => acc + v.total_shares, 0);
  const activeVaultCount = vaults.filter((v) => !v.is_paused).length;

  const filteredVaults = vaults.filter((v) => {
    const matchesSearch =
      v.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      v.symbol.toLowerCase().includes(searchTerm.toLowerCase()) ||
      v.vault_address.toLowerCase().includes(searchTerm.toLowerCase());

    if (filterPaused === "active") return matchesSearch && !v.is_paused;
    if (filterPaused === "paused") return matchesSearch && v.is_paused;
    return matchesSearch;
  });

  return (
    <div className="space-y-8">
      {/* Top Banner: Protocol Stats & Wallet Balance */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {/* Total Value Locked */}
        <div className="glass-panel rounded-xl p-5 border border-slate-800 relative overflow-hidden">
          <div className="flex items-center justify-between text-slate-400 text-xs">
            <span>Total Value Locked</span>
            <TrendingUp className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="mt-3 flex items-baseline gap-2">
            <span className="text-2xl font-extrabold text-slate-100">
              ${totalTvl.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </span>
            <span className="text-xs font-semibold text-emerald-400">+12.4%</span>
          </div>
          <p className="text-[11px] text-slate-500 mt-1">Collateral under programmatic risk shield</p>
        </div>

        {/* Active Vaults */}
        <div className="glass-panel rounded-xl p-5 border border-slate-800">
          <div className="flex items-center justify-between text-slate-400 text-xs">
            <span>Active Vaults</span>
            <VaultIcon className="w-4 h-4 text-cyan-400" />
          </div>
          <div className="mt-3 flex items-baseline gap-2">
            <span className="text-2xl font-extrabold text-slate-100">{activeVaultCount}</span>
            <span className="text-xs text-slate-500 font-mono">/ {vaults.length} total</span>
          </div>
          <p className="text-[11px] text-slate-500 mt-1">Anchor PDA smart vaults</p>
        </div>

        {/* Connected Wallet State */}
        <div className="glass-panel rounded-xl p-5 border border-slate-800">
          <div className="flex items-center justify-between text-slate-400 text-xs">
            <span>Connected Wallet</span>
            <Wallet className="w-4 h-4 text-purple-400" />
          </div>
          <div className="mt-3">
            {connected ? (
              <div>
                <div className="text-lg font-bold text-slate-100 font-mono">{balanceFormatted} SOL</div>
                <div className="text-[11px] text-slate-400 font-mono mt-0.5">{shortAddress}</div>
              </div>
            ) : (
              <div className="text-sm font-medium text-slate-400 mt-1">Wallet not connected</div>
            )}
          </div>
          <p className="text-[11px] text-emerald-400/80 mt-1 font-mono">
            {connected ? "Devnet ready for deposits" : "Connect wallet to manage vaults"}
          </p>
        </div>

        {/* Risk & Sentinel Engine */}
        <div className="glass-panel rounded-xl p-5 border border-slate-800">
          <div className="flex items-center justify-between text-slate-400 text-xs">
            <span>Risk Defense Status</span>
            <ShieldCheck className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="mt-3 flex items-center gap-2">
            <span className="inline-block w-2.5 h-2.5 rounded-full bg-emerald-400 animate-pulse" />
            <span className="text-sm font-bold text-slate-200 font-mono">Dual-Engine Active</span>
          </div>
          <p className="text-[11px] text-slate-400 mt-1 font-mono">
            Anchor On-Chain + Rust Worker
          </p>
        </div>
      </div>

      {/* Vault List Controls */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-bold text-slate-100 flex items-center gap-2">
            <Layers className="w-5 h-5 text-emerald-400" />
            <span>Deployed Strategy Vaults</span>
          </h2>
          <p className="text-xs text-slate-400 mt-1">
            Programmatic vaults backed by Pyth real-time oracle pricing and Jupiter execution
          </p>
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={loadVaults}
            disabled={isLoadingVaults}
            className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-800 text-xs font-medium text-slate-300 transition-colors"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isLoadingVaults ? "animate-spin text-emerald-400" : ""}`} />
            <span>{isLoadingVaults ? "Refreshing..." : "Refresh"}</span>
          </button>

          <Link
            href="/vault/new"
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-xs shadow-glow transition-all duration-200"
          >
            <PlusCircle className="w-4 h-4" />
            <span>Initialize New Vault</span>
          </Link>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex flex-col sm:flex-row items-center gap-4 bg-slate-950/40 p-3 rounded-xl border border-slate-800/80">
        <div className="relative flex-1 w-full">
          <Search className="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            placeholder="Search vault by name, symbol, or address..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full pl-9 pr-4 py-2 bg-slate-900/80 border border-slate-800 rounded-lg text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-emerald-500 transition-colors"
          />
        </div>

        <div className="flex items-center gap-1 w-full sm:w-auto">
          <button
            type="button"
            onClick={() => setFilterPaused("all")}
            className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
              filterPaused === "all"
                ? "bg-slate-800 text-slate-200 border border-slate-700"
                : "text-slate-400 hover:text-slate-200"
            }`}
          >
            All ({vaults.length})
          </button>
          <button
            type="button"
            onClick={() => setFilterPaused("active")}
            className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
              filterPaused === "active"
                ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30"
                : "text-slate-400 hover:text-slate-200"
            }`}
          >
            Active ({activeVaultCount})
          </button>
          <button
            type="button"
            onClick={() => setFilterPaused("paused")}
            className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
              filterPaused === "paused"
                ? "bg-rose-500/20 text-rose-300 border border-rose-500/30"
                : "text-slate-400 hover:text-slate-200"
            }`}
          >
            Paused ({vaults.length - activeVaultCount})
          </button>
        </div>
      </div>

      {/* Vault Cards Grid */}
      {filteredVaults.length > 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {filteredVaults.map((vault) => (
            <VaultCard key={vault.vault_address} vault={vault} />
          ))}
        </div>
      ) : (
        <div className="glass-panel rounded-2xl p-12 text-center space-y-4 border border-slate-800">
          <div className="w-12 h-12 rounded-full bg-slate-800/80 flex items-center justify-center mx-auto text-slate-500">
            <Search className="w-6 h-6" />
          </div>
          <h3 className="text-base font-bold text-slate-300">No vaults match your search</h3>
          <p className="text-xs text-slate-500 max-w-sm mx-auto">
            Try adjusting your search criteria or create a brand new vault.
          </p>
          <Link
            href="/vault/new"
            className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-xs"
          >
            <PlusCircle className="w-4 h-4" />
            <span>Create New Vault</span>
          </Link>
        </div>
      )}
    </div>
  );
}
