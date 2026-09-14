"use client";

import { useMemo } from "react";
import { AnchorProvider, Program } from "@coral-xyz/anchor";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";

import idl from "../lib/idl/equity_vault.json";

export const PROGRAM_ID = new PublicKey(
  "8NhtqxR1mwq7a3HTUtcGABNZ3KWzQi9KM3fXu3rHS8LH"
);

export function useAnchorProgram() {
  const { connection } = useConnection();
  const wallet = useWallet();

  const provider = useMemo(() => {
    if (!wallet || !wallet.publicKey || !wallet.signTransaction) {
      return null;
    }
    return new AnchorProvider(
      connection,
      wallet as any,
      AnchorProvider.defaultOptions()
    );
  }, [connection, wallet]);

  const program = useMemo(() => {
    if (!provider) return null;
    return new Program(idl as any, provider);
  }, [provider]);

  return {
    program,
    provider,
    programId: PROGRAM_ID,
  };
}
