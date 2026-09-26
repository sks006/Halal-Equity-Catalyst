import { createAsyncThunk, createSlice, PayloadAction } from "@reduxjs/toolkit";
import { VaultModel } from "@equity-catalyst/sdk";
import { getSdkClient } from "../lib/sdk";

export interface VaultsState {
  items: VaultModel[];
  selectedVault: VaultModel | null;
  isLoading: boolean;
  error: string | null;
}

const initialState: VaultsState = {
  items: [],
  selectedVault: null,
  isLoading: false,
  error: null,
};

/**
 * Asynchronous thunk to fetch all vaults from API backend (fail-closed on error)
 */
export const fetchVaults = createAsyncThunk(
  "vaults/fetchVaults",
  async (_, { rejectWithValue }) => {
    try {
      const sdk = getSdkClient();
      if (!sdk.apiUrl) {
        return rejectWithValue("API URL not configured");
      }
      const res = await fetch(`${sdk.apiUrl}/vaults`, {
        signal: AbortSignal.timeout(5000),
      });
      if (!res.ok) {
        return rejectWithValue(`Failed to fetch vaults: HTTP ${res.status}`);
      }
      const data = await res.json();
      if (Array.isArray(data)) {
        return data as VaultModel[];
      }
      return rejectWithValue("Invalid response format: expected array of vaults");
    } catch (err: any) {
      return rejectWithValue(err?.message || "Failed to fetch vaults");
    }
  }
);

/**
 * Asynchronous thunk to fetch a single vault record by address
 */
export const fetchVaultByAddress = createAsyncThunk(
  "vaults/fetchVaultByAddress",
  async (vaultAddress: string, { rejectWithValue }) => {
    try {
      const sdk = getSdkClient();
      if (!sdk.apiUrl) {
        return rejectWithValue("API URL not configured");
      }
      const vault = await sdk.vaults.getVaultFromApi(vaultAddress);
      if (vault) return vault;
      return rejectWithValue(`Vault ${vaultAddress} not found`);
    } catch (err: any) {
      return rejectWithValue(err?.message || "Failed to fetch vault");
    }
  }
);

export const vaultsSlice = createSlice({
  name: "vaults",
  initialState,
  reducers: {
    setVaults: (state, action: PayloadAction<VaultModel[]>) => {
      state.items = action.payload;
    },
    setSelectedVault: (state, action: PayloadAction<VaultModel | null>) => {
      state.selectedVault = action.payload;
    },
    setVaultPausedOptimistic: (
      state,
      action: PayloadAction<{ vaultAddress: string; isPaused: boolean }>
    ) => {
      const { vaultAddress, isPaused } = action.payload;
      const vault = state.items.find((v) => v.vault_address === vaultAddress);
      if (vault) {
        vault.is_paused = isPaused;
      }
      if (state.selectedVault && state.selectedVault.vault_address === vaultAddress) {
        state.selectedVault.is_paused = isPaused;
      }
    },
    updateVaultDepositsOptimistic: (
      state,
      action: PayloadAction<{ vaultAddress: string; depositDelta: number; sharesDelta: number }>
    ) => {
      const { vaultAddress, depositDelta, sharesDelta } = action.payload;
      const vault = state.items.find((v) => v.vault_address === vaultAddress);
      if (vault) {
        vault.total_deposits = Math.max(0, vault.total_deposits + depositDelta);
        vault.total_shares = Math.max(0, vault.total_shares + sharesDelta);
      }
      if (state.selectedVault && state.selectedVault.vault_address === vaultAddress) {
        state.selectedVault.total_deposits = Math.max(0, state.selectedVault.total_deposits + depositDelta);
        state.selectedVault.total_shares = Math.max(0, state.selectedVault.total_shares + sharesDelta);
      }
    },
    addVault: (state, action: PayloadAction<VaultModel>) => {
      state.items.unshift(action.payload);
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchVaults.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(fetchVaults.fulfilled, (state, action) => {
        state.isLoading = false;
        state.items = action.payload;
      })
      .addCase(fetchVaults.rejected, (state, action) => {
        state.isLoading = false;
        state.error = (action.payload as string) || action.error.message || "Failed to fetch vaults";
      })
      .addCase(fetchVaultByAddress.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(fetchVaultByAddress.fulfilled, (state, action) => {
        state.isLoading = false;
        state.selectedVault = action.payload;
      })
      .addCase(fetchVaultByAddress.rejected, (state, action) => {
        state.isLoading = false;
        state.error = (action.payload as string) || action.error.message || "Failed to fetch vault";
      });
  },
});

export const {
  setVaults,
  setSelectedVault,
  setVaultPausedOptimistic,
  updateVaultDepositsOptimistic,
  addVault,
} = vaultsSlice.actions;

export default vaultsSlice.reducer;
