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
  DepositParams,
  InitializeVaultParams,
  SYSTEM_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  TogglePauseParams,
  VaultAccount,
  VaultModel,
  WithdrawParams,
} from "./types";
import { findPolicyPda } from "./policies";

// --- Anchor Instruction Discriminators ---
const INITIALIZE_VAULT_DISCRIMINATOR = Buffer.from([
  48, 191, 163, 44, 71, 129, 63, 164,
]);
const DEPOSIT_DISCRIMINATOR = Buffer.from([
  242, 35, 68, 137, 82, 225, 242, 182,
]);
const WITHDRAW_DISCRIMINATOR = Buffer.from([
  183, 18, 70, 156, 148, 109, 161, 34,
]);
const EMERGENCY_EXIT_DISCRIMINATOR = Buffer.from([
  206, 8, 114, 219, 141, 142, 218, 230,
]);

/**
 * Derives the Vault PDA: `[b"vault", authority, name]`
 */
export function findVaultPda(
  authority: PublicKey,
  name: string,
  programId: PublicKey = DEFAULT_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), authority.toBuffer(), Buffer.from(name)],
    programId
  );
}

/**
 * Derives UserShares PDA: `[b"user_shares", vault, user]`
 */
export function findUserSharesPda(
  vault: PublicKey,
  user: PublicKey,
  programId: PublicKey = DEFAULT_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("user_shares"), vault.toBuffer(), user.toBuffer()],
    programId
  );
}

/**
 * Vaults sub-client providing state queries and instruction assemblers.
 */
export class VaultsClient {
  constructor(
    private readonly connection: Connection,
    private readonly programId: PublicKey = DEFAULT_PROGRAM_ID,
    private readonly apiUrl?: string
  ) {}

  /** Builds an initialize_vault instruction */
  public buildInitializeVaultIx(
    params: InitializeVaultParams
  ): TransactionInstruction {
    const [vaultPda] = findVaultPda(params.authority, params.name, this.programId);
    const [policyPda] = findPolicyPda(vaultPda, this.programId);

    const nameBytes = Buffer.from(params.name);
    const symbolBytes = Buffer.from(params.symbol);

    // Serialization: disc(8) + name_len(4) + name + sym_len(4) + sym + min_cash(2) + max_pos(2)
    const data = Buffer.alloc(8 + 4 + nameBytes.length + 4 + symbolBytes.length + 2 + 2);
    let offset = 0;
    INITIALIZE_VAULT_DISCRIMINATOR.copy(data, offset);
    offset += 8;

    data.writeUInt32LE(nameBytes.length, offset);
    offset += 4;
    nameBytes.copy(data, offset);
    offset += nameBytes.length;

    data.writeUInt32LE(symbolBytes.length, offset);
    offset += 4;
    symbolBytes.copy(data, offset);
    offset += symbolBytes.length;

    data.writeUInt16LE(params.minCashBps, offset);
    offset += 2;
    data.writeUInt16LE(params.maxPositionBps, offset);

    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: params.authority, isSigner: true, isWritable: true },
        { pubkey: vaultPda, isSigner: false, isWritable: true },
        { pubkey: policyPda, isSigner: false, isWritable: true },
        { pubkey: params.assetMint, isSigner: false, isWritable: false },
        { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data,
    });
  }

  /** Builds a deposit instruction */
  public buildDepositIx(params: DepositParams): TransactionInstruction {
    const [userSharesPda] = findUserSharesPda(params.vault, params.user, this.programId);
    const userAssetAccount = getAssociatedTokenAddressSync(params.assetMint, params.user);
    const vaultAssetAccount = getAssociatedTokenAddressSync(params.assetMint, params.vault, true);

    const amountBn = new BN(params.amount.toString());
    const data = Buffer.alloc(8 + 8);
    DEPOSIT_DISCRIMINATOR.copy(data, 0);
    amountBn.toArrayLike(Buffer, "le", 8).copy(data, 8);

    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: params.user, isSigner: true, isWritable: true },
        { pubkey: params.vault, isSigner: false, isWritable: true },
        { pubkey: userSharesPda, isSigner: false, isWritable: true },
        { pubkey: userAssetAccount, isSigner: false, isWritable: true },
        { pubkey: vaultAssetAccount, isSigner: false, isWritable: true },
        { pubkey: params.assetMint, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data,
    });
  }

  /** Builds a withdraw instruction */
  public buildWithdrawIx(params: WithdrawParams): TransactionInstruction {
    const [userSharesPda] = findUserSharesPda(params.vault, params.user, this.programId);
    const userAssetAccount = getAssociatedTokenAddressSync(params.assetMint, params.user);
    const vaultAssetAccount = getAssociatedTokenAddressSync(params.assetMint, params.vault, true);

    const sharesBn = new BN(params.sharesToBurn.toString());
    const data = Buffer.alloc(8 + 8);
    WITHDRAW_DISCRIMINATOR.copy(data, 0);
    sharesBn.toArrayLike(Buffer, "le", 8).copy(data, 8);

    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: params.user, isSigner: true, isWritable: true },
        { pubkey: params.vault, isSigner: false, isWritable: true },
        { pubkey: userSharesPda, isSigner: false, isWritable: true },
        { pubkey: userAssetAccount, isSigner: false, isWritable: true },
        { pubkey: vaultAssetAccount, isSigner: false, isWritable: true },
        { pubkey: params.assetMint, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data,
    });
  }

  /** Builds an emergency pause/unpause toggle instruction */
  public buildTogglePauseIx(params: TogglePauseParams): TransactionInstruction {
    const data = Buffer.alloc(8 + 1);
    EMERGENCY_EXIT_DISCRIMINATOR.copy(data, 0);
    data.writeUInt8(params.isPaused ? 1 : 0, 8);

    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: params.authority, isSigner: true, isWritable: true },
        { pubkey: params.vault, isSigner: false, isWritable: true },
      ],
      data,
    });
  }

  /** Assembles a ready-to-sign initialize vault transaction */
  public async createInitializeVaultTx(
    params: InitializeVaultParams
  ): Promise<Transaction> {
    const ix = this.buildInitializeVaultIx(params);
    const tx = new Transaction().add(ix);
    tx.feePayer = params.authority;
    return tx;
  }

  /** Assembles a ready-to-sign deposit transaction */
  public async createDepositTx(params: DepositParams): Promise<Transaction> {
    const ix = this.buildDepositIx(params);
    const tx = new Transaction().add(ix);
    tx.feePayer = params.user;
    return tx;
  }

  /** Assembles a ready-to-sign withdraw transaction */
  public async createWithdrawTx(params: WithdrawParams): Promise<Transaction> {
    const ix = this.buildWithdrawIx(params);
    const tx = new Transaction().add(ix);
    tx.feePayer = params.user;
    return tx;
  }

  /** Fetches a vault record from the API backend */
  public async getVaultFromApi(vaultAddress: string): Promise<VaultModel | null> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }
    const resp = await fetch(`${this.apiUrl}/vaults/${vaultAddress}`);
    if (resp.status === 404) return null;
    const data = await resp.json();
    return data as VaultModel;
  }
}
