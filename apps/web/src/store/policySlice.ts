import { createAsyncThunk, createSlice, PayloadAction } from "@reduxjs/toolkit";
import { PolicyModel } from "@equity-catalyst/sdk";
import { DEMO_POLICY, getSdkClient } from "../lib/sdk";

export interface PolicyState {
  currentPolicy: PolicyModel | null;
  isLoading: boolean;
  isSaving: boolean;
  error: string | null;
}

const initialState: PolicyState = {
  currentPolicy: DEMO_POLICY,
  isLoading: false,
  isSaving: false,
  error: null,
};

/**
 * Asynchronous thunk to fetch risk policy for a vault
 */
export const fetchPolicyByVault = createAsyncThunk(
  "policy/fetchPolicyByVault",
  async (vaultAddress: string, { rejectWithValue }) => {
    try {
      const sdk = getSdkClient();
      if (sdk.apiUrl) {
        const policy = await sdk.policies.getPolicyFromApi(vaultAddress);
        if (policy) return policy;
      }
      return { ...DEMO_POLICY, vault_address: vaultAddress };
    } catch (err: any) {
      console.warn("Policy fetch falling back to fixtures:", err);
      return { ...DEMO_POLICY, vault_address: vaultAddress };
    }
  }
);

/**
 * Asynchronous thunk to save policy to backend API
 */
export const savePolicy = createAsyncThunk(
  "policy/savePolicy",
  async (policyData: PolicyModel, { rejectWithValue }) => {
    try {
      const sdk = getSdkClient();
      if (sdk.apiUrl) {
        await fetch(`${sdk.apiUrl}/vaults/${policyData.vault_address}/policy`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            min_cash_bps: policyData.min_cash_bps,
            max_position_bps: policyData.max_position_bps,
            stop_loss_bps: policyData.stop_loss_bps,
            take_profit_bps: policyData.take_profit_bps,
            rebalance_threshold_bps: policyData.rebalance_threshold_bps,
            is_active: policyData.is_active,
          }),
        });
      }
      return policyData;
    } catch (err: any) {
      return policyData; // Return local optimistic policy if API is offline
    }
  }
);

export const policySlice = createSlice({
  name: "policy",
  initialState,
  reducers: {
    setPolicy: (state, action: PayloadAction<PolicyModel | null>) => {
      state.currentPolicy = action.payload;
    },
    updatePolicyLocal: (state, action: PayloadAction<Partial<PolicyModel>>) => {
      if (state.currentPolicy) {
        state.currentPolicy = { ...state.currentPolicy, ...action.payload };
      }
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchPolicyByVault.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(fetchPolicyByVault.fulfilled, (state, action) => {
        state.isLoading = false;
        state.currentPolicy = action.payload;
      })
      .addCase(fetchPolicyByVault.rejected, (state, action) => {
        state.isLoading = false;
        state.error = action.error.message || "Failed to fetch policy";
      })
      .addCase(savePolicy.pending, (state) => {
        state.isSaving = true;
      })
      .addCase(savePolicy.fulfilled, (state, action) => {
        state.isSaving = false;
        state.currentPolicy = action.payload;
      })
      .addCase(savePolicy.rejected, (state) => {
        state.isSaving = false;
      });
  },
});

export const { setPolicy, updatePolicyLocal } = policySlice.actions;

export default policySlice.reducer;
