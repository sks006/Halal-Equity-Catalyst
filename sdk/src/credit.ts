import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import BN from "bn.js";

import {
  BorrowParams,
  DEFAULT_PROGRAM_ID,
  RepayParams,
  SYSTEM_PROGRAM_ID,
} from "./types";
import { findPolicyPda } from "./policies";

// Anchor Instruction Discriminators
const BORROW_DISCRIMINATOR = Buffer.from([
  228, 253, 131, 202, 207, 116, 89, 18,
]);
const REPAY_DISCRIMINATOR = Buffer.from([
  234, 103, 67, 82, 208, 234, 219, 166,
]);

/**
 * Derives the Loan PDA: `[b"loan", vault, borrower]`
 */
export function findLoanPda(
  vault: PublicKey,
  borrower: PublicKey,
  programId: PublicKey = DEFAULT_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("loan"), vault.toBuffer(), borrower.toBuffer()],
    programId
  );
}

/**
 * Credit sub-client managing loan collateralization, borrowing, and repayments.
 */
export class CreditClient {
  constructor(
    private readonly connection: Connection,
    private readonly programId: PublicKey = DEFAULT_PROGRAM_ID,
    private readonly apiUrl?: string
  ) {}

  /** Builds an on-chain borrow instruction */
  public buildBorrowIx(params: BorrowParams): TransactionInstruction {
    const [loanPda] = findLoanPda(params.vault, params.borrower, this.programId);
    const [policyPda] = findPolicyPda(params.vault, this.programId);

    const amountBn = new BN(params.borrowAmount.toString());
    const data = Buffer.alloc(8 + 8);
    BORROW_DISCRIMINATOR.copy(data, 0);
    amountBn.toArrayLike(Buffer, "le", 8).copy(data, 8);

    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: params.borrower, isSigner: true, isWritable: true },
        { pubkey: params.vault, isSigner: false, isWritable: true },
        { pubkey: loanPda, isSigner: false, isWritable: true },
        { pubkey: policyPda, isSigner: false, isWritable: false },
        { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data,
    });
  }

  /** Builds an on-chain repay instruction */
  public buildRepayIx(params: RepayParams): TransactionInstruction {
    const [loanPda] = findLoanPda(params.vault, params.borrower, this.programId);

    const amountBn = new BN(params.repayAmount.toString());
    const data = Buffer.alloc(8 + 8);
    REPAY_DISCRIMINATOR.copy(data, 0);
    amountBn.toArrayLike(Buffer, "le", 8).copy(data, 8);

    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: params.borrower, isSigner: true, isWritable: true },
        { pubkey: params.vault, isSigner: false, isWritable: true },
        { pubkey: loanPda, isSigner: false, isWritable: true },
        { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data,
    });
  }

  /** Assembles a ready-to-sign borrow transaction */
  public async createBorrowTx(params: BorrowParams): Promise<Transaction> {
    const ix = this.buildBorrowIx(params);
    const tx = new Transaction().add(ix);
    tx.feePayer = params.borrower;
    return tx;
  }

  /** Assembles a ready-to-sign repay transaction */
  public async createRepayTx(params: RepayParams): Promise<Transaction> {
    const ix = this.buildRepayIx(params);
    const tx = new Transaction().add(ix);
    tx.feePayer = params.borrower;
    return tx;
  }
}
