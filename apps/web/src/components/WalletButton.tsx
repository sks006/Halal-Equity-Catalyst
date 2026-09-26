"use client";

import React, { useState } from "react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import { Check, Copy, LogOut, Wallet } from "lucide-react";

import { useSolana } from "../hooks/useSolana";
import { useWallet } from "../hooks/useWallet";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";

export function WalletButton() {
  const { connected, connecting, shortAddress, publicKey, disconnect } = useWallet();
  const { balanceSol, isLoading } = useSolana();
  const { setVisible } = useWalletModal();

  const [copied, setCopied] = useState(false);
  const [showDropdown, setShowDropdown] = useState(false);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (publicKey) {
      navigator.clipboard.writeText(publicKey.toBase58());
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  if (!connected) {
    return (
      <Button
        variant="emerald"
        size="sm"
        onClick={() => setVisible(true)}
        disabled={connecting}
        className="flex items-center gap-2"
      >
        <Wallet className="w-4 h-4" />
        <span>{connecting ? "Connecting..." : "Connect Wallet"}</span>
      </Button>
    );
  }

  return (
    <div className="relative">
      <div
        onClick={() => setShowDropdown(!showDropdown)}
        className="flex items-center gap-2.5 px-3 py-1.5 rounded-lg border border-slate-200 bg-white hover:bg-slate-50 cursor-pointer shadow-sm transition-all"
      >
        {/* SOL Balance Pill */}
        <Badge variant="success" className="text-xs px-2 py-0.5">
          {isLoading ? "..." : `${balanceSol} SOL`}
        </Badge>

        {/* Address */}
        <span className="text-xs font-mono font-semibold text-slate-800">
          {shortAddress}
        </span>

        {/* Copy Button */}
        <button
          onClick={handleCopy}
          title="Copy Address"
          className="p-1 rounded hover:bg-slate-100 text-slate-400 hover:text-slate-700 transition-colors"
        >
          {copied ? (
            <Check className="w-3.5 h-3.5 text-emerald-600" />
          ) : (
            <Copy className="w-3.5 h-3.5" />
          )}
        </button>
      </div>

      {/* Dropdown Menu */}
      {showDropdown && (
        <div className="absolute right-0 mt-2 w-52 rounded-xl border border-slate-200 bg-white p-2 shadow-lg z-50 animate-in fade-in zoom-in-95">
          <div className="px-3 py-2 border-b border-slate-100">
            <p className="text-[11px] text-slate-500 font-medium">Connected Address</p>
            <p className="text-xs font-mono text-slate-800 font-bold truncate mt-0.5">
              {publicKey?.toBase58()}
            </p>
          </div>
          <button
            onClick={() => {
              disconnect();
              setShowDropdown(false);
            }}
            className="w-full mt-1.5 flex items-center gap-2 px-3 py-2 text-xs font-medium text-rose-600 hover:bg-rose-50 rounded-lg transition-colors"
          >
            <LogOut className="w-3.5 h-3.5" />
            <span>Disconnect Wallet</span>
          </button>
        </div>
      )}
    </div>
  );
}
