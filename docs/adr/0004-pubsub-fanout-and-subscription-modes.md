# ADR 0004: Pub/Sub fan-out and subscription modes

- Status: Accepted
- Date: 2026-01-05

## Context

Publishers must efficiently broadcast updates to many subscribers, with configurable delivery behaviors similar to DIM (once/periodic/on-change).

## Decision

- Implement Pub/Sub with a `Publisher` that:
  - accepts TCP client connections,
  - receives a `Subscribe` request and replies with `SubscribeAck` (including optional `current_value`),
  - fans out updates from a single internal update stream to all connected clients.
- Use a Tokio `broadcast` channel as the internal update bus.
- Apply per-client filtering based on `SubscriptionMode`:
  - `Once`: send the next eligible update once,
  - `OnChange`: send only when value changes,
  - `Periodic`: enforce minimum interval between sends.

Implementation anchors:

- Pub/Sub server implementation: `crates/wind-server/src/publisher.rs`.
- Subscription mode types: `crates/wind-core/src/types.rs`.
- Subscriber client handling of `Publish` and `SubscribeAck`: `crates/wind-client/src/subscriber.rs`.

## Consequences

- Broadcast channel decouples producer (`publish`) from client I/O fan-out.
- Per-client subscription behavior is implemented server-side and does not require multiple topics/streams.
- Current publisher design shares a single `TcpStream` per client between tasks; it works for the current protocol shape but can become brittle as message types expand.

## Alternatives considered

- **Per-client dedicated writer task + mpsc**: cleaner separation of concerns and backpressure; more code.
- **One connection per subscription**: simpler server state, higher connection overhead.

## Notes / follow-ups

- Consider splitting sockets into read/write halves per client to avoid write/read contention as protocol complexity grows.
- QoS fields (`QosParams`) are currently accepted but not enforced beyond channel sizing.
