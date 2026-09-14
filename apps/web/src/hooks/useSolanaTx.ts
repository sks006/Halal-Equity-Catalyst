"use client";

import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { Transaction, TransactionInstruction } from "@solana/web3.js";
import { useCallback, useState } from "react";

export type TxStage =
  | "idle"
  | "signing"
  | "sending"
  | "confirming"
  | "confirmed"
  | "error";

export interface TxStatus {
  stage: TxStage;
  signature?: string;
  error?: string;
}

export function useSolanaTx() {
  const { connection } = useConnection();
  const { publicKey, sendTransaction, signTransaction } = useWallet();

  const [status, setStatus] = useState<TxStatus>({ stage: "idle" });

  const reset = useCallback(() => {
    setStatus({ stage: "idle" });
  }, []);

  const executeTx = useCallback(
    async (
      transactionOrIxs: Transaction | TransactionInstruction | TransactionInstruction[]
    ): Promise<string> => {
      if (!publicKey) {
        const err = "Wallet is not connected";
        setStatus({ stage: "error", error: err });
        throw new Error(err);
      }

      try {
        let tx: Transaction;
        if (transactionOrIxs instanceof Transaction) {
          tx = transactionOrIxs;
        } else if (Array.isArray(transactionOrIxs)) {
          tx = new Transaction().add(...transactionOrIxs);
        } else {
          tx = new Transaction().add(transactionOrIxs);
        }

        setStatus({ stage: "signing" });

        const latestBlockhash = await connection.getLatestBlockhash("confirmed");
        tx.recentBlockhash = latestBlockhash.blockhash;
        tx.feePayer = tx.feePayer || publicKey;

        setStatus({ stage: "sending" });
        const signature = await sendTransaction(tx, connection, {
          skipPreflight: false,
          preflightCommitment: "confirmed",
        });

        setStatus({ stage: "confirming", signature });

        const confirmation = await connection.confirmTransaction(
          {
            signature,
            blockhash: latestBlockhash.blockhash,
            lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
          },
          "confirmed"
        );

        if (confirmation.value.err) {
          const errStr = `Transaction failed: ${JSON.stringify(confirmation.value.err)}`;
          setStatus({ stage: "error", signature, error: errStr });
          throw new Error(errStr);
        }

        setStatus({ stage: "confirmed", signature });
        return signature;
      } catch (err: any) {
        const errorMsg = err?.message || String(err);
        setStatus({ stage: "error", error: errorMsg });
        throw err;
      }
    },
    [connection, publicKey, sendTransaction]
  );

  return {
    status,
    executeTx,
    reset,
  };
}
