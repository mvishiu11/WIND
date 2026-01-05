# ADR 0002: Service discovery via registry + TTL

- Status: Accepted
- Date: 2026-01-05

## Context

Clients need to locate publishers and RPC servers dynamically without hard-coded endpoints.
Services may disappear; discovery must avoid returning stale endpoints.

## Decision

- Introduce a dedicated **Registry** service (`wind-registry`) responsible for:
  - accepting service registrations (`RegisterService`),
  - answering discovery queries by name or pattern (`DiscoverServices`),
  - expiring services using **TTL**.
- Store active services in an in-memory concurrent map with per-entry expiration.
- Support **glob-pattern discovery** (e.g. `SENSOR/*/TEMP`).
- Refresh liveness via periodic re-registration by the service process (publisher “heartbeat” implemented as repeated `RegisterService`).

Implementation anchors:

- Registry server and message handling: `crates/wind-registry/src/server.rs`.
- TTL storage, expiration, and pattern matching: `crates/wind-registry/src/registry.rs`, `crates/wind-registry/src/pattern.rs`.
- Publisher periodic re-registration: `crates/wind-server/src/publisher.rs` (`start_heartbeat_task`).

## Consequences

- Centralized discovery simplifies clients and enables dynamic deployments.
- TTL reduces the risk of stale endpoints without requiring explicit deregistration.
- Registry becomes a logical single point of failure; high availability would require replication or caching.
- Current heartbeat semantics are “renew by re-registering”; there is no dedicated renew/heartbeat message handled by the registry server.

## Alternatives considered

- **Static config**: simplest but not suitable for dynamic systems.
- **Peer-to-peer gossip**: removes central dependency but increases complexity.
- **External discovery systems** (Consul/etcd/DNS-SD): mature but adds ops dependencies.

## Notes / follow-ups

- `MessagePayload::Heartbeat` exists but is not handled by the registry; consider either implementing it or removing it to avoid ambiguity.
- `RpcServer` currently registers once; consider adding periodic re-registration similar to `Publisher`.
