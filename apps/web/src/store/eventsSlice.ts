import { createAsyncThunk, createSlice, PayloadAction } from "@reduxjs/toolkit";
import { EventModel } from "@equity-catalyst/sdk";
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

const initialState: EventsState = {
  events: [],
  selectedEventId: "",
  activeStage: 6, // Default to full completed timeline
  isPlaying: false,
  isLoading: false,
  filterType: "ALL",
  filterStatus: "ALL",
  pipelines: {},
};

export const fetchEventsByVault = createAsyncThunk(
  "events/fetchEventsByVault",
  async (vaultAddress: string, { rejectWithValue }) => {
    try {
      const sdk = getSdkClient();
      if (!sdk.apiUrl) {
        return rejectWithValue("API URL not configured");
      }
      const events = await sdk.events.listEventsByVault(vaultAddress);
      if (Array.isArray(events)) {
        return events;
      }
      return rejectWithValue("Invalid events response");
    } catch (err: any) {
      return rejectWithValue(err?.message || "Failed to fetch events");
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
    setPipeline: (state, action: PayloadAction<DetailedEventPipeline>) => {
      state.pipelines[action.payload.eventId] = action.payload;
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
        if (action.payload.length > 0 && !state.selectedEventId) {
          state.selectedEventId = action.payload[0].event_id;
        }
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
  setPipeline,
} = eventsSlice.actions;

export default eventsSlice.reducer;
