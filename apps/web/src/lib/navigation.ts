import {
  LayoutDashboard,
  TrendingUp,
  PieChart,
  History,
  Settings as SettingsIcon,
  Bot,
  FileCheck2,
  ShieldAlert,
  Coins,
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

/**
 * Primary Navigation Architecture
 * Strictly simplified to the 4 essential user tasks:
 * 1. Dashboard
 * 2. Markets
 * 3. Portfolio
 * 4. Activity
 */
export const APP_NAVIGATION: NavItem[] = [
  {
    id: "dashboard",
    label: "Dashboard",
    href: "/dashboard",
    icon: LayoutDashboard,
    description: "Portfolio value, holdings, market overview, and safety status",
  },
  {
    id: "markets",
    label: "Markets",
    href: "/markets",
    icon: TrendingUp,
    description: "Explore and trade verified equity assets",
  },
  {
    id: "portfolio",
    label: "Portfolio",
    href: "/portfolio",
    icon: PieChart,
    description: "Manage holdings, cash balance, and target allocation",
  },
  {
    id: "activity",
    label: "Activity",
    href: "/activity",
    icon: History,
    description: "Recent trades, rebalances, and transaction history",
  },
];

/**
 * Secondary Navigation:
 * Kept outside of primary navigation flow.
 */
export const SECONDARY_NAVIGATION: NavItem[] = [
  {
    id: "settings",
    label: "Settings",
    href: "/settings",
    icon: SettingsIcon,
    description: "Network connection, RPC settings, and slippage defaults",
  },
];

/**
 * Advanced Technical Views:
 * Preserved for deep technical audits, accessible via Activity or Settings,
 * never exposed as primary navigation items.
 */
export const ADVANCED_NAVIGATION: NavItem[] = [
  {
    id: "agent",
    label: "Automation",
    href: "/agent",
    icon: Bot,
    description: "Strategy automation engine suggestions and reasoning logs",
  },
  {
    id: "proposals",
    label: "Proposals",
    href: "/proposals",
    icon: FileCheck2,
    description: "Multi-stage proposal review and simulation logs",
  },
  {
    id: "executions",
    label: "Executions",
    href: "/executions",
    icon: History,
    description: "On-chain signatures, confirmation receipts, and slippage reconciliation",
  },
  {
    id: "risk",
    label: "Risk",
    href: "/risk",
    icon: ShieldAlert,
    description: "Position exposure limits, cash reserves, and spot circuit breakers",
  },
  {
    id: "settings",
    label: "Settings",
    href: "/settings",
    icon: SettingsIcon,
    description: "Network configuration, RPC endpoints, and slippage parameters",
  },
];
