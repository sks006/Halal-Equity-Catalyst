import { Connection, PublicKey } from "@solana/web3.js";
import { DEFAULT_PROGRAM_ID, PortfolioModel } from "./types";

/**
 * Derives Position PDA: `[b"position", vault, assetMint]`
 */
export function findPositionPda(
  vault: PublicKey,
  assetMint: PublicKey,
  programId: PublicKey = DEFAULT_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position"), vault.toBuffer(), assetMint.toBuffer()],
    programId
  );
}

/**
 * Portfolio sub-client for asset weight analysis, position tracking, and valuation sync.
 */
export class PortfolioClient {
  constructor(
    private readonly connection: Connection,
    private readonly programId: PublicKey = DEFAULT_PROGRAM_ID,
    private readonly apiUrl?: string
  ) {}

  /**
   * Fetches the current portfolio positions and weights for a vault from the API
   */
  public async getPortfolioByVault(vaultAddress: string): Promise<PortfolioModel[]> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }

    const resp = await fetch(`${this.apiUrl}/vaults/${vaultAddress}/portfolio`);
    if (!resp.ok) throw new Error(`Failed to get portfolio: ${resp.statusText}`);
    const data = await resp.json();
    return data as PortfolioModel[];
  }

  /**
   * Triggers an off-chain valuation sync updating asset prices and weights via Pyth oracles
   */
  public async syncPortfolioValuation(vaultAddress: string): Promise<PortfolioModel[]> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }

    const resp = await fetch(`${this.apiUrl}/vaults/${vaultAddress}/portfolio/sync`, {
      method: "POST",
    });

    if (!resp.ok) {
      const errText = await resp.text();
      throw new Error(`Valuation sync failed: ${errText}`);
    }

    const data = await resp.json();
    return data as PortfolioModel[];
  }
}
