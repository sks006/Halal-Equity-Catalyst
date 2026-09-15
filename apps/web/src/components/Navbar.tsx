"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Activity, BarChart3, Layers, PlusCircle, Rocket, Sparkles, Zap } from "lucide-react";

import { WalletButton } from "./WalletButton";
import { Badge } from "./ui/badge";

export function Navbar() {
  const pathname = usePathname();

  const navItems = [
    { label: "Dashboard", href: "/dashboard", icon: Layers },
    { label: "DBC Launchpad", href: "/launch", icon: Rocket },
    { label: "DBC Simulator", href: "/launch/simulate", icon: BarChart3 },
    { label: "Demo Pipeline", href: "/demo", icon: Sparkles },
    { label: "New Vault", href: "/vault/new", icon: PlusCircle },
  ];

  return (
    <header className="sticky top-0 z-40 w-full border-b border-slate-200 bg-white/95 backdrop-blur-md">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
        {/* Brand Logo */}
        <div className="flex items-center gap-8">
          <Link href="/dashboard" className="flex items-center gap-2.5 group">
            <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-600 group-hover:scale-105 transition-transform">
              <Zap className="w-4 h-4 fill-emerald-600" />
            </div>
            <div>
              <span className="text-base font-bold tracking-tight text-slate-900">
                Equity Catalyst
              </span>
              <span className="text-[10px] text-slate-500 block uppercase tracking-wider -mt-1 font-mono">
                Decentralized Alpha Vaults
              </span>
            </div>
          </Link>

          {/* Navigation Links */}
          <nav className="hidden md:flex items-center gap-1.5">
            {navItems.map((item) => {
              const Icon = item.icon;
              const isActive = pathname === item.href;
              return (
                <Link
                  key={item.href}
                  href={item.href}
                  className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors ${
                    isActive
                      ? "bg-slate-100 text-slate-900 border border-slate-200 shadow-sm"
                      : "text-slate-600 hover:text-slate-900 hover:bg-slate-50"
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
          <Badge variant="cyan" className="hidden sm:flex items-center gap-1.5 font-mono text-[11px] py-1">
            <Activity className="w-3 h-3 animate-pulse" />
            <span>Solana Devnet</span>
          </Badge>

          <WalletButton />
        </div>
      </div>
    </header>
  );
}
