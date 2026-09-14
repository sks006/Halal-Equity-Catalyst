"use client";

import React from "react";
import Link from "next/link";
import { ArrowUpRight, PauseCircle, PlayCircle, ShieldCheck } from "lucide-react";
import { VaultModel } from "@equity-catalyst/sdk";

import { Card, CardContent, CardFooter, CardHeader } from "../../components/ui/card";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";

interface Props {
  vault: VaultModel;
}

export function VaultCard({ vault }: Props) {
  const shortAuthority = `${vault.authority.slice(0, 4)}...${vault.authority.slice(-4)}`;
  const shortAddress = `${vault.vault_address.slice(0, 4)}...${vault.vault_address.slice(-4)}`;

  return (
    <Card className="flex flex-col justify-between hover:border-slate-300 hover:shadow-md transition-all group bg-white">
      <div>
        <CardHeader className="p-5 pb-3">
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-700 font-bold text-xs font-mono">
                {vault.symbol.slice(0, 3)}
              </div>
              <div>
                <h3 className="text-base font-bold text-slate-900 group-hover:text-emerald-600 transition-colors line-clamp-1">
                  {vault.name}
                </h3>
                <p className="text-xs font-mono text-slate-500 font-semibold">{vault.symbol}</p>
              </div>
            </div>

            {/* Status Badge */}
            {vault.is_paused ? (
              <Badge variant="rose" className="flex items-center gap-1 font-medium">
                <PauseCircle className="w-3 h-3" />
                <span>Paused</span>
              </Badge>
            ) : (
              <Badge variant="success" className="flex items-center gap-1 font-medium">
                <PlayCircle className="w-3 h-3" />
                <span>Active</span>
              </Badge>
            )}
          </div>
        </CardHeader>

        <CardContent className="p-5 pt-2">
          {/* Metric Grid */}
          <div className="grid grid-cols-2 gap-3 p-3 rounded-lg bg-slate-50 border border-slate-100">
            <div>
              <p className="text-[11px] text-slate-500 font-medium">Total Deposits</p>
              <p className="text-sm font-bold text-slate-900 mt-0.5 font-mono">
                ${(vault.total_deposits / 1_000_000).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
              </p>
            </div>
            <div>
              <p className="text-[11px] text-slate-500 font-medium">Total Shares</p>
              <p className="text-sm font-bold text-slate-900 mt-0.5 font-mono">
                {(vault.total_shares / 1_000_000).toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 0 })}
              </p>
            </div>
          </div>

          {/* Metadata */}
          <div className="mt-4 space-y-1.5 text-[11px] text-slate-500 font-mono">
            <div className="flex justify-between">
              <span>Vault PDA:</span>
              <span className="text-slate-800 font-medium">{shortAddress}</span>
            </div>
            <div className="flex justify-between">
              <span>Authority:</span>
              <span className="text-slate-800 font-medium">{shortAuthority}</span>
            </div>
          </div>
        </CardContent>
      </div>

      {/* Action Footer */}
      <CardFooter className="p-5 pt-3 border-t border-slate-100 flex items-center justify-between">
        <div className="flex items-center gap-1.5 text-xs text-slate-600 font-medium">
          <ShieldCheck className="w-4 h-4 text-emerald-600" />
          <span>Risk Guard Active</span>
        </div>

        <Link href={`/vault/${vault.vault_address}`}>
          <Button variant="outline" size="sm" className="flex items-center gap-1 hover:bg-slate-100 text-xs font-semibold">
            <span>View Vault</span>
            <ArrowUpRight className="w-3.5 h-3.5" />
          </Button>
        </Link>
      </CardFooter>
    </Card>
  );
}
