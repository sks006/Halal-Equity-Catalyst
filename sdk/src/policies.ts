import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";

import {
  DEFAULT_PROGRAM_ID,
  PolicyModel,
  UpdatePolicyParams,
} from "./types";

// Anchor Instruction Discriminator for `update_policy`
const UPDATE_POLICY_DISCRIMINATOR = Buffer.from([
  102, 192, 169, 114, 219, 137, 240, 150,
]);

/**
 * Derives the Policy PDA: `[b"policy", vault]`
 */
export function findPolicyPda(
  vault: PublicKey,
  programId: PublicKey = DEFAULT_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("policy"), vault.toBuffer()],
    programId
  );
}

/**
 * Policies sub-client managing vault risk parameters and on-chain updates.
 */
export class PoliciesClient {
  constructor(
    private readonly connection: Connection,
    private readonly programId: PublicKey = DEFAULT_PROGRAM_ID,
    private readonly apiUrl?: string
  ) {}

  /** Builds an update_policy instruction */
  public buildUpdatePolicyIx(
    params: UpdatePolicyParams
  ): TransactionInstruction {
    const [policyPda] = findPolicyPda(params.vault, this.programId);

    // Args: max_ltv(2) + max_pos(2) + stop_loss(2) + take_profit(2) + rebalance(2) + is_active(1)
    const data = Buffer.alloc(8 + 2 + 2 + 2 + 2 + 2 + 1);
    let offset = 0;
    UPDATE_POLICY_DISCRIMINATOR.copy(data, offset);
    offset += 8;

    data.writeUInt16LE(params.maxLtvBps, offset);
    offset += 2;
    data.writeUInt16LE(params.maxPositionBps, offset);
    offset += 2;
    data.writeUInt16LE(params.stopLossBps, offset);
    offset += 2;
    data.writeUInt16LE(params.takeProfitBps, offset);
    offset += 2;
    data.writeUInt16LE(params.rebalanceThresholdBps, offset);
    offset += 2;
    data.writeUInt8(params.isActive ? 1 : 0, offset);

    return new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: params.authority, isSigner: true, isWritable: true },
        { pubkey: params.vault, isSigner: false, isWritable: false },
        { pubkey: policyPda, isSigner: false, isWritable: true },
      ],
      data,
    });
  }

  /** Assembles a ready-to-sign update policy transaction */
  public async createUpdatePolicyTx(
    params: UpdatePolicyParams
  ): Promise<Transaction> {
    const ix = this.buildUpdatePolicyIx(params);
    const tx = new Transaction().add(ix);
    tx.feePayer = params.authority;
    return tx;
  }

  /** Fetches vault risk policy from the API backend */
  public async getPolicyFromApi(vaultAddress: string): Promise<PolicyModel | null> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }
    const resp = await fetch(`${this.apiUrl}/vaults/${vaultAddress}/policy`);
    if (resp.status === 404) return null;
    if (!resp.ok) throw new Error(`Failed to fetch policy: ${resp.statusText}`);
    const data = await resp.json();
    return data as PolicyModel;
  }
}
