import { configureStore } from "@reduxjs/toolkit";
import vaultsReducer from "./vaultsSlice";
import portfolioReducer from "./portfolioSlice";
import policyReducer from "./policySlice";
import eventsReducer from "./eventsSlice";

export const store = configureStore({
  reducer: {
    vaults: vaultsReducer,
    portfolio: portfolioReducer,
    policy: policyReducer,
    events: eventsReducer,
  },
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware({
      serializableCheck: false,
    }),
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;
