# Bench Harness (cross-protocol)

This folder contains a **process-based benchmarking harness** that runs **release binaries**.

- The harness is language-agnostic (orchestrator in Python).
- Each protocol provides an **agent binary** that can act as publisher/subscriber and prints a JSON summary.

## Quick start (WIND)

Build release binaries:

```bash
# from repo root
cargo build --release -p wind-registry
cargo build --release --manifest-path bench-harness/agents/wind-agent/Cargo.toml
```

Run Suite A1 (baseline latency) once:

```bash
python3 -m bench_harness run a1 \
  --registry-addr 127.0.0.1:7001 \
  --payload-bytes 256 \
  --hz 1000 \
  --duration-secs 5
```

Run the other scenarios (defaults follow `docs/benchmarking.md`):

```bash
python3 -m bench_harness run a2 --subscribers 50
python3 -m bench_harness run a3 --publishers 50
python3 -m bench_harness run a4 --publishers 50 --subscribers 500 --publishers-per-subscriber 10

python3 -m bench_harness run b1 --lambda-hz 10000
python3 -m bench_harness run b2 --publishers 50 --subscribers 500 --publishers-per-subscriber 10
```

This prints the results directory path and writes:
- `config.json`
- `summary.json`
- `raw/run-XX/result.json` (per-run structured output, including resource samples)
- `raw/run-XX/*.log` (stdout/stderr per process)

## Agent contract (high level)

Agents are spawned as OS processes and must:
- accept CLI flags to configure topology and workload
- emit a final JSON object on stdout (single line)
- include `latency_hist` as a list of `[latency_us, count]` pairs for aggregation

For cross-language comparability, WIND agents publish `WindValue::Bytes` payloads where the first 8 bytes are a little-endian `i64` UNIX timestamp in microseconds.
