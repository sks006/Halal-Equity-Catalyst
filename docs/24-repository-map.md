# 24. Repository Map & Directory Layout

## 1. Complete Project File Structure

Below is the exhaustive file structure of the **Equity Catalyst** repository (`https://github.com/sks006/equity-catalyst.git`), mapped to every active source file, module, migration, and integration test.

```text
equity-catalyst/
├── Cargo.toml                                 # Top-level Cargo workspace manifest (resolver = "2")
├── Cargo.lock                                 # Pinned dependencies (Rust 1.75 / Solana platform-tools compatible)
├── Anchor.toml                                # Anchor deployment, cluster, and test configuration
├── package.json                               # Root Node.js dependencies (web3.js, mocha, typescript)
├── pnpm-workspace.yaml                        # PNPM monorepo workspace definition (apps/*, sdk)
├── tsconfig.json                              # Root TypeScript compiler settings
├── .env.example                               # Template environment variables (RPC, DB, Redis)
│
├── apps/                                      # Application packages
│   ├── api/                                   # Axum HTTP backend server & domain engines
│   │   ├── Cargo.toml                         # API dependencies (axum, tokio, deadpool, redis, solana-sdk)
│   │   ├── src/
│   │   │   ├── main.rs                        # Axum server entry point & shutdown handler
│   │   │   ├── lib.rs                         # Library exports for API integration testing
│   │   │   ├── config.rs                      # Environment configuration parser
│   │   │   ├── state.rs                       # Shared AppState (pool, redis, services)
│   │   │   ├── router.rs                      # Route declarations, CORS, and middleware
│   │   │   ├── error.rs                       # ApiError enum & HTTP response conversions
│   │   │   │
│   │   │   ├── engines/                       # Deterministic analytical engines
│   │   │   │   ├── mod.rs                     # Exports PolicyEngine, RiskEngine, DecisionEngine
│   │   │   │   ├── decision_engine/           # Decision synthesis & keeper signing
│   │   │   │   │   ├── mod.rs                 # process_event orchestration pipeline
│   │   │   │   │   ├── decision.rs            # ExecutionRequest & TradeOrder models
│   │   │   │   │   ├── signer.rs              # ExecutionSigner isolated keypair holder
│   │   │   │   │   └── validation.rs          # Pre-flight decision sanity validation
│   │   │   │   ├── policy_engine/             # Rule matching & target allocation
│   │   │   │   │   ├── mod.rs                 # evaluate() entry point
│   │   │   │   │   ├── rules.rs               # match_rule (EarningsBeat, DriftRebalance, etc.)
│   │   │   │   │   ├── signals.rs             # generate_signal mapping rules to SignalType
│   │   │   │   │   ├── allocation.rs          # Target weight adjustment & rebalance planning
│   │   │   │   │   └── conditions.rs          # Condition predicates & sentiment thresholds
│   │   │   │   ├── risk_engine/               # Multi-factor defensive risk validation
│   │   │   │   │   ├── mod.rs                 # evaluate_proposed_trades pipeline
│   │   │   │   │   ├── exposure.rs            # Single-asset concentration exposure checks
│   │   │   │   │   ├── limits.rs              # 10% single-trade cap & slippage bounds
│   │   │   │   │   ├── ltv.rs                 # Loan-to-Value borrowing checks
│   │   │   │   │   └── stops.rs               # Stop-loss & take-profit trigger guards
│   │   │   │   ├── portfolio_engine/          # (Empty directory - logic lives in crates/shared)
│   │   │   │   └── event_engine/              # (Empty directory - logic lives in workers & routes)
│   │   │   │
│   │   │   ├── routes/                        # HTTP route handler functions
│   │   │   │   ├── mod.rs                     # Handler re-exports
│   │   │   │   ├── health.rs                  # GET /health, /health/detailed, /ready
│   │   │   │   ├── vaults.rs                  # GET /vaults, POST /vaults, GET /vaults/:addr
│   │   │   │   ├── policies.rs                # GET /policies, POST /policies
│   │   │   │   ├── events.rs                  # POST /events, GET /events, GET /events/pending
│   │   │   │   ├── quotes.rs                  # POST /quotes/evaluate (Quote-only execution)
│   │   │   │   ├── executions.rs              # GET /executions, GET /vaults/:addr/executions
│   │   │   │   └── oracle.rs                  # GET /oracle/price/:symbol
│   │   │   │
│   │   │   ├── services/                      # Business & integration service coordinators
│   │   │   │   ├── mod.rs                     # Service re-exports
│   │   │   │   ├── quote_service.rs           # QuoteExecutionService integrating Jupiter v6
│   │   │   │   ├── solana_service.rs          # SolanaService managing RPC, WS, and Anchor
│   │   │   │   ├── oracle_service.rs          # OracleService managing normalized Pyth feeds
│   │   │   │   ├── vault_service.rs           # VaultService high-level coordination
│   │   │   │   └── health_monitor.rs          # HealthMonitor background telemetry checks
│   │   │   │
│   │   │   ├── workers/                       # Asynchronous background daemon loops
│   │   │   │   ├── mod.rs                     # Worker re-exports
│   │   │   │   ├── event_listener.rs          # Ingestion queue pusher (Redis / Postgres)
│   │   │   │   └── policy_worker.rs           # Event consumer, dry-run auditor, execution logger
│   │   │   │
│   │   │   ├── models/                        # PostgreSQL relational data models
│   │   │   │   ├── mod.rs                     # Model re-exports
│   │   │   │   ├── vault.rs                   # VaultModel representation
│   │   │   │   ├── policy.rs                  # PolicyModel representation
│   │   │   │   ├── event.rs                   # EventModel representation
│   │   │   │   ├── portfolio.rs               # PortfolioModel representation
│   │   │   │   └── execution.rs               # ExecutionModel representation
│   │   │   │
│   │   │   └── repositories/                  # PostgreSQL data access layer
│   │   │       ├── mod.rs                     # Repository re-exports
│   │   │       ├── vault_repository.rs        # Vault queries & state sync
│   │   │       ├── policy_repository.rs       # Policy queries & updates
│   │   │       ├── event_repository.rs        # Event persistence & pending queries
│   │   │       ├── portfolio_repository.rs    # Position queries & target weight sync
│   │   │       └── execution_repository.rs    # Execution audit trail creation & queries
│   │   └── tests/                             # API Integration tests
│   │       ├── api_endpoints_test.rs          # End-to-end HTTP endpoint tests
│   │       ├── domain_services_test.rs        # Domain services unit tests
│   │       ├── engine_matrix_test.rs          # Engine cross-product matrix tests
│   │       ├── event_processing_test.rs       # Worker event pipeline tests
│   │       ├── full_pipeline_e2e_test.rs      # End-to-end backend integration tests
│   │       ├── health_monitor_test.rs         # Health telemetry tests
│   │       ├── health_test.rs                 # Health endpoint tests
│   │       ├── oracle_service_test.rs         # Pyth normalization tests
│   │       ├── quote_execution_test.rs        # Quote execution sandbox tests
│   │       ├── repository_test.rs             # Database queries tests
│   │       └── solana_service_test.rs         # SolanaService read-only & RPC tests
│   │
│   └── web/                                   # Next.js 14+ App Router Web Application
│       ├── package.json                       # React, Next.js, Solana Wallet Adapter, Redux
│       ├── tsconfig.json                      # Web TypeScript config
│       ├── tailwind.config.ts                 # Tailwind styling & dark mode design tokens
│       ├── postcss.config.js                  # PostCSS plugins
│       ├── next.config.mjs                    # Next.js build configuration
│       └── src/
│           ├── app/                           # App Router pages
│           │   ├── layout.tsx                 # Root layout with WalletProvider & StoreProvider
│           │   ├── page.tsx                   # Marketing landing & protocol overview
│           │   ├── globals.css                # Custom glassmorphism & color design system
│           │   ├── dashboard/page.tsx         # User portfolio overview & aggregate stats
│           │   ├── discover/                  # Vault discovery catalog
│           │   ├── demo/page.tsx              # Interactive visual pipeline demo replay
│           │   ├── portfolio/                 # Global portfolio breakdown
│           │   └── vault/
│           │       ├── new/page.tsx           # Vault deployment & policy creation wizard
│           │       └── [address]/page.tsx     # Single vault deep-dive & deposit/withdraw UI
│           ├── components/                    # Reusable UI components
│           │   ├── Navbar.tsx                 # Navigation bar with wallet status
│           │   ├── WalletProvider.tsx         # Solana wallet adapter context wrapper
│           │   ├── WalletButton.tsx           # Connect wallet button
│           │   ├── StoreProvider.tsx          # Redux Toolkit store wrapper
│           │   ├── TransactionStatus.tsx      # Modal transaction feedback
│           │   └── ui/                        # Design primitives (button, card, input, table, etc.)
│           ├── features/                      # Domain-specific UI features
│           │   ├── vault/                     # VaultCard, RiskMeter
│           │   ├── policy/                    # PolicyBuilder
│           │   ├── portfolio/                 # PositionTable
│           │   └── events/                    # EventFeed, DecisionTimeline
│           ├── hooks/                         # React hooks
│           │   ├── useSolana.ts               # RPC querying hook
│           │   ├── useAnchorProgram.ts        # Anchor program instance hook
│           │   ├── useSolanaTx.ts             # Transaction signing & dispatch hook
│           │   └── useWallet.ts               # Wallet state wrapper
│           ├── store/                         # Redux Toolkit state
│           │   ├── store.ts                   # Store configuration
│           │   ├── vaultsSlice.ts             # Vaults cache
│           │   ├── portfolioSlice.ts          # Positions cache
│           │   ├── eventsSlice.ts             # Events cache
│           │   ├── policySlice.ts             # Policy parameters
│           │   └── hooks.ts                   # Typed Redux hooks
│           └── lib/                           # Web utilities & SDK re-exports
│               ├── sdk.ts                     # Initialized SDK singleton
│               └── idl/equity_vault.json      # Program IDL
│
├── crates/                                    # Internal Rust crates
│   └── shared/                                # Pure deterministic domain logic
│       ├── Cargo.toml                         # Shared crate manifest (serde, thiserror)
│       └── src/
│           ├── lib.rs                         # Crate root & module re-exports
│           ├── types.rs                       # BasisPoints, RiskLimits, SignalType, AssetWeight
│           ├── constants.rs                   # MAX_BPS (10,000), default risk parameters
│           ├── math.rs                        # Pro-rata shares math, basis points conversions
│           ├── risk.rs                        # calculate_ltv, check_position_exposure, stops
│           ├── allocation.rs                  # calculate_rebalance_plan, calculate_drift
│           ├── policy.rs                      # PolicyDefinition, evaluate_event_signal
│           └── validation.rs                  # ValidationError enum, validate_bps
│
├── programs/                                  # Solana Smart Contracts (Anchor)
│   └── equity_vault/                          # Core vault program
│       ├── Cargo.toml                         # Anchor program manifest (anchor-lang, anchor-spl)
│       └── src/
│           ├── lib.rs                         # Program entrypoint & instruction dispatchers
│           ├── constants.rs                   # MAX_NAME_LEN, MAX_SYMBOL_LEN, MAX_BPS
│           ├── errors.rs                      # EquityVaultError enum codes
│           ├── events.rs                      # Anchor event definitions
│           ├── state/                         # On-chain account data structures
│           │   ├── mod.rs                     # Account struct re-exports
│           │   ├── vault.rs                   # Vault account (authority, shares, deposits, paused)
│           │   ├── policy.rs                  # Policy account (max_ltv, max_position, stops)
│           │   ├── user_shares.rs             # UserShares account (shares owned by user)
│           │   ├── position.rs                # Position account (holding of single asset)
│           │   ├── loan.rs                    # Loan account (borrower, collateral, debt)
│           │   └── execution.rs               # Execution account (trade record)
│           ├── instructions/                  # Instruction execution logic
│           │   ├── mod.rs                     # Instruction re-exports
│           │   ├── initialize_vault.rs        # Vault & Policy PDA initialization
│           │   ├── deposit.rs                 # Asset deposit & pro-rata share minting
│           │   ├── withdraw.rs                # Share burn & pro-rata asset return
│           │   ├── update_policy.rs           # Policy parameter modification
│           │   └── emergency_exit.rs          # Circuit breaker pause/unpause
│           └── utils/                         # Smart contract helpers
│               ├── mod.rs
│               ├── math.rs                    # Safe math & share calculations
│               └── validation.rs              # Account constraint validation
│
├── integrations/                              # External protocol connectors
│   ├── solana/                                # Solana RPC, WebSocket, & Anchor client
│   │   ├── Cargo.toml
│   │   ├── mod.rs
│   │   ├── rpc.rs                             # SolanaRpcClient implementation
│   │   ├── websocket.rs                       # SolanaWebSocketClient log listener
│   │   ├── anchor_client.rs                   # Instruction builders & read-only lock
│   │   ├── accounts.rs                        # Binary deserialization for Anchor accounts
│   │   └── tests/
│   │       └── solana_integration_test.rs
│   ├── pyth/                                  # Pyth Network Hermes price oracle
│   │   ├── Cargo.toml
│   │   ├── mod.rs
│   │   ├── client.rs                          # PythClient & mock mode implementation
│   │   ├── feeds.rs                           # Pyth feed ID registry
│   │   ├── types.rs                           # Pyth price response data models
│   │   └── tests/
│   │       └── pyth_test.rs
│   ├── jupiter/                               # Jupiter v6 DEX Aggregator
│   │   ├── Cargo.toml
│   │   ├── mod.rs
│   │   ├── client.rs                          # JupiterClient & mock mode implementation
│   │   ├── quotes.rs                          # Price impact & slippage validation
│   │   ├── swap.rs                            # Swap instruction builder
│   │   ├── types.rs                           # Quote & swap request/response models
│   │   └── tests/
│   │       └── jupiter_test.rs
│   ├── xstocks/                               # (Empty directory - generic SPL used)
│   └── ondo/                                  # (Empty directory - generic SPL used)
│
├── sdk/                                       # TypeScript Client SDK
│   ├── package.json                           # SDK dependencies (@solana/web3.js, @solana/spl-token)
│   ├── tsconfig.json                          # SDK TypeScript compiler config
│   ├── src/
│   │   ├── index.ts                           # Root SDK exports
│   │   ├── client.ts                          # EquityCatalystClient master orchestrator
│   │   ├── vaults.ts                          # VaultsClient (PDAs, initialize, deposit, withdraw)
│   │   ├── policies.ts                        # PoliciesClient (PDAs, update_policy)
│   │   ├── events.ts                          # EventsClient (API event querying)
│   │   ├── execution.ts                       # ExecutionClient (Quote evaluation & orders)
│   │   ├── portfolio.ts                       # PortfolioClient (Positions & drift querying)
│   │   ├── credit.ts                          # CreditClient (Borrow, repay, loan PDAs)
│   │   └── types.ts                           # Typed interfaces & discriminator constants
│   ├── tests/
│   │   └── sdk.test.ts                        # SDK unit & PDA derivation tests
│   └── dist/                                  # Compiled JavaScript & declaration files
│
├── db/                                        # Database migrations & seeds
│   ├── migrations/                            # PostgreSQL SQL migrations
│   │   ├── 001_initial.sql                    # UUID extensions & updated_at trigger
│   │   ├── 002_vaults.sql                     # vaults mirror table
│   │   ├── 003_policies.sql                   # policies risk parameters table
│   │   ├── 004_events.sql                     # events ingestion table
│   │   ├── 005_executions.sql                 # executions audit trail table
│   │   └── 006_portfolios.sql                 # portfolios & positions table
│   └── seeds/                                 # Seed data scripts
│
├── demo-data/                                 # Deterministic test fixtures & demo scenarios
│   ├── demo-script.json                       # NVDA earnings surprise replay scenario
│   ├── nvda-earnings-event.json               # Raw earnings event fixture
│   ├── sample-vault.json                      # Seed vault parameters
│   ├── sample-policy.json                     # Seed policy parameters
│   ├── sample-portfolio.json                  # Initial 5-asset portfolio positions
│   └── sample-position.json                   # Single position fixture
│
├── scripts/                                   # Automation & demo scripts
│   ├── initialize.ts                          # Setup & account bootstrap script
│   └── replay-demo.ts                         # End-to-end terminal demo replay runner
│
├── tests/                                     # Cross-cutting integration tests
│   ├── anchor/                                # Anchor local validator test suite
│   │   ├── equity_vault.ts                    # 15 Anchor integration tests in TypeScript
│   │   └── equity_vault.js                    # Compiled test runner file
│   └── integration/                           # Full-stack E2E tests
│       └── full_pipeline_e2e.test.ts          # E2E pipeline test
│
└── docs/                                      # Complete Engineering Documentation Suite
    ├── README.md                              # Master index & navigation map
    ├── 01-overview.md through 26-...          # 26 Detailed architectural topic documents
```
