import { createAsyncThunk, createSlice, PayloadAction } from "@reduxjs/toolkit";
import { EventModel, ExecutionModel } from "@equity-catalyst/sdk";
import { getSdkClient } from "../lib/sdk";

export interface PipelineStageData {
  stage: number;
  title: string;
  subtitle: string;
  status: "pending" | "processing" | "completed" | "warning";
  timestamp: string;
  metrics: Record<string, any>;
  description: string;
}

export interface DetailedEventPipeline {
  eventId: string;
  vaultAddress: string;
  stages: PipelineStageData[];
}

export interface EventsState {
  events: EventModel[];
  selectedEventId: string;
  activeStage: number; // 1 to 6
  isPlaying: boolean;
  isLoading: boolean;
  filterType: string;
  filterStatus: string;
  pipelines: Record<string, DetailedEventPipeline>;
}

export const DEMO_EVENTS: EventModel[] = [
  {
    event_id: "a8b1c2d3-e4f5-4678-90ab-cdef12345678",
    vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    event_type: "EARNINGS_BEAT",
    source: "bloomberg_terminal",
    sentiment_score: 0.85,
    payload: {
      symbol: "NVDA",
      company_name: "NVIDIA Corporation",
      quarter: "Q2 FY2027",
      headline: "NVIDIA Reports Record Q2 Revenue of $30.04B, Up 122% YoY on Surging Hopper & Blackwell AI Demand",
      eps_actual: 0.68,
      eps_estimate: 0.64,
      eps_surprise_pct: "+6.25%",
      revenue_actual_usd: 30040000000,
      revenue_estimate_usd: 28700000000,
      guidance_q3_revenue_usd: 32500000000,
    },
    status: "CONFIRMED",
    detected_at: "2026-09-14T14:30:00Z",
    processed_at: "2026-09-14T14:30:01Z",
  },
  {
    event_id: "b7c2d3e4-f5a6-4789-91bc-def012345679",
    vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    event_type: "DRIFT_REBALANCE",
    source: "pyth_oracle_stream",
    sentiment_score: 0.2,
    payload: {
      symbol: "SOL",
      feed_id: "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d",
      current_price_usd: 152.2,
      drift_bps: 185,
      threshold_bps: 150,
      headline: "SOL Price Momentum Drift Exceeds Policy Rebalance Threshold (+1.85%)",
    },
    status: "CONFIRMED",
    detected_at: "2026-09-14T12:15:20Z",
    processed_at: "2026-09-14T12:15:21Z",
  },
  {
    event_id: "c6d3e4f5-a7b8-4890-92cd-ef1234567890",
    vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    event_type: "DEPOSIT_DETECTED",
    source: "solana_websocket",
    sentiment_score: null,
    payload: {
      user: "7Xw1e...v8K9",
      amount_usd: 150000,
      shares_minted: 136986,
      tx_signature: "3nKp9...7Vx2",
      headline: "Institutional LP Deposited 150,000 USDC into Vault",
    },
    status: "CONFIRMED",
    detected_at: "2026-09-14T10:45:00Z",
    processed_at: "2026-09-14T10:45:01Z",
  },
  {
    event_id: "d5e4f5a6-b8c9-4901-93de-f23456789012",
    vault_address: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
    event_type: "NEWS_VOLATILITY",
    source: "sec_edgar_rss",
    sentiment_score: -0.15,
    payload: {
      symbol: "JUP",
      headline: "SEC Regulatory Notice Regarding Decentralized Synthetics Framework",
      severity: "LOW",
      action_taken: "NOOP_GUARDED",
    },
    status: "PROCESSED",
    detected_at: "2026-09-14T08:20:10Z",
    processed_at: "2026-09-14T08:20:11Z",
  },
];

