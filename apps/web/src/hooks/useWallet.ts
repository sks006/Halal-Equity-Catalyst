"use client";

import { useWallet as useSolanaWallet } from "@solana/wallet-adapter-react";
import { useMemo } from "react";

export function useWallet() {
  const wallet = useSolanaWallet();

  const shortAddress = useMemo(() => {
    if (!wallet.publicKey) return null;
    const base58 = wallet.publicKey.toBase58();
    return `${base58.slice(0, 4)}...${base58.slice(-4)}`;
  }, [wallet.publicKey]);

  return {
    ...wallet,
    shortAddress,
  };
}
