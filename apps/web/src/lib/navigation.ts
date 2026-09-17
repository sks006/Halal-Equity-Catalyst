import {
  LayoutDashboard,
  Coins,
  PieChart,
  TrendingUp,
  Bot,
  FileCheck2,
  History,
  ShieldAlert,
  Settings as SettingsIcon,
  LucideIcon,
} from "lucide-react";

export interface NavItem {
  id: string;
  label: string;
  href: string;
  icon: LucideIcon;
  description: string;
  badge?: string;
}

export interface NavCategory {
  category: string;
  items: NavItem[];
}

/**
 * Canonical Application Information Architecture (Phase 07.1)
 * Covers the 9 core functional areas of Equity Catalyst.
 */
export const APP_NAVIGATION: NavItem[] = [
  {
    id: "dashboard",
    label: "Dashboard",
    href: "/dashboard",
    icon: LayoutDashboard,
    description: "Vault overview, aggregate value, active positions, and high-level risk",
  },
  {
    id: "assets",
    label: "Assets",
    href: "/assets",
    icon: Coins,
    description: "PreStocks & Tessera tokenized equities, live Pyth Pro feeds, and mint metadata",
  },
  {
    id: "portfolio",
    label: "Portfolio",
    href: "/portfolio",
    icon: PieChart,
    description: "Position breakdown, target vs. actual weights, cash, and rebalance triggers",
  },
  {
    id: "markets",
    label: "Markets",
    href: "/markets",
    icon: TrendingUp,
    description: "Meteora Dynamic Bonding Curve pools, real-time liquidity, and curve progress",
  },
  {
    id: "agent",
    label: "AI Agent",
    href: "/agent",
    icon: Bot,
    description: "Autonomous keeper recommendations, reasoning logs, and deterministic gate verdicts",
    badge: "Active",
  },
  {
    id: "proposals",
    label: "Proposals",
    href: "/proposals",
    icon: FileCheck2,
    description: "6-stage proposal review: Proposed, Validated, Approved, Simulated, Executed, Failed",
  },
  {
    id: "executions",
    label: "Executions",
    href: "/executions",
    icon: History,
    description: "On-chain transaction signatures, confirmation receipts, and slippage reconciliation",
  },
  {
    id: "risk",
    label: "Risk",
    href: "/risk",
    icon: ShieldAlert,
    description: "Position exposure limits, portfolio drawdowns, unencumbered cash reserves, and spot circuit breakers",
  },
  {
    id: "settings",
    label: "Settings",
    href: "/settings",
    icon: SettingsIcon,
    description: "Solana RPC endpoints, API connectivity, slippage bounds, and signing boundaries",
  },
];
