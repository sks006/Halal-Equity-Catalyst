"use client";

import React, { useState } from "react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import { Check, Copy, LogOut, Wallet } from "lucide-react";

import { useSolana } from "../hooks/useSolana";
import { useWallet } from "../hooks/useWallet";

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
      <button
        onClick={() => setVisible(true)}
        disabled={connecting}
        className="flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-lg bg-emerald-500 hover:bg-emerald-400 text-slate-950 shadow-glow transition-all duration-200"
      >
        <Wallet className="w-4 h-4" />
        {connecting ? "Connecting..." : "Connect Wallet"}
      </button>
    );
  }

  return (
    <div className="relative">
      <div
        onClick={() => setShowDropdown(!showDropdown)}
        className="flex items-center gap-3 px-3 py-1.5 rounded-lg glass-panel border border-slate-700/60 hover:border-emerald-500/40 cursor-pointer transition-all duration-200"
      >
        {/* SOL Balance Pill */}
        <div className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 text-xs font-semibold">
          <span>{isLoading ? "..." : `${balanceSol} SOL`}</span>
        </div>

        {/* Address */}
        <span className="text-xs font-mono font-medium text-slate-200">
          {shortAddress}
        </span>

        {/* Copy Button */}
        <button
          onClick={handleCopy}
          title="Copy Address"
          className="p-1 rounded hover:bg-slate-800 text-slate-400 hover:text-slate-200 transition-colors"
        >
          {copied ? (
            <Check className="w-3.5 h-3.5 text-emerald-400" />
          ) : (
            <Copy className="w-3.5 h-3.5" />
          )}
        </button>
      </div>

      {/* Dropdown Menu */}
      {showDropdown && (
        <div className="absolute right-0 mt-2 w-48 rounded-lg glass-panel border border-slate-700 p-1.5 shadow-2xl z-50 animate-in fade-in zoom-in-95">
          <div className="px-3 py-2 border-b border-slate-800/80">
            <p className="text-[11px] text-slate-400">Connected Public Key</p>
            <p className="text-xs font-mono text-slate-300 truncate">
              {publicKey?.toBase58()}
            </p>
          </div>
          <button
            onClick={() => {
              disconnect();
              setShowDropdown(false);
            }}
            className="w-full mt-1 flex items-center gap-2 px-3 py-2 text-xs text-rose-400 hover:bg-rose-500/10 rounded-md transition-colors"
          >
            <LogOut className="w-3.5 h-3.5" />
            Disconnect Wallet
          </button>
        </div>
      )}
    </div>
  );
}
