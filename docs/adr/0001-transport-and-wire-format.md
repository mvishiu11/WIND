# ADR 0001: Transport and wire format

- Status: Accepted
- Date: 2026-01-05

## Context

WIND needs a simple, performant, cross-language wire protocol for Pub/Sub and RPC.
The project targets high throughput and low latency and must remain easy to implement in a future C/DIM comparison.

## Decision

- Use **TCP** as the transport for all communication.
- Encode each protocol message as:
  - a **4-byte big-endian unsigned length prefix** (Tokio `read_u32` / `put_u32` semantics), followed by
  - a **bincode-serialized** `wind_core::Message`.
- Enforce a **maximum message size of 16 MiB**.

Implementation anchors:

- Framing and max size: `crates/wind-core/src/codec.rs` (`MessageCodec`, `MAX_MESSAGE_SIZE`).
- Message structure: `crates/wind-core/src/protocol.rs`.

## Consequences

- Efficient framing and serialization with minimal per-message overhead.
- Simpler cross-language implementations (length-prefixed binary blobs).
- TCP provides in-order, reliable delivery; application-level reliability controls in QoS are currently mostly declarative.
- bincode is Rust-centric; cross-language support requires a compatible encoding strategy (or a future migration to a more language-neutral encoding).

## Alternatives considered

- **UDP + custom reliability**: lower overhead but significantly more complexity and operational risk.
- **HTTP/JSON or gRPC**: easier interoperability but higher overhead and less control over low-latency behavior.
- **TLS everywhere**: improves security but adds handshake/CPU overhead and complicates microbenchmarks; can be introduced later behind an option.

## Notes / follow-ups

- Consider a versioned envelope (magic/version) ahead of the payload if forward/backward compatibility becomes a priority.
- Consider documenting endianness explicitly in external-facing protocol documentation.
