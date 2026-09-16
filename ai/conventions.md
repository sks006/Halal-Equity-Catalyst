# Engineering Conventions

## Rust

- Rust 2021 or workspace-selected edition
- cargo fmt
- cargo clippy
- cargo test
- Result-based error handling
- typed domain models
- no unwrap() in production paths unless justified
- no panic-based business logic

## Financial math

- Prefer integers/fixed-point/Decimal for monetary values.
- Explicitly document units.
- Explicitly document decimal precision.
- Avoid f64 for authoritative financial accounting.

## Async

Use Tokio.

Avoid:

- blocking I/O in async tasks
- unbounded channels without justification
- infinite retry loops

## APIs

- validate input at boundaries
- typed request/response DTOs
- typed errors
- timeouts
- bounded retries

## External providers

All external services require adapters.

Never spread provider-specific response types across the domain layer.

## Logging

Never log:

- API keys
- seed phrases
- private keys
- bearer tokens
- authorization headers

Logs must include correlation/request IDs where practical.

## Frontend

- TypeScript strict mode
- API calls through feature/API layer
- no provider secrets
- no direct signing logic outside wallet integration boundaries
