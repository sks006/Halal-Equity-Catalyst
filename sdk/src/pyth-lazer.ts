import {
  PythLazerClient,
  type JsonOrBinaryResponse,
  type ParsedFeedPayload,
} from "@pythnetwork/pyth-lazer-sdk";

export { PythLazerClient };

/** Default WebSocket streaming endpoints for Pyth Lazer */
export const DEFAULT_LAZER_STREAM_URLS = [
  "wss://pyth-lazer-0.dourolabs.app/v1/stream",
  "wss://pyth-lazer-1.dourolabs.app/v1/stream",
  "wss://pyth-lazer-2.dourolabs.app/v1/stream",
];

/** Normalized Lazer price update model */
export interface LazerPriceUpdate {
  priceFeedId: number;
  symbol?: string;
  price: number;
  priceRaw: string;
  exponent: number;
  confidence?: number;
  timestampUs: string;
  solanaBinaryHex?: string;
}

export type LazerConnectionStatus = "disconnected" | "connecting" | "connected" | "reconnecting" | "down";

export interface LazerClientOptions {
  token?: string;
  streamUrls?: string[];
  feedIds?: number[];
  feedSymbols?: Record<number, string>;
  channel?: "real_time" | "fixed_rate@50ms" | "fixed_rate@200ms" | "fixed_rate@1000ms";
}

/** Known mapping of Pyth Lazer Feed IDs for reference */
export const KNOWN_FEED_SYMBOLS: Record<number, string> = {
  1: "BTC/USD",
  2: "ETH/USD",
  6: "SOL/USD",
  10: "AAPL/USD",
  11: "MSFT/USD",
  12: "GOOGL/USD",
};

/**
 * Pyth Lazer streaming service managing low-latency price feeds for Equity Catalyst.
 */
export class EquityPythLazerService {
  private client: PythLazerClient | null = null;
  private status: LazerConnectionStatus = "disconnected";
  private readonly token: string;
  private readonly streamUrls: string[];
  private readonly feedIds: number[];
  private readonly feedSymbols: Record<number, string>;
  private readonly channel: "real_time" | "fixed_rate@50ms" | "fixed_rate@200ms" | "fixed_rate@1000ms";

  private latestPrices = new Map<number, LazerPriceUpdate>();
  private updateListeners = new Set<(update: LazerPriceUpdate) => void>();
  private statusListeners = new Set<(status: LazerConnectionStatus) => void>();

  constructor(options: LazerClientOptions = {}) {
    this.token = options.token || (typeof process !== "undefined" ? process.env?.LAZER_TOKEN || "" : "");
    this.streamUrls = options.streamUrls || DEFAULT_LAZER_STREAM_URLS;
    this.feedIds = options.feedIds || [1, 2];
    this.feedSymbols = { ...KNOWN_FEED_SYMBOLS, ...(options.feedSymbols || {}) };
    this.channel = options.channel || "fixed_rate@200ms";
  }

  public getStatus(): LazerConnectionStatus {
    return this.status;
  }

  public getLatestPrice(feedId: number): LazerPriceUpdate | undefined {
    return this.latestPrices.get(feedId);
  }

  public getAllLatestPrices(): Map<number, LazerPriceUpdate> {
    return new Map(this.latestPrices);
  }

  public onPriceUpdate(callback: (update: LazerPriceUpdate) => void): () => void {
    this.updateListeners.add(callback);
    return () => this.updateListeners.delete(callback);
  }

  public onStatusChange(callback: (status: LazerConnectionStatus) => void): () => void {
    this.statusListeners.add(callback);
    return () => this.statusListeners.delete(callback);
  }

  private setStatus(status: LazerConnectionStatus) {
    this.status = status;
    this.statusListeners.forEach((listener) => {
      try {
        listener(status);
      } catch (err) {
        console.error("Error in status listener:", err);
      }
    });
  }

  /**
   * Initializes WebSocket pool connection and subscribes to price feeds.
   */
  public async start(): Promise<void> {
    if (this.client) {
      return;
    }

    if (!this.token) {
      console.warn("[PythLazer] No token provided. Set LAZER_TOKEN or pass token in options.");
      this.setStatus("down");
      return;
    }

    this.setStatus("connecting");

    try {
      this.client = await PythLazerClient.create({
        token: this.token,
        webSocketPoolConfig: {
          urls: this.streamUrls,
        },
      });

      this.setStatus("connected");

      this.client.addAllConnectionsDownListener(() => {
        this.setStatus("down");
      });

      this.client.addConnectionRestoredListener(() => {
        this.setStatus("connected");
      });

      this.client.addConnectionReconnectListener(() => {
        this.setStatus("reconnecting");
      });

      this.client.addMessageListener((event: JsonOrBinaryResponse) => {
        this.handleMessage(event);
      });

      this.client.subscribe({
        type: "subscribe",
        subscriptionId: 1,
        priceFeedIds: this.feedIds,
        properties: ["price", "exponent", "confidence"],
        formats: ["solana"],
        channel: this.channel,
        deliveryFormat: "json",
        jsonBinaryEncoding: "hex",
        parsed: true,
      });
    } catch (err) {
      this.setStatus("down");
      console.error("[PythLazer] Failed to start Lazer client:", err);
      throw err;
    }
  }

  /**
   * Disconnects and shuts down client connection pool.
   */
  public shutdown(): void {
    if (this.client) {
      try {
        this.client.shutdown();
      } catch (err) {
        console.error("[PythLazer] Error during shutdown:", err);
      }
      this.client = null;
    }
    this.setStatus("disconnected");
  }

  private handleMessage(event: JsonOrBinaryResponse): void {
    let priceFeeds: ParsedFeedPayload[] = [];
    let timestampUs = Date.now().toString() + "000";
    let solanaHex: string | undefined;

    if (event.type === "json") {
      if (event.value.type === "streamUpdated") {
        if (event.value.parsed?.priceFeeds) {
          priceFeeds = event.value.parsed.priceFeeds;
          timestampUs = event.value.parsed.timestampUs;
        }
        if (event.value.solana?.data) {
          solanaHex = event.value.solana.data;
        }
      }
    } else if (event.type === "binary") {
      if (event.value.parsed?.priceFeeds) {
        priceFeeds = event.value.parsed.priceFeeds;
        timestampUs = event.value.parsed.timestampUs;
      }
      if (event.value.solana) {
        solanaHex = event.value.solana.toString("hex");
      }
    }

    for (const feed of priceFeeds) {
      if (!feed.price) continue;

      const expo = feed.exponent ?? -8;
      const rawPrice = BigInt(feed.price);
      // Normalized floating price
      const priceFloat = Number(rawPrice) * Math.pow(10, expo);

      const update: LazerPriceUpdate = {
        priceFeedId: feed.priceFeedId,
        symbol: this.feedSymbols[feed.priceFeedId] || `FEED-#${feed.priceFeedId}`,
        price: priceFloat,
        priceRaw: feed.price,
        exponent: expo,
        confidence: feed.confidence,
        timestampUs,
        solanaBinaryHex: solanaHex,
      };

      this.latestPrices.set(feed.priceFeedId, update);

      this.updateListeners.forEach((listener) => {
        try {
          listener(update);
        } catch (listenerErr) {
          console.error("[PythLazer] Error in price update listener:", listenerErr);
        }
      });
    }
  }
}
