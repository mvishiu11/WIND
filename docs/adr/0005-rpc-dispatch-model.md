# ADR 0005: RPC server dispatch model

- Status: Accepted
- Date: 2026-01-05

## Context

WIND needs request/response calls with dynamic method selection and asynchronous handlers.

## Decision

- Implement RPC with an `RpcServer` that:
  - registers itself with the registry,
  - accepts TCP connections,
  - reads `RpcCall { service, method, params, schema_id }`,
  - dispatches to a handler map keyed by `method`,
  - responds with `RpcResponse { call_id, result, schema_id }`.
- Use a trait-object-safe handler interface (`RpcHandler`) to support async handlers without requiring an async trait.

Implementation anchors:

- RPC server: `crates/wind-server/src/rpc_server.rs`.
- RPC message payloads: `crates/wind-core/src/protocol.rs`.
- RPC client: `crates/wind-client/src/rpc_client.rs`.

## Consequences

- Simple per-connection request loop; concurrency scales by spawning one task per client connection.
- Method dispatch is flexible and works well for dynamic RPC APIs.
- The client currently uses a new connection per call; pipelining/multiplexing is not implemented.

## Alternatives considered

- **Multiplexed RPC over a single connection**: better efficiency but requires correlation, buffering, and more complex client/server state.
- **Codegen-based RPC stubs**: nicer ergonomics but adds schema/tooling requirements.

## Notes / follow-ups

- Add periodic re-registration for RPC servers to respect registry TTL semantics.
- Consider propagating schema validation failures as structured errors.
