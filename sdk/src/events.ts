import { Connection, PublicKey } from "@solana/web3.js";
import { DEFAULT_PROGRAM_ID, EventModel } from "./types";

export interface ParsedEvent<T = Record<string, unknown>> {
  name: string;
  data: T;
  rawLog: string;
}

/**
 * Events sub-client for querying system events and parsing Anchor event logs.
 */
export class EventsClient {
  constructor(
    private readonly connection: Connection,
    private readonly programId: PublicKey = DEFAULT_PROGRAM_ID,
    private readonly apiUrl?: string
  ) {}

  /** Queries historical events for a specific vault from the API */
  public async listEventsByVault(vaultAddress: string): Promise<EventModel[]> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }
    const resp = await fetch(`${this.apiUrl}/vaults/${vaultAddress}/events`);
    if (!resp.ok) throw new Error(`Failed to list events: ${resp.statusText}`);
    const data = await resp.json();
    return data as EventModel[];
  }

  /** Queries pending unprocessed events from the API */
  public async getPendingEvents(): Promise<EventModel[]> {
    if (!this.apiUrl) {
      throw new Error("API URL is not configured in SDK client");
    }
    const resp = await fetch(`${this.apiUrl}/events/pending`);
    if (!resp.ok) throw new Error(`Failed to get pending events: ${resp.statusText}`);
    const data = await resp.json();
    return data as EventModel[];
  }

  /**
   * Decodes Anchor program logs emitted via `emit!` or `msg!`
   */
  public parseLogs(logs: string[]): ParsedEvent[] {
    const parsed: ParsedEvent[] = [];

    for (const log of logs) {
      if (log.startsWith("Program data: ")) {
        const base64Data = log.replace("Program data: ", "").trim();
        try {
          const buffer = Buffer.from(base64Data, "base64");
          // 8-byte discriminator followed by serialized payload
          parsed.push({
            name: "AnchorProgramData",
            data: { rawBase64: base64Data, bytesLength: buffer.length },
            rawLog: log,
          });
        } catch {
          // Ignore invalid base64
        }
      } else if (log.includes("Instruction: InitializeVault")) {
        parsed.push({ name: "InitializeVaultLog", data: {}, rawLog: log });
      } else if (log.includes("Instruction: Deposit")) {
        parsed.push({ name: "DepositLog", data: {}, rawLog: log });
      } else if (log.includes("Instruction: Withdraw")) {
        parsed.push({ name: "WithdrawLog", data: {}, rawLog: log });
      }
    }

    return parsed;
  }
}
