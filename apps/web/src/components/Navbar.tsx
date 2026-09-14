"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Activity, Layers, PlusCircle, ShieldCheck, Zap } from "lucide-react";

import { WalletButton } from "./WalletButton";

export function Navbar() {
  const pathname = usePathname();

  const navItems = [
    { label: "Dashboard", href: "/dashboard", icon: Layers },
    { label: "New Vault", href: "/vault/new", icon: PlusCircle },
  ];

  return (
    <header className="sticky top-0 z-40 w-full border-b border-slate-800/80 glass-panel">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
        {/* Brand Logo */}
        <div className="flex items-center gap-8">
          <Link href="/dashboard" className="flex items-center gap-2.5 group">
            <div className="w-8 h-8 rounded-lg bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 group-hover:scale-105 transition-transform shadow-glow">
              <Zap className="w-4 h-4 fill-emerald-400/20" />
            </div>
            <div>
              <span className="text-base font-bold tracking-tight bg-gradient-to-r from-emerald-400 to-cyan-400 bg-clip-text text-transparent">
                Equity Catalyst
              </span>
              <span className="text-[10px] text-slate-500 block uppercase tracking-wider -mt-1 font-mono">
                Decentralized Alpha Vaults
              </span>
            </div>
          </Link>

          {/* Navigation Links */}
          <nav className="hidden md:flex items-center gap-1">
            {navItems.map((item) => {
              const Icon = item.icon;
              const isActive = pathname === item.href;
              return (
                <Link
                  key={item.href}
                  href={item.href}
                  className={`flex items-center gap-2 px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
                    isActive
                      ? "bg-slate-800/80 text-emerald-400 border border-slate-700/60"
                      : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/40"
                  }`}
                >
                  <Icon className="w-3.5 h-3.5" />
                  {item.label}
                </Link>
              );
            })}
          </nav>
        </div>

        {/* Right Section: Cluster & Wallet */}
        <div className="flex items-center gap-3">
          <div className="hidden sm:flex items-center gap-1.5 px-2.5 py-1 rounded-full badge-cyan text-[11px] font-mono font-medium">
            <Activity className="w-3 h-3 animate-pulse" />
            <span>Solana Devnet</span>
          </div>

          <WalletButton />
        </div>
      </div>
    </header>
  );
}
