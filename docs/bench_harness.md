# Bench Harness (cross-protocol)

This repository includes a **process-based benchmarking harness** intended to run **release-built binaries** and generate reproducible performance reports for Suite A (deterministic) and Suite B (stochastic) scenarios described in `docs/benchmarking.md`.

The harness is designed to be **protocol-agnostic**. WIND is supported today via a WIND-specific “agent” binary. A future DIM/C implementation can be integrated by providing a compatible agent binary without changing the harness.

## Goals

- **Benchmark release artifacts**: measure the performance of what you ship (`target/release/...`), not library code in-process.
- **Cross-protocol comparability**: the same harness drives WIND and any future C/DIM implementation.
- **Reproducibility**: fixed seeds, explicit configs, per-run raw logs, and machine-readable summaries.
- **Paper-friendly outputs**: percentile-focused results with histograms for tail behavior.

## Where it lives

- Harness root: `bench-harness/`
- Python package: `bench-harness/bench_harness/`
- WIND agent: `bench-harness/agents/wind-agent/`
- Results output (gitignored): `bench-harness/results/`

## Architecture overview

The harness is an **orchestrator**, not a load generator by itself.

1. It spawns a registry (currently: `wind-registry`) as an OS process.
2. It spawns one or more **agent** processes which act as publishers/subscribers.
3. Each agent prints a final **single-line JSON object** to stdout.
4. The harness collects agent outputs, merges histograms, and writes `summary.json`.

Key modules:

- `bench_harness/cli.py`: CLI entry point (`python3 -m bench_harness ...`).
- `bench_harness/scenarios.py`: scenario orchestration (Suite A/B).
- `bench_harness/proc.py`: process management and JSON summary extraction.
- `bench_harness/metrics.py`: histogram merge + percentile summarization.
- `bench_harness/resources.py`: lightweight `/proc` sampling for RSS/CPU tick snapshots.

## Scenario mapping (Suite A / Suite B)

The harness provides scenario names matching the benchmark plan:

- `a1`: Baseline latency (1 publisher, 1 subscriber)
  - Deterministic by default; can be switched to Poisson scheduling with `--poisson`.
- `a2`: Fan-out (1 publisher, N subscribers)
- `a3`: Fan-in (N publishers, 1 subscriber)
- `a4`: Scalability (N publishers, M subscribers, each subscriber connects to a subset)
- `b1`: Stochastic latency profile (Poisson arrivals + IoT payload mix)
- `b2`: Scalability under chaos (multi-pub/multi-sub + stochastic load + subset subscriptions)

Notes:

- Suite B’s payload sizes follow the plan’s “Typical IoT” mix inside the publisher agent.
- Subset subscription topologies are implemented by having each subscriber subscribe to a deterministic subset of service names.

## Output format

Each harness run creates a timestamped directory and prints its path.

- `config.json`: exact CLI arguments (and resolved paths) used for the run.
- `summary.json`: aggregated histogram summary and per-run summaries.
- `raw/run-XX/result.json`: per-run structured result (includes resource samples).
- `raw/run-XX/*.stdout.log` / `*.stderr.log`: raw process logs for debugging.

### Aggregation behavior

- For multi-subscriber scenarios (`a2`, `a4`, `b2`), the harness merges all subscribers’ `latency_hist` into one histogram for that run.
- For repeated runs (`--runs`), the harness merges per-run histograms into one overall histogram.

## Running the harness

### Build the required binaries

From the repo root:

```bash
cargo build --release -p wind-registry
cargo build --release --manifest-path bench-harness/agents/wind-agent/Cargo.toml
```

### Run examples

From `bench-harness/`:

```bash
# Suite A1 baseline
python3 -m bench_harness run a1 --duration-secs 5 --hz 1000 --payload-bytes 256

# Suite A2 fan-out
python3 -m bench_harness run a2 --subscribers 50 --duration-secs 10

# Suite B1 stochastic latency profile
python3 -m bench_harness run b1 --lambda-hz 10000 --duration-secs 10
```

### Reproducibility knobs

- Use `--seed` to make stochastic schedules and payload bytes deterministic.
- Use `--runs` (default: 5) to repeat and aggregate.
- Keep `--registry-addr` fixed to avoid accidental cross-talk with other services.

## Extending

- To add a new protocol: implement a compatible agent binary (see `docs/benchmark_agents.md`).
- To add new scenarios: implement a new `run_*_once(...)` function in `bench_harness/scenarios.py` and register a subcommand in `bench_harness/cli.py`.
