"use client";

import React from "react";
import Link from "next/link";
import { ArrowUpRight, Coins, PauseCircle, PlayCircle, ShieldCheck } from "lucide-react";
import { VaultModel } from "@equity-catalyst/sdk";

interface Props {
  vault: VaultModel;
}

export function VaultCard({ vault }: Props) {
  const shortAuthority = `${vault.authority.slice(0, 4)}...${vault.authority.slice(-4)}`;
  const shortAddress = `${vault.vault_address.slice(0, 4)}...${vault.vault_address.slice(-4)}`;

  return (
    <div className="glass-card rounded-xl p-5 relative flex flex-col justify-between group">
      <div>
        {/* Header */}
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-2.5">
            <div className="w-10 h-10 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400 font-bold text-sm">
              {vault.symbol.slice(0, 3)}
            </div>
            <div>
              <h3 className="text-base font-bold text-slate-100 group-hover:text-emerald-400 transition-colors">
                {vault.name}
              </h3>
              <p className="text-xs font-mono text-slate-400">{vault.symbol}</p>
            </div>
          </div>

          {/* Status Badge */}
          {vault.is_paused ? (
            <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full badge-rose text-[11px] font-medium">
              <PauseCircle className="w-3 h-3" />
              Paused
            </span>
          ) : (
            <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full badge-emerald text-[11px] font-medium">
              <PlayCircle className="w-3 h-3" />
              Active
            </span>
          )}
        </div>

        {/* Metric Grid */}
        <div className="grid grid-cols-2 gap-3 mt-5 p-3 rounded-lg bg-slate-950/40 border border-slate-800/60">
          <div>
            <p className="text-[11px] text-slate-400 font-medium">Total Deposits</p>
            <p className="text-sm font-bold text-slate-100 mt-0.5">
              ${(vault.total_deposits / 1_000_000).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </p>
          </div>
          <div>
            <p className="text-[11px] text-slate-400 font-medium">Total Shares</p>
            <p className="text-sm font-bold text-slate-100 mt-0.5">
              {vault.total_shares.toLocaleString()}
            </p>
          </div>
        </div>

        {/* Metadata */}
        <div className="mt-4 space-y-1 text-[11px] text-slate-400 font-mono">
          <div className="flex justify-between">
            <span>Vault PDA:</span>
            <span className="text-slate-300">{shortAddress}</span>
          </div>
          <div className="flex justify-between">
            <span>Authority:</span>
            <span className="text-slate-300">{shortAuthority}</span>
          </div>
        </div>
      </div>

      {/* Action Footer */}
      <div className="mt-6 pt-4 border-t border-slate-800/80 flex items-center justify-between">
        <div className="flex items-center gap-1.5 text-xs text-slate-400">
          <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
          <span>Risk Shielded</span>
        </div>

        <Link
          href={`/vault/${vault.vault_address}`}
          className="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg bg-slate-800/80 hover:bg-emerald-500 hover:text-slate-950 text-xs font-semibold text-slate-200 transition-all duration-200"
        >
          <span>View Vault</span>
          <ArrowUpRight className="w-3.5 h-3.5" />
        </Link>
      </div>
    </div>
  );
}
