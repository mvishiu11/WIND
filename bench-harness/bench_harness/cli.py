from __future__ import annotations

import argparse
import datetime as _dt
from pathlib import Path

from .results import RunPaths
from .metrics import Histogram, summarize_hist
from .scenarios import (
    Binaries,
    run_a1_once,
    run_a2_once,
    run_a3_once,
    run_a4_once,
    run_b1_once,
    run_b2_once,
)


def _path(p: str) -> Path:
    return Path(p).expanduser().resolve()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="bench-harness")
    sub = parser.add_subparsers(dest="cmd", required=True)

    run = sub.add_parser("run", help="run scenarios")
    run_sub = run.add_subparsers(dest="scenario", required=True)

    a1 = run_sub.add_parser("a1", help="Suite A1 baseline latency")
    a1.add_argument("--wind-registry", type=_path, default=_path("../target/release/wind-registry"))
    a1.add_argument("--wind-agent", type=_path, default=_path("./agents/wind-agent/target/release/wind-agent"))
    a1.add_argument("--results-dir", type=_path, default=_path("./results"))
    a1.add_argument("--registry-addr", default="127.0.0.1:7001")
    a1.add_argument("--service", default="BENCH/A1/LATENCY")
    a1.add_argument("--payload-bytes", type=int, default=256)
    a1.add_argument("--hz", type=float, default=1000.0)
    a1.add_argument("--duration-secs", type=int, default=5)
    a1.add_argument("--poisson", action="store_true")
    a1.add_argument("--seed", type=int, default=1)
    a1.add_argument("--runs", type=int, default=5)

    a2 = run_sub.add_parser("a2", help="Suite A2 fan-out throughput")
    a2.add_argument("--wind-registry", type=_path, default=_path("../target/release/wind-registry"))
    a2.add_argument("--wind-agent", type=_path, default=_path("./agents/wind-agent/target/release/wind-agent"))
    a2.add_argument("--results-dir", type=_path, default=_path("./results"))
    a2.add_argument("--registry-addr", default="127.0.0.1:7001")
    a2.add_argument("--service", default="BENCH/A2/FANOUT")
    a2.add_argument("--subscribers", type=int, default=10)
    a2.add_argument("--payload-bytes", type=int, default=1024)
    a2.add_argument("--hz", type=float, default=10_000.0)
    a2.add_argument("--duration-secs", type=int, default=5)
    a2.add_argument("--seed", type=int, default=1)
    a2.add_argument("--runs", type=int, default=5)

    a3 = run_sub.add_parser("a3", help="Suite A3 fan-in stress")
    a3.add_argument("--wind-registry", type=_path, default=_path("../target/release/wind-registry"))
    a3.add_argument("--wind-agent", type=_path, default=_path("./agents/wind-agent/target/release/wind-agent"))
    a3.add_argument("--results-dir", type=_path, default=_path("./results"))
    a3.add_argument("--registry-addr", default="127.0.0.1:7001")
    a3.add_argument("--service-prefix", default="BENCH/A3/FANIN")
    a3.add_argument("--publishers", type=int, default=10)
    a3.add_argument("--payload-bytes", type=int, default=1024)
    a3.add_argument("--hz-per-publisher", type=float, default=1000.0)
    a3.add_argument("--duration-secs", type=int, default=5)
    a3.add_argument("--seed", type=int, default=1)
    a3.add_argument("--runs", type=int, default=5)

    a4 = run_sub.add_parser("a4", help="Suite A4 scalability")
    a4.add_argument("--wind-registry", type=_path, default=_path("../target/release/wind-registry"))
    a4.add_argument("--wind-agent", type=_path, default=_path("./agents/wind-agent/target/release/wind-agent"))
    a4.add_argument("--results-dir", type=_path, default=_path("./results"))
    a4.add_argument("--registry-addr", default="127.0.0.1:7001")
    a4.add_argument("--service-prefix", default="BENCH/A4/SCALE")
    a4.add_argument("--publishers", type=int, default=10)
    a4.add_argument("--subscribers", type=int, default=100)
    a4.add_argument("--publishers-per-subscriber", type=int, default=10)
    a4.add_argument("--payload-bytes", type=int, default=1024)
    a4.add_argument("--hz-per-publisher", type=float, default=1000.0)
    a4.add_argument("--duration-secs", type=int, default=5)
    a4.add_argument("--seed", type=int, default=1)
    a4.add_argument("--runs", type=int, default=5)

    b1 = run_sub.add_parser("b1", help="Suite B1 stochastic latency profile")
    b1.add_argument("--wind-registry", type=_path, default=_path("../target/release/wind-registry"))
    b1.add_argument("--wind-agent", type=_path, default=_path("./agents/wind-agent/target/release/wind-agent"))
    b1.add_argument("--results-dir", type=_path, default=_path("./results"))
    b1.add_argument("--registry-addr", default="127.0.0.1:7001")
    b1.add_argument("--service", default="BENCH/B1/LATENCY")
    b1.add_argument("--lambda-hz", type=float, default=10_000.0)
    b1.add_argument("--duration-secs", type=int, default=5)
    b1.add_argument("--seed", type=int, default=1)
    b1.add_argument("--runs", type=int, default=5)

    b2 = run_sub.add_parser("b2", help="Suite B2 scalability under chaos")
    b2.add_argument("--wind-registry", type=_path, default=_path("../target/release/wind-registry"))
    b2.add_argument("--wind-agent", type=_path, default=_path("./agents/wind-agent/target/release/wind-agent"))
    b2.add_argument("--results-dir", type=_path, default=_path("./results"))
    b2.add_argument("--registry-addr", default="127.0.0.1:7001")
    b2.add_argument("--service-prefix", default="BENCH/B2/CHAOS")
    b2.add_argument("--publishers", type=int, default=10)
    b2.add_argument("--subscribers", type=int, default=50)
    b2.add_argument("--publishers-per-subscriber", type=int, default=10)
    b2.add_argument("--lambda-hz-per-publisher", type=float, default=1000.0)
    b2.add_argument("--duration-secs", type=int, default=5)
    b2.add_argument("--seed", type=int, default=1)
    b2.add_argument("--runs", type=int, default=5)

    args = parser.parse_args(argv)

    if args.cmd == "run":
        run_id = _dt.datetime.utcnow().strftime("%Y%m%dT%H%M%SZ")
        paths = RunPaths.create(args.results_dir, f"{args.scenario}-{run_id}")
        bins = Binaries(wind_registry=args.wind_registry, wind_agent=args.wind_agent)

        total_hist = Histogram(counts=[])
        per_run: list[dict[str, object]] = []

        for i in range(int(getattr(args, "runs", 1))):
            run_out = paths.raw_dir / f"run-{i:02d}"
            run_out.mkdir(parents=True, exist_ok=True)

            if args.scenario == "a1":
                result = run_a1_once(
                    bins=bins,
                    registry_addr=args.registry_addr,
                    service=args.service,
                    payload_bytes=args.payload_bytes,
                    hz=args.hz,
                    duration_secs=args.duration_secs,
                    poisson=args.poisson,
                    seed=args.seed + i,
                    out_dir=run_out,
                )
                sub = result.get("subscriber") or {}
                run_hist = Histogram.from_json(sub.get("latency_hist"))
            elif args.scenario == "a2":
                result = run_a2_once(
                    bins=bins,
                    registry_addr=args.registry_addr,
                    service=args.service,
                    subscribers=args.subscribers,
                    payload_bytes=args.payload_bytes,
                    hz=args.hz,
                    duration_secs=args.duration_secs,
                    seed=args.seed + i,
                    out_dir=run_out,
                )
                subs = result.get("subscribers") or []
                run_hist = Histogram(counts=[])
                for sub in subs:
                    run_hist = run_hist.merge(Histogram.from_json(sub.get("latency_hist")))
            elif args.scenario == "a3":
                services = [f"{args.service_prefix}/{j:04d}" for j in range(args.publishers)]
                result = run_a3_once(
                    bins=bins,
                    registry_addr=args.registry_addr,
                    services=services,
                    payload_bytes=args.payload_bytes,
                    hz_per_publisher=args.hz_per_publisher,
                    duration_secs=args.duration_secs,
                    seed=args.seed + i,
                    out_dir=run_out,
                )
                sub = result.get("subscriber") or {}
                run_hist = Histogram.from_json(sub.get("latency_hist"))
            elif args.scenario == "a4":
                services = [f"{args.service_prefix}/{j:04d}" for j in range(args.publishers)]
                result = run_a4_once(
                    bins=bins,
                    registry_addr=args.registry_addr,
                    services=services,
                    subscribers=args.subscribers,
                    publishers_per_subscriber=args.publishers_per_subscriber,
                    payload_bytes=args.payload_bytes,
                    hz_per_publisher=args.hz_per_publisher,
                    duration_secs=args.duration_secs,
                    seed=args.seed + i,
                    out_dir=run_out,
                )
                subs = result.get("subscribers") or []
                run_hist = Histogram(counts=[])
                for sub in subs:
                    run_hist = run_hist.merge(Histogram.from_json(sub.get("latency_hist")))
            elif args.scenario == "b1":
                result = run_b1_once(
                    bins=bins,
                    registry_addr=args.registry_addr,
                    service=args.service,
                    lambda_hz=args.lambda_hz,
                    duration_secs=args.duration_secs,
                    seed=args.seed + i,
                    out_dir=run_out,
                )
                sub = result.get("subscriber") or {}
                run_hist = Histogram.from_json(sub.get("latency_hist"))
            elif args.scenario == "b2":
                services = [f"{args.service_prefix}/{j:04d}" for j in range(args.publishers)]
                result = run_b2_once(
                    bins=bins,
                    registry_addr=args.registry_addr,
                    services=services,
                    subscribers=args.subscribers,
                    publishers_per_subscriber=args.publishers_per_subscriber,
                    lambda_hz_per_publisher=args.lambda_hz_per_publisher,
                    duration_secs=args.duration_secs,
                    seed=args.seed + i,
                    out_dir=run_out,
                )
                subs = result.get("subscribers") or []
                run_hist = Histogram(counts=[])
                for sub in subs:
                    run_hist = run_hist.merge(Histogram.from_json(sub.get("latency_hist")))
            else:
                raise AssertionError("unhandled scenario")

            paths.write_json(f"raw/run-{i:02d}/result.json", result)

            total_hist = total_hist.merge(run_hist)
            per_run.append({"run": i, "latency": summarize_hist(run_hist)})

        cfg = {
            k: (str(v) if isinstance(v, Path) else v)
            for k, v in vars(args).items()
        }
        cfg["scenario"] = args.scenario
        paths.write_json("config.json", cfg)
        paths.write_json(
            "summary.json",
            {
                "latency": summarize_hist(total_hist),
                "per_run": per_run,
            },
        )

        print(str(paths.root))
        return 0

    return 2
