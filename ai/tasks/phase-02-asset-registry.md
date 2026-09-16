# Phase 02 — Asset Registry

Status: DONE
Owner: Agent 2
Dependencies: Phase 00
Blocked by: None

## Checklist

- [x] implementation
- [x] unit tests
- [x] integration tests
- [x] verification
- [x] documentation
- [x] review

## Objective

Normalize external RWA asset sources.

## Context

Tokenized real-world assets (RWAs) and pre-IPO equity tokens originate from multiple disparate platforms (PreStocks, Tessera, Backed Finance). The Asset Registry establishes an extensible adapter architecture to discover, validate, and normalize these assets into a single canonical `RwaAsset` representation.

## Allowed Files

- `apps/api/src/providers/prestocks/**`
- `apps/api/src/providers/tessera/**`
- `apps/api/src/services/asset_registry.rs`
- `apps/api/src/routes/assets.rs`
- `crates/shared/src/models/asset.rs`
- `tests/assets/**`

## Forbidden Files

- `apps/web/**` (no provider-specific scraping or direct secret access)
- Direct database writes from within provider adapters
- Programs layer (`programs/equity_vault/**`)

## Requirements

1. **Providers**:
   - PreStocks adapter
   - Tessera adapter
2. **Interface**:
   - `AssetProvider` asynchronous trait defining standard discovery methods:
     - `list_assets(&self) -> Result<Vec<RwaAsset>, ProviderError>`
     - `get_asset(&self, symbol: &str) -> Result<Option<RwaAsset>, ProviderError>`
3. **Output**:
   - Unified `RwaAsset` domain struct:
     - `symbol`: canonical ticker (e.g. `NVDAx`, `AAPLx`)
     - `name`: descriptive company name
     - `mint`: validated Solana public key string
     - `decimals`: integer token decimal precision (e.g. 8)
     - `provider`: source platform enum (`PreStocks`, `Tessera`, `Backed`)
     - `source_id`: platform-specific reference identifier
     - `status`: active, suspended, or pre-launch
4. **Validation & Invariants**:
   - Validate that every mint address is a valid 32-byte Solana public key.
   - Normalize decimal representation across calculations.
   - Preserve provider identity and original source ID.
   - Never infer ownership rights or legal custody beyond statutory backing certificates.

## Implementation Steps

1. Define `RwaAsset` model and `AssetProvider` trait in `crates/shared` and API domain layer.
2. Build `PreStocksProvider` adapter mapping raw PreStocks API payloads to `RwaAsset`.
3. Build `TesseraProvider` adapter mapping fractionalized vault shares to `RwaAsset`.
4. Implement `AssetRegistryService` in `apps/api/src/services/asset_registry.rs` to aggregate adapters with memory caching.
5. Add mint validation using `solana_sdk::pubkey::Pubkey::from_str`.
6. Expose REST endpoints: `GET /assets` and `GET /assets/:symbol`.
7. Write unit and integration tests for parsing, validation, and provider downtime.

## Tests

- Valid asset ingestion and decimal normalization.
- Missing or malformed mint address rejection.
- Invalid or truncated JSON response parsing.
- Provider unavailable handling (graceful degradation without registry crash).

## Verification Commands

```bash
cargo test --package equity-catalyst-api asset_registry
curl -s http://localhost:8080/assets | jq .
curl -s http://localhost:8080/assets/NVDAx | jq .
```

## Acceptance Criteria

- All supported RWA assets returned as unified, typed `RwaAsset` instances.
- Zero invalid or unparseable mint addresses permitted into the registry.
- Provider outages fail closed without terminating the API service.

## Documentation Updates

- Update `ai/current-state.md` with Asset Registry status.
- Update `ai/context/prestocks.md` and `ai/context/tessera.md` with provider integration notes.

## Failure Conditions

- Propagating provider-specific schemas into core domain or frontend layers.
- Allowing invented or mock equity mints to be registered as valid RWAs.
- Silent assumption of legal custody or share ownership rights.
