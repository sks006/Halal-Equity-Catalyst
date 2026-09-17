"use client";

import { useEffect, useState, useRef } from "react";
import {
  EquityPythLazerService,
  type LazerPriceUpdate,
  type LazerConnectionStatus,
  KNOWN_FEED_SYMBOLS,
} from "@equity-catalyst/sdk";

export interface UsePythLazerOptions {
  token?: string;
  feedIds?: number[];
  channel?: "real_time" | "fixed_rate@50ms" | "fixed_rate@200ms" | "fixed_rate@1000ms";
}

export function usePythLazer(options: UsePythLazerOptions = {}) {
  const [status, setStatus] = useState<LazerConnectionStatus>("disconnected");
  const [prices, setPrices] = useState<Record<number, LazerPriceUpdate>>({});
  const serviceRef = useRef<EquityPythLazerService | null>(null);

  const token =
    options.token ||
    (typeof process !== "undefined"
      ? process.env.NEXT_PUBLIC_LAZER_TOKEN || process.env.LAZER_TOKEN
      : undefined);

  const feedIds = options.feedIds || [1, 2, 6, 10, 11, 12];
  const channel = options.channel || "fixed_rate@200ms";

  useEffect(() => {
    if (typeof window === "undefined") return;

    const service = new EquityPythLazerService({
      token,
      feedIds,
      channel,
      feedSymbols: KNOWN_FEED_SYMBOLS,
    });
    serviceRef.current = service;

    const unsubStatus = service.onStatusChange((newStatus) => {
      setStatus(newStatus);
    });

    const unsubPrices = service.onPriceUpdate((update) => {
      setPrices((prev) => ({
        ...prev,
        [update.priceFeedId]: update,
      }));
    });

    service.start().catch((err) => {
      console.warn("[usePythLazer] WebSocket pool start notice:", err?.message || err);
    });

    return () => {
      unsubStatus();
      unsubPrices();
      service.shutdown();
      serviceRef.current = null;
    };
  }, [token, JSON.stringify(feedIds), channel]);

  return {
    status,
    prices,
    isConnected: status === "connected",
    isStreaming: status === "connected" && Object.keys(prices).length > 0,
  };
}
