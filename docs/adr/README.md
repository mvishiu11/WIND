# Architecture Decision Records (ADRs)

This directory captures the key architectural decisions for WIND in a lightweight, reviewable format.

## Conventions

- File naming: `NNNN-short-title.md` (e.g. `0001-transport-and-wire-format.md`).
- Status values: **Proposed**, **Accepted**, **Superseded**.
- Keep ADRs implementation-grounded: prefer referencing concrete modules (e.g. `wind-core/src/codec.rs`) over aspirational design.

## ADR Index

- 0001: Transport and wire format (`0001-transport-and-wire-format.md`)
- 0002: Service discovery via registry + TTL (`0002-service-discovery-registry-and-ttl.md`)
- 0003: Message model and typed values (`0003-message-model-and-typed-values.md`)
- 0004: Pub/Sub fan-out and subscription modes (`0004-pubsub-fanout-and-subscription-modes.md`)
- 0005: RPC server dispatch model (`0005-rpc-dispatch-model.md`)
- 0006: Client connection strategy (`0006-client-connection-strategy.md`)

## Template

Use `adr-template.md` when creating a new ADR.
