import type { Metadata } from "next";
import { Inter } from "next/font/google";

import "./globals.css";
import { Navbar } from "../components/Navbar";
import { WalletProvider } from "../components/WalletProvider";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Equity Catalyst | Decentralized Automated Vaults on Solana",
  description:
    "Autonomous algorithmic portfolio and equity synthetic vaults with on-chain risk policies and real-time Pyth & Jupiter execution.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.className} min-h-screen flex flex-col bg-background text-slate-100 selection:bg-emerald-500/30 selection:text-emerald-200`}>
        <WalletProvider>
          <div className="fixed inset-0 radial-glow pointer-events-none z-0" />
          <div className="fixed inset-0 radial-glow-cyan pointer-events-none z-0" />
          <Navbar />
          <main className="flex-1 relative z-10">{children}</main>
        </WalletProvider>
      </body>
    </html>
  );
}