export const NVDA_PIPELINE: DetailedEventPipeline = {
  eventId: "a8b1c2d3-e4f5-4678-90ab-cdef12345678",
  vaultAddress: "EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
  stages: [
    {
      stage: 1,
      title: "Event detected",
      subtitle: "High-Confidence External News Ingested",
      status: "completed",
      timestamp: "14:30:00.124 UTC",
      description: "Real-time market earnings report ingested from Bloomberg Terminal data stream with verified sentiment analysis.",
      metrics: {
        Source: "Bloomberg Terminal / SEC EDGAR",
        Symbol: "NVDA",
        "Event Type": "EARNINGS_BEAT",
        "EPS Actual": "$0.68 vs $0.64 est (+6.25%)",
        Revenue: "$30.04B (+4.67%)",
        "Sentiment Score": "+0.85 (Bullish)",
        Confidence: "98.4%",
      },
    },
    {
      stage: 2,
      title: "Policy evaluated",
      subtitle: "Rule Matching & Target Allocation Rebalance",
      status: "completed",
      timestamp: "14:30:00.280 UTC",
      description: "PolicyEngine matched PolicyRule::EarningsBeat, generated a Bullish domain signal, and computed target allocation drift.",
      metrics: {
        "Matched Rule": "PolicyRule::EarningsBeat",
        "Signal Type": "BULLISH",
        "Weight Delta": "+5.00% (+500 bps)",
        "Previous Target": "16.00% (1,600 bps)",
        "New Target": "21.00% (2,100 bps)",
        "Proposed Trade": "BUY $242,500 NVDA via USDC",
        "Normalized Sum": "10,000 bps (100.00%)",
      },
    },
    {
      stage: 3,
      title: "Risk evaluated",
      subtitle: "Multi-Factor Defensive Risk Shield Verification",
      status: "completed",
      timestamp: "14:30:00.345 UTC",
      description: "RiskEngine evaluated proposed trade across four independent defense limits: Position Exposure, Trade Drift, LTV, and Stop-Loss.",
      metrics: {
        "1. Position Exposure": "21.41% <= 25.00% Max Limit (PASSED)",
        "2. Max Trade Drift": "5.00% <= 10.00% Single Limit (PASSED)",
        "3. Vault LTV / Debt": "0.00% <= 65.00% Max LTV (PASSED)",
        "4. Stop-Loss Drawdown": "+8.35% > -8.00% Stop Trigger (PASSED)",
        "Risk Verdict": "RiskAssessment::Approved ✓",
      },
    },
    {
      stage: 4,
      title: "Decision generated",
      subtitle: "Cryptographic Authorization & Order Synthesis",
      status: "completed",
      timestamp: "14:30:00.410 UTC",
      description: "DecisionEngine synthesized atomic ExecutionRequest, verified authority preflight permissions, and prepared Jupiter DEX routing.",
      metrics: {
        "Decision ID": "dec-9011e2f4-8a71-46e3-b1d2-09cba5678912",
        Action: "BUY / REBALANCE",
        Orders: "1 Order: BUY $242,500 NVDA",
        "Pre-Flight Check": "PASSED (No Halt, Active Policy)",
        "Authority Signer": "auth99X...b3N4 Verified",
      },
    },
    {
      stage: 5,
      title: "Execution submitted",
      subtitle: "Jupiter v6 DEX Route & Anchor Instruction",
      status: "completed",
      timestamp: "14:30:00.612 UTC",
      description: "ExecutionService fetched optimal Jupiter swap quote, verified price impact against policy tolerance, and built execute_action instruction.",
      metrics: {
        "Input Amount": "242,500 USDC ($242,500.00)",
        "Expected Output": "1,888.6293 NVDA (@ $128.40)",
        "Price Impact": "4 bps (0.04% <= 150 bps allowed)",
        Route: "USDC -> DLMM Pool -> Wrapped SOL -> NVDA",
        "Anchor Program": "equity_vault (execute_action)",
        "Execution PDA": "Exec91X...91M6",
      },
    },
    {
      stage: 6,
      title: "Transaction confirmed",
      subtitle: "Solana Devnet Block Finality Reached",
      status: "completed",
      timestamp: "14:30:01.030 UTC",
      description: "Solana cluster confirmed transaction with finalized commitment in 418ms. Vault portfolio state synchronized.",
      metrics: {
        Cluster: "Solana Devnet",
        Status: "Finalized",
        "Tx Signature": "4ZpT7VyJvM9Q1bC6w8nL2p4k8R1e3S5x7Z9c1V2b3N4EQTYv7cK89Wq3yK9u4J2b8j9Q1M6z9Y7w9X8c1V2b3N4",
        "Latency": "418 ms",
        "New Allocation": "21.41% NVDA ($1,038,580)",
        "NAV Synchronized": "$4,850,000 USD",
      },
    },
  ],
};

const initialState: EventsState = {
  events: DEMO_EVENTS,
  selectedEventId: DEMO_EVENTS[0].event_id,
  activeStage: 6, // Default to full completed timeline
  isPlaying: false,
  isLoading: false,
  filterType: "ALL",
  filterStatus: "ALL",
  pipelines: {
    [DEMO_EVENTS[0].event_id]: NVDA_PIPELINE,
  },
};

export const fetchEventsByVault = createAsyncThunk(
  "events/fetchEventsByVault",
  async (vaultAddress: string) => {
    try {
      const sdk = getSdkClient();
      if (sdk.apiUrl) {
        const events = await sdk.events.listEventsByVault(vaultAddress);
        if (Array.isArray(events) && events.length > 0) {
          return events;
        }
      }
      return DEMO_EVENTS;
    } catch {
      return DEMO_EVENTS;
    }
  }
);

export const eventsSlice = createSlice({
  name: "events",
  initialState,
  reducers: {
    setSelectedEvent: (state, action: PayloadAction<string>) => {
      state.selectedEventId = action.payload;
      state.activeStage = 6;
      state.isPlaying = false;
    },
    setActiveStage: (state, action: PayloadAction<number>) => {
      state.activeStage = Math.max(1, Math.min(6, action.payload));
    },
    nextStage: (state) => {
      if (state.activeStage < 6) {
        state.activeStage += 1;
      } else {
        state.isPlaying = false;
      }
    },
    prevStage: (state) => {
      if (state.activeStage > 1) {
        state.activeStage -= 1;
      }
    },
    setIsPlaying: (state, action: PayloadAction<boolean>) => {
      state.isPlaying = action.payload;
    },
    startReplay: (state) => {
      state.activeStage = 1;
      state.isPlaying = true;
    },
    setFilterType: (state, action: PayloadAction<string>) => {
      state.filterType = action.payload;
    },
    setFilterStatus: (state, action: PayloadAction<string>) => {
      state.filterStatus = action.payload;
    },
    addEvent: (state, action: PayloadAction<EventModel>) => {
      state.events.unshift(action.payload);
      state.selectedEventId = action.payload.event_id;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchEventsByVault.pending, (state) => {
        state.isLoading = true;
      })
      .addCase(fetchEventsByVault.fulfilled, (state, action) => {
        state.isLoading = false;
        state.events = action.payload;
      })
      .addCase(fetchEventsByVault.rejected, (state) => {
        state.isLoading = false;
      });
  },
});

export const {
  setSelectedEvent,
  setActiveStage,
  nextStage,
  prevStage,
  setIsPlaying,
  startReplay,
  setFilterType,
  setFilterStatus,
  addEvent,
} = eventsSlice.actions;

export default eventsSlice.reducer;
