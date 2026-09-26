import { createAsyncThunk, createSlice, PayloadAction } from "@reduxjs/toolkit";
import { PortfolioModel } from "@equity-catalyst/sdk";
import { getSdkClient } from "../lib/sdk";

export interface PortfolioState {
  positions: PortfolioModel[];
  isLoading: boolean;
  isSyncingPrices: boolean;
  error: string | null;
}

const initialState: PortfolioState = {
  positions: [],
  isLoading: false,
  isSyncingPrices: false,
  error: null,
};

/**
 * Asynchronous thunk to fetch portfolio allocations for a vault (fail-closed on error)
 */
export const fetchPortfolioByVault = createAsyncThunk(
  "portfolio/fetchPortfolioByVault",
  async (vaultAddress: string, { rejectWithValue }) => {
    try {
      const sdk = getSdkClient();
      if (!sdk.apiUrl) {
        return rejectWithValue("API URL not configured");
      }
      const positions = await sdk.portfolio.getPortfolioByVault(vaultAddress);
      if (Array.isArray(positions)) {
        return positions;
      }
      return rejectWithValue("Invalid portfolio response");
    } catch (err: any) {
      return rejectWithValue(err?.message || "Failed to fetch portfolio");
    }
  }
);

/**
 * Asynchronous thunk to update asset valuations using normalized Pyth oracle feeds
 */
export const syncPortfolioPrices = createAsyncThunk(
  "portfolio/syncPortfolioPrices",
  async (vaultAddress: string, { getState, rejectWithValue }) => {
    try {
      const sdk = getSdkClient();
      const state = getState() as { portfolio: PortfolioState };
      const currentPositions = state.portfolio.positions;

      if (sdk.apiUrl && currentPositions.length > 0) {
        const updated = await Promise.all(
          currentPositions.map(async (pos) => {
            try {
              const oraclePrice = await sdk.getOraclePrice(pos.asset_symbol);
              return {
                ...pos,
                current_price_usd: oraclePrice.price_usd,
                current_value_usd: pos.amount * oraclePrice.price_usd,
              };
            } catch {
              return pos;
            }
          })
        );
        return updated;
      }
      return currentPositions;
    } catch (err: any) {
      return rejectWithValue(err?.message || "Failed to sync Pyth prices");
    }
  }
);

export const portfolioSlice = createSlice({
  name: "portfolio",
  initialState,
  reducers: {
    setPositions: (state, action: PayloadAction<PortfolioModel[]>) => {
      state.positions = action.payload;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchPortfolioByVault.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(fetchPortfolioByVault.fulfilled, (state, action) => {
        state.isLoading = false;
        state.positions = action.payload;
      })
      .addCase(fetchPortfolioByVault.rejected, (state, action) => {
        state.isLoading = false;
        state.error = (action.payload as string) || action.error.message || "Failed to fetch portfolio";
      })
      .addCase(syncPortfolioPrices.pending, (state) => {
        state.isSyncingPrices = true;
      })
      .addCase(syncPortfolioPrices.fulfilled, (state, action) => {
        state.isSyncingPrices = false;
        state.positions = action.payload;
      })
      .addCase(syncPortfolioPrices.rejected, (state, action) => {
        state.isSyncingPrices = false;
        state.error = action.payload as string;
      });
  },
});

export const { setPositions } = portfolioSlice.actions;

export default portfolioSlice.reducer;
