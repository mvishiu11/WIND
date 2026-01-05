# Benchmark Agents

A **benchmark agent** is a small OS process used by the cross-protocol harness (`bench-harness/`) to create load (publishers) and to measure outcomes (subscribers). Agents are the protocol-specific part of the system.

- Harness responsibility: orchestration, run matrices, aggregation, reporting.
- Agent responsibility: implement protocol interactions and emit structured metrics.

This split is what allows WIND-vs-C/DIM comparisons: the harness stays constant, only the agent changes.

## Why agents exist (vs “just run existing binaries”)

You can only “just run binaries” if those binaries:

- can be parameterized for **rate model**, **payload distribution**, **duration**, and **topology**
- can emit **machine-readable** results (JSON) including latency histograms/percentiles
- behave consistently across protocols

Production/debug tools (e.g., a CLI or a demo example) typically aren’t designed for deterministic load generation and rigorous measurement. An agent is intentionally minimal and benchmark-focused.

## Agent contract

The harness treats an agent as a black box that:

1. Accepts CLI flags.
2. Runs for a bounded duration.
3. Writes a single final JSON object on stdout.
4. Exits with code 0 on success.

The harness captures stdout/stderr to per-run log files and extracts the last JSON object it sees on stdout.

### Required behaviors

- **Single-line JSON summary** on stdout at end of run.
- Include a histogram representation for merging:
  - `latency_hist`: list of `[latency_us, count]` pairs.
- Deterministic randomness:
  - accept a `--seed` and use it for schedules and random payload bytes.

### Timestamp-in-payload convention

To compare implementations across languages, agents should agree on how latency is measured.

For pub/sub latency, the current convention is:

- Publisher sends `bytes` payload where:
  - first 8 bytes: little-endian `i64` UNIX timestamp in microseconds (send time)
  - remaining bytes: pseudo-random (non-compressible), generated from a fixed seed
- Subscriber computes latency as `now_us - sent_us`.

This convention is used by the WIND agent today and is easy to reproduce in C.

## WIND agent (reference implementation)

Location: `bench-harness/agents/wind-agent/`

Build:

```bash
cargo build --release --manifest-path bench-harness/agents/wind-agent/Cargo.toml
```

### Publisher CLI

Current shape:

- `wind-agent publisher`
  - `--service <NAME>`
  - `--registry <ADDR>`
  - `--bind <ADDR>` (default `127.0.0.1:0`)
  - `--duration-secs <N>`
  - `--mode deterministic|poisson`
  - `--hz <RATE>` (deterministic = fixed period; poisson = mean λ)
  - `--payload-bytes <N>` (used when profile is fixed)
  - `--payload-profile fixed|iot`
  - `--seed <N>`

Publisher output JSON contains at least:

- `role: "publisher"`
- `service`, `registry`
- `mode`, `hz`, `payload_bytes`, `duration_secs`
- `published`, `publish_errors`

### Subscriber CLI

Current shape:

- `wind-agent subscriber`
  - `--registry <ADDR>`
  - either:
    - `--service <NAME>` (repeatable; can be provided multiple times)
    - `--pattern <GLOB>` (discover then subscribe to exact names)
  - `--duration-secs <N>`
  - `--max-samples <N>` (optional)
  - `--seed <N>` (reserved)

Subscriber output JSON contains at least:

- `role: "subscriber"`
- `services: [..]`
- `received`, `received_bytes`, `decode_errors`
- `latency_hist: [[latency_us, count], ...]`
- `latency: {min_us,p50_us,p90_us,p95_us,p99_us,p999_us,max_us}`

## Implementing a future C/DIM agent

To integrate a C/DIM implementation later:

1. Create a new agent binary (example path: `bench-harness/agents/dim-agent/`).
2. Implement the same contract (CLI flags + JSON fields).
3. Point the harness at it via `--wind-agent`-equivalent flag (the harness can be extended to accept `--dim-agent` or a generic `--agent` per protocol).

### Minimal checklist

- Publisher supports deterministic and Poisson scheduling.
- Publisher supports the IoT payload size mix:
  - 70%: 64–256 B
  - 25%: 1–4 KiB
  - 5%: 16–32 KiB
- Subscriber supports subscribing to multiple explicit services.
- Subscriber emits a mergeable histogram (`latency_hist`).
- Subscriber handles startup races (retry until discover/subscribe succeeds or timeout).

### JSON compatibility

The harness currently merges histograms by reading `latency_hist` and does not require a strict schema validator. Still, keeping field names identical across agents will make analysis simpler and reduce harness branching.

## Common pitfalls

- **Service discovery vs subscription**: even if the registry supports patterns, the subscription step usually needs an exact service name. Agents should discover first, then subscribe to exact names.
- **Startup races**: the subscriber may start before the publisher registers. Agents should retry subscribe/discover for a bounded window.
- **Clock issues**: on multi-host benchmarks, publishers/subscribers must have synchronized clocks (e.g., NTP) for timestamp-based latency. For single-host runs this is naturally consistent.
