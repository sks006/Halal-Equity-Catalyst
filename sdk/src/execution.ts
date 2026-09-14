import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import BN from "bn.js";

import {
  DEFAULT_PROGRAM_ID,
  ExecuteActionParams,
  ExecutionModel,
  QuoteExecutionRequest,
  QuoteExecutionVerdict,
  SYSTEM_PROGRAM_ID,
} from "./types";
import { findPolicyPda } from "./policies";
import { findPositionPda } from "./portfolio";

// Anchor Instruction Discriminator for `execute_action`
const EXECUTE_ACTION_DISCRIMINATOR = Buffer.from([
  167, 100, 14, 255, 178, 12, 10, 206,
]);

/**
 * Derives the Execution PDA: `[b"execution", vault, executionId (u64 le bytes)]`
 */
export function findExecutionPda(
  vault: PublicKey,
  executionId: BN | number | string,
  programId: PublicKey = DEFAULT_PROGRAM_ID
): [PublicKey, number] {
  const bn = new BN(executionId.toString());
  const idBytes = bn.toArrayLike(Buffer, "le", 8);
  return PublicKey.findProgramAddressSync(
    [Buffer.from("execution"), vault.toBuffer(), idBytes],
    programId
  );
}

/**
 * Execution sub-client handling quote evaluations and trade execution instructions.
 */
export class ExecutionClient {
  constructor(
    private readonly connection: Connection,
    private readonly programId: PublicKey = DEFAULT_PROGRAM_ID,
    private readonly apiUrl?: string
  ) {}

  /**
   * Submits a quote evaluation request to the API backend (Quote-Only execution mode)
   */
  public async evaluateQuote(
    request: QuoteExecutionRequest
  ): Promise<QuoteExecutionVerdict> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }

    const resp = await fetch(`${this.apiUrl}/quotes/evaluate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });

    if (!resp.ok) {
      const errText = await resp.text();
      throw new Error(`Quote evaluation failed (${resp.status}): ${errText}`);
    }

    const data = await resp.json();
    return data as QuoteExecutionVerdict;
  }

  /**
   * Builds an on-chain execute_action instruction
   */
  public buildExecuteActionIx(
    params: ExecuteActionParams
  ): TransactionInstruction {
    const [policyPda] = findPolicyPda(params.vault, this.programId);
    const [executionPda] = findExecutionPda(params.vault, params.executionId, this.programId);
    const [sourcePosPda] = findPositionPda(params.vault, params.sourceMint, this.programId);
    const [targetPosPda] = findPositionPda(params.vault, params.targetMint, this.programId);
    const vaultAssetAccount = getAssociatedTokenAddressSync(params.sourceMint, params.vault, true);

    const execIdBn = new BN(params.executionId.toString());
    const inAmountBn = new BN(params.inputAmount.toString());
    const minOutAmountBn = new BN(params.minOutputAmount.toString());

    // Layout: disc(8) + action_type(1) + execution_id(8) + input_amount(8) + min_output_amount(8)
    const data = Buffer.alloc(8 + 1 + 8 + 8 + 8);
    let offset = 0;
    EXECUTE_ACTION_DISCRIMINATOR.copy(data, offset);
    offset += 8;

    data.writeUInt8(params.actionType, offset);
    offset += 1;

    execIdBn.toArrayLike(Buffer, "le", 8).copy(data, offset);
    offset += 8;

    inAmountBn.toArrayLike(Buffer, "le", 8).copy(data, offset);
    offset += 8;

    minOutAmountBn.toArrayLike(Buffer, "le", 8).copy(data, offset);

    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: params.authority, isSigner: true, isWritable: true },
        { pubkey: params.vault, isSigner: false, isWritable: true },
        { pubkey: policyPda, isSigner: false, isWritable: false },
        { pubkey: executionPda, isSigner: false, isWritable: true },
        { pubkey: sourcePosPda, isSigner: false, isWritable: true },
        { pubkey: targetPosPda, isSigner: false, isWritable: true },
        { pubkey: vaultAssetAccount, isSigner: false, isWritable: true },
        { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data,
    });
  }

  /**
   * Assembles a ready-to-sign execute_action transaction
   */
  public async createExecuteActionTx(
    params: ExecuteActionParams
  ): Promise<Transaction> {
    const ix = this.buildExecuteActionIx(params);
    const tx = new Transaction().add(ix);
    tx.feePayer = params.authority;
    return tx;
  }

  /**
   * Queries execution history for a vault from the API
   */
  public async listExecutionsByVault(vaultAddress: string): Promise<ExecutionModel[]> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }

    const resp = await fetch(`${this.apiUrl}/vaults/${vaultAddress}/executions`);
    if (!resp.ok) throw new Error(`Failed to list executions: ${resp.statusText}`);
    const data = await resp.json();
    return data as ExecutionModel[];
  }
}
