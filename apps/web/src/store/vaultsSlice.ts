import { createAsyncThunk, createSlice, PayloadAction } from "@reduxjs/toolkit";
import { VaultModel } from "@equity-catalyst/sdk";
import { DEMO_VAULTS, getSdkClient } from "../lib/sdk";

export interface VaultsState {
  items: VaultModel[];
  selectedVault: VaultModel | null;
  isLoading: boolean;
  error: string | null;
}

const initialState: VaultsState = {
  items: DEMO_VAULTS,
  selectedVault: null,
  isLoading: false,
  error: null,
};

/**
 * Asynchronous thunk to fetch all vaults from API backend with fallback
 */
export const fetchVaults = createAsyncThunk(
  "vaults/fetchVaults",
  async (_, { rejectWithValue }) => {
    try {
      const sdk = getSdkClient();
      if (sdk.apiUrl) {
        const res = await fetch(`${sdk.apiUrl}/vaults`, {
          signal: AbortSignal.timeout(3000),
        });
        if (res.ok) {
          const data = await res.json();
          if (Array.isArray(data) && data.length > 0) {
            return data as VaultModel[];
          }
        }
      }
      return DEMO_VAULTS;
    } catch (err: any) {
      console.warn("Redux fetchVaults falling back to demo fixtures:", err);
      return DEMO_VAULTS;
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
      if (sdk.apiUrl) {
        const vault = await sdk.vaults.getVaultFromApi(vaultAddress);
        if (vault) return vault;
      }
      const match = DEMO_VAULTS.find((v) => v.vault_address === vaultAddress);
      return match || {
        ...DEMO_VAULTS[0],
        vault_address: vaultAddress,
        name: "Custom Strategy Vault",
      };
    } catch (err: any) {
      const match = DEMO_VAULTS.find((v) => v.vault_address === vaultAddress);
      return match || {
        ...DEMO_VAULTS[0],
        vault_address: vaultAddress,
        name: "Custom Strategy Vault",
      };
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
        state.error = action.error.message || "Failed to fetch vaults";
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
        state.error = action.error.message || "Failed to fetch vault";
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
