"use client";

import React, { useMemo } from "react";
import {
  ConnectionProvider,
  WalletProvider as SolanaWalletProvider,
} from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";

// Include wallet adapter default styles
import "@solana/wallet-adapter-react-ui/styles.css";

interface Props {
  children: React.ReactNode;
}

export function WalletProvider({ children }: Props) {
  const endpoint =
    process.env.NEXT_PUBLIC_SOLANA_RPC_URL || "https://api.devnet.solana.com";

  // Standard wallet discovery (Phantom, Solflare, Backpack, etc.) works automatically
  const wallets = useMemo(() => [], []);

  return (
    <ConnectionProvider endpoint={endpoint}>
      <SolanaWalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>{children}</WalletModalProvider>
      </SolanaWalletProvider>
    </ConnectionProvider>
  );
}
