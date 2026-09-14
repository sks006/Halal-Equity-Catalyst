"use client";

import React, { useEffect, useState } from "react";
import Link from "next/link";
import {
  Layers,
  PlusCircle,
  RefreshCw,
  Search,
  ShieldCheck,
  TrendingUp,
  Vault as VaultIcon,
  Wallet,
} from "lucide-react";

import { useWallet } from "../../hooks/useWallet";
import { useSolana } from "../../hooks/useSolana";
import { VaultCard } from "../../features/vault/VaultCard";
import { useAppDispatch, useAppSelector } from "../../store/hooks";
import { fetchVaults } from "../../store/vaultsSlice";
import { Card, CardContent } from "../../components/ui/card";
import { Button } from "../../components/ui/button";
import { Badge } from "../../components/ui/badge";
import { Input } from "../../components/ui/input";

export default function DashboardPage() {
  const dispatch = useAppDispatch();
  const { connected, shortAddress } = useWallet();
  const { balanceSol } = useSolana();

  // Redux state for vaults
  const { items: vaults, isLoading: isLoadingVaults } = useAppSelector(
    (state) => state.vaults
  );

  const [searchTerm, setSearchTerm] = useState<string>("");
  const [filterPaused, setFilterPaused] = useState<"all" | "active" | "paused">("all");

  // Fetch vaults through Redux async thunk
  const loadVaults = () => {
    dispatch(fetchVaults());
  };

  useEffect(() => {
    loadVaults();
  }, [dispatch]);

  // Aggregated metrics
  const totalTvl = vaults.reduce((acc, v) => acc + v.total_deposits, 0) / 1_000_000;
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
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Total Value Locked */}
        <Card className="bg-white">
          <CardContent className="p-5">
            <div className="flex items-center justify-between text-slate-500 text-xs font-medium">
              <span>Total Value Locked</span>
              <div className="w-8 h-8 rounded-lg bg-emerald-50 text-emerald-600 flex items-center justify-center">
                <TrendingUp className="w-4 h-4" />
              </div>
            </div>
            <div className="mt-3 flex items-baseline gap-2">
              <span className="text-2xl font-extrabold text-slate-900 font-mono">
                ${totalTvl.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
              </span>
              <Badge variant="success" className="text-[10px] font-bold">+12.4%</Badge>
            </div>
            <p className="text-xs text-slate-500 mt-1">Collateral under programmatic risk shield</p>
          </CardContent>
        </Card>

        {/* Active Vaults */}
        <Card className="bg-white">
          <CardContent className="p-5">
            <div className="flex items-center justify-between text-slate-500 text-xs font-medium">
              <span>Active Vaults</span>
              <div className="w-8 h-8 rounded-lg bg-cyan-50 text-cyan-600 flex items-center justify-center">
                <VaultIcon className="w-4 h-4" />
              </div>
            </div>
            <div className="mt-3 flex items-baseline gap-2">
              <span className="text-2xl font-extrabold text-slate-900 font-mono">{activeVaultCount}</span>
              <span className="text-xs text-slate-500 font-mono">/ {vaults.length} total</span>
            </div>
            <p className="text-xs text-slate-500 mt-1">Anchor PDA smart vaults</p>
          </CardContent>
        </Card>

        {/* Connected Wallet State */}
        <Card className="bg-white">
          <CardContent className="p-5">
            <div className="flex items-center justify-between text-slate-500 text-xs font-medium">
              <span>Connected Wallet</span>
              <div className="w-8 h-8 rounded-lg bg-purple-50 text-purple-600 flex items-center justify-center">
                <Wallet className="w-4 h-4" />
              </div>
            </div>
            <div className="mt-3">
              {connected ? (
                <div>
                  <div className="text-xl font-bold text-slate-900 font-mono">{balanceSol} SOL</div>
                  <div className="text-xs text-slate-500 font-mono mt-0.5">{shortAddress}</div>
                </div>
              ) : (
                <div className="text-sm font-medium text-slate-500 mt-1">Wallet not connected</div>
              )}
            </div>
            <p className="text-xs text-emerald-700 font-mono mt-1">
              {connected ? "Devnet ready for deposits" : "Connect wallet to manage vaults"}
            </p>
          </CardContent>
        </Card>

        {/* Risk & Sentinel Engine */}
        <Card className="bg-white">
          <CardContent className="p-5">
            <div className="flex items-center justify-between text-slate-500 text-xs font-medium">
              <span>Risk Defense Status</span>
              <div className="w-8 h-8 rounded-lg bg-emerald-50 text-emerald-600 flex items-center justify-center">
                <ShieldCheck className="w-4 h-4" />
              </div>
            </div>
            <div className="mt-3 flex items-center gap-2">
              <span className="inline-block w-2.5 h-2.5 rounded-full bg-emerald-500 animate-pulse" />
              <span className="text-sm font-bold text-slate-900 font-mono">Dual-Engine Active</span>
            </div>
            <p className="text-xs text-slate-500 mt-1 font-mono">
              Anchor CPI + Rust Policy Worker
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Vault List Controls */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-bold text-slate-900 flex items-center gap-2">
            <Layers className="w-5 h-5 text-emerald-600" />
            <span>Deployed Strategy Vaults</span>
          </h2>
          <p className="text-xs text-slate-500 mt-1">
            Programmatic vaults backed by Pyth real-time oracle pricing and Jupiter execution (Redux managed)
          </p>
        </div>

        <div className="flex items-center gap-3">
          <Button
            variant="outline"
            size="sm"
            onClick={loadVaults}
            disabled={isLoadingVaults}
            className="flex items-center gap-1.5 text-xs font-medium"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isLoadingVaults ? "animate-spin text-emerald-600" : ""}`} />
            <span>{isLoadingVaults ? "Syncing..." : "Sync Redux Store"}</span>
          </Button>

          <Link href="/vault/new">
            <Button variant="emerald" size="sm" className="flex items-center gap-2 font-bold">
              <PlusCircle className="w-4 h-4" />
              <span>Initialize New Vault</span>
            </Button>
          </Link>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <Card className="bg-white p-3 border-slate-200">
        <div className="flex flex-col sm:flex-row items-center gap-4">
          <div className="relative flex-1 w-full">
            <Search className="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <Input
              type="text"
              placeholder="Search vault by name, symbol, or address..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-9 bg-slate-50/60 border-slate-200"
            />
          </div>

          <div className="flex items-center gap-1.5 w-full sm:w-auto">
            <Button
              variant={filterPaused === "all" ? "default" : "ghost"}
              size="sm"
              onClick={() => setFilterPaused("all")}
              className="text-xs h-8"
            >
              All ({vaults.length})
            </Button>
            <Button
              variant={filterPaused === "active" ? "secondary" : "ghost"}
              size="sm"
              onClick={() => setFilterPaused("active")}
              className={`text-xs h-8 ${filterPaused === "active" ? "text-emerald-700 font-bold bg-emerald-50 border border-emerald-200" : ""}`}
            >
              Active ({activeVaultCount})
            </Button>
            <Button
              variant={filterPaused === "paused" ? "secondary" : "ghost"}
              size="sm"
              onClick={() => setFilterPaused("paused")}
              className={`text-xs h-8 ${filterPaused === "paused" ? "text-rose-700 font-bold bg-rose-50 border border-rose-200" : ""}`}
            >
              Paused ({vaults.length - activeVaultCount})
            </Button>
          </div>
        </div>
      </Card>

      {/* Vault Cards Grid */}
      {filteredVaults.length > 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {filteredVaults.map((vault) => (
            <VaultCard key={vault.vault_address} vault={vault} />
          ))}
        </div>
      ) : (
        <Card className="bg-white p-12 text-center space-y-4">
          <div className="w-12 h-12 rounded-full bg-slate-100 flex items-center justify-center mx-auto text-slate-400">
            <Search className="w-6 h-6" />
          </div>
          <h3 className="text-base font-bold text-slate-800">No vaults match your search</h3>
          <p className="text-xs text-slate-500 max-w-sm mx-auto">
            Try adjusting your search criteria or create a brand new vault.
          </p>
          <Link href="/vault/new">
            <Button variant="emerald" size="sm" className="inline-flex items-center gap-2">
              <PlusCircle className="w-4 h-4" />
              <span>Create New Vault</span>
            </Button>
          </Link>
        </Card>
      )}
    </div>
  );
}
