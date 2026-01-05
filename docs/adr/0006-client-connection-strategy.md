# ADR 0006: Client connection strategy

- Status: Accepted
- Date: 2026-01-05

## Context

Client APIs should be straightforward and reliable under transient network failures without overcomplicating connection pooling.

## Decision

- Implement a lightweight `Connection` abstraction that:
  - lazily connects on first use,
  - retries connection attempts with exponential backoff up to a fixed maximum,
  - marks the connection disconnected when send/receive fails.
- For now, higher-level clients (`Subscriber`, `RpcClient`) establish a **new TCP connection per service interaction**:
  - `Subscriber::subscribe` connects to the discovered publisher and then uses that connection for the subscription stream.
  - `RpcClient::call` creates a new connection per call.

Implementation anchors:

- Connection manager: `crates/wind-client/src/connection.rs`.
- Subscriber and RPC client usage: `crates/wind-client/src/subscriber.rs`, `crates/wind-client/src/rpc_client.rs`.

## Consequences

- Simple mental model and fewer lifetime/pooling concerns.
- “Reconnect” exists at the connection primitive, but end-to-end resubscription and RPC retry semantics are not fully implemented (notably, `Subscriber` reconnection is a TODO).
- New connection per RPC call increases overhead and may cap throughput for small, frequent calls.

## Alternatives considered

- **Connection pooling / persistent per-service connections**: better performance, more state and failure modes.
- **Full session layer with automatic resubscribe and call retry**: best UX, significant complexity.

## Notes / follow-ups

- Add explicit reconnection + resubscribe behavior for subscriptions.
- Consider caching discovered `ServiceInfo` briefly to reduce registry load in high-frequency call paths.
