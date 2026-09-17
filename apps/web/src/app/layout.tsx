import type { Metadata } from "next";
import { Inter } from "next/font/google";

import "./globals.css";
import { Navbar } from "../components/Navbar";
import { LiveMarketTicker } from "../components/LiveMarketTicker";
import { WalletProvider } from "../components/WalletProvider";
import { StoreProvider } from "../components/StoreProvider";

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
    <html lang="en" className="overflow-x-hidden">
      <body className={`${inter.className} min-h-screen flex flex-col bg-slate-50/50 text-slate-900 selection:bg-emerald-100 selection:text-emerald-900 overflow-x-hidden w-full max-w-full`}>
        <StoreProvider>
          <WalletProvider>
            <Navbar />
            <LiveMarketTicker />
            <main className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8">
              {children}
            </main>
          </WalletProvider>
        </StoreProvider>
      </body>
    </html>
  );
}
