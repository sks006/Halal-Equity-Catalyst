import { EquityCatalystClient } from "@equity-catalyst/sdk";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:4000";
const RPC_URL = process.env.NEXT_PUBLIC_SOLANA_RPC_URL || "https://api.devnet.solana.com";

let clientInstance: EquityCatalystClient | null = null;

export function getSdkClient(): EquityCatalystClient {
  if (!clientInstance) {
    clientInstance = new EquityCatalystClient({
      apiUrl: API_URL,
      rpcUrl: RPC_URL,
    });
  }
  return clientInstance;
}
