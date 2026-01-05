# ADR 0003: Message model and typed values

- Status: Accepted
- Date: 2026-01-05

## Context

WIND must support Pub/Sub and RPC across a common message layer, with space for schema/type validation.

## Decision

- Represent all on-the-wire units as a `wind_core::Message` with:
  - `id: Uuid` (unique per message),
  - `timestamp_us: u64` (sender-side timestamp),
  - `payload: MessagePayload` (protocol operation).
- Represent data values with `wind_core::WindValue` as a tagged union of common scalar and container types.
- Carry an optional `schema_id` on operations that transmit or interpret a `WindValue` (e.g. Subscribe, Publish, RpcCall, RpcResponse) to enable type validation.

Implementation anchors:

- Message and payload definitions: `crates/wind-core/src/protocol.rs`.
- Value types: `crates/wind-core/src/types.rs`.
- Schema definitions and validation helpers: `crates/wind-core/src/schema.rs`.

## Consequences

- A single message model supports multiple interaction patterns.
- `WindValue` is easy to generate/consume in Rust and maps cleanly to dynamic languages.
- Schema validation exists as a library feature, but enforcement is not currently integrated into publisher/subscriber/RPC request paths.

## Alternatives considered

- **Multiple protocol-specific message types**: could reduce payload size but increases complexity and duplication.
- **Protobuf/FlatBuffers schema-first**: improves cross-language interop and evolution, but adds build tooling and a more rigid data model.

## Notes / follow-ups

- Consider documenting semantics of `timestamp_us` (wall clock vs monotonic) and whether receivers should trust it.
- Consider reserving/standardizing error payload usage (`MessagePayload::Error`) vs per-operation error fields.
