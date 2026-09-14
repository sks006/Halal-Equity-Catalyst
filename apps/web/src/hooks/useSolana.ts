"use client";

import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { LAMPORTS_PER_SOL } from "@solana/web3.js";
import { useCallback, useEffect, useState } from "react";

export function useSolana() {
  const { connection } = useConnection();
  const { publicKey } = useWallet();

  const [balanceLamports, setBalanceLamports] = useState<number | null>(null);
  const [balanceSol, setBalanceSol] = useState<string>("0.00");
  const [isLoading, setIsLoading] = useState<boolean>(false);

  const refreshBalance = useCallback(async () => {
    if (!publicKey) {
      setBalanceLamports(null);
      setBalanceSol("0.00");
      return;
    }

    try {
      setIsLoading(true);
      const lamports = await connection.getBalance(publicKey, "confirmed");
      setBalanceLamports(lamports);
      setBalanceSol((lamports / LAMPORTS_PER_SOL).toFixed(4));
    } catch (err) {
      console.error("Failed to fetch SOL balance:", err);
    } finally {
      setIsLoading(false);
    }
  }, [connection, publicKey]);

  useEffect(() => {
    refreshBalance();
    const interval = setInterval(refreshBalance, 15_000);
    return () => clearInterval(interval);
  }, [refreshBalance]);

  return {
    connection,
    cluster: "devnet",
    rpcUrl: connection.rpcEndpoint,
    balanceLamports,
    balanceSol,
    isLoading,
    refreshBalance,
  };
}
