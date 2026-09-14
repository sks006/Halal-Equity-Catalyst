"use client";

import React from "react";
import { CheckCircle2, ExternalLink, Loader2, X, XCircle } from "lucide-react";

import { TxStatus } from "../hooks/useSolanaTx";

interface Props {
  status: TxStatus;
  onClose?: () => void;
}

export function TransactionStatus({ status, onClose }: Props) {
  if (status.stage === "idle") return null;

  const isPending =
    status.stage === "signing" ||
    status.stage === "sending" ||
    status.stage === "confirming";

  return (
    <div className="fixed bottom-6 right-6 max-w-md w-full z-50 animate-in slide-in-from-bottom-5 fade-in duration-200">
      <div className="bg-white rounded-xl p-4 border border-slate-200 shadow-xl relative">
        {onClose && !isPending && (
          <button
            onClick={onClose}
            className="absolute top-3 right-3 text-slate-400 hover:text-slate-600"
          >
            <X className="w-4 h-4" />
          </button>
        )}

        <div className="flex items-start gap-3">
          {/* Status Icon */}
          <div className="mt-0.5">
            {isPending && (
              <Loader2 className="w-5 h-5 text-emerald-600 animate-spin" />
            )}
            {status.stage === "confirmed" && (
              <CheckCircle2 className="w-5 h-5 text-emerald-600" />
            )}
            {status.stage === "error" && (
              <XCircle className="w-5 h-5 text-rose-600" />
            )}
          </div>

          <div className="flex-1 pr-4">
            <h4 className="text-sm font-semibold text-slate-900 capitalize">
              {status.stage === "signing" && "Awaiting Wallet Signature"}
              {status.stage === "sending" && "Submitting Transaction"}
              {status.stage === "confirming" && "Confirming On-Chain"}
              {status.stage === "confirmed" && "Transaction Confirmed"}
              {status.stage === "error" && "Transaction Failed"}
            </h4>

            <p className="text-xs text-slate-500 mt-0.5">
              {status.stage === "signing" && "Please sign the instruction in your connected wallet."}
              {status.stage === "sending" && "Broadcasting transaction to Solana Devnet nodes..."}
              {status.stage === "confirming" && "Waiting for finalized block commitment."}
              {status.stage === "confirmed" && "State change successfully recorded on Solana Devnet."}
              {status.stage === "error" && (status.error || "An unexpected error occurred.")}
            </p>

            {status.signature && (
              <a
                href={`https://explorer.solana.com/tx/${status.signature}?cluster=devnet`}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 text-[11px] text-emerald-700 hover:text-emerald-800 font-semibold mt-2 font-mono"
              >
                <span>View on Solana Explorer</span>
                <ExternalLink className="w-3 h-3" />
              </a>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
