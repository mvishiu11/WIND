from __future__ import annotations

import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .metrics import Histogram, summarize_hist
from .proc import ManagedProcess, terminate_all
from .resources import collect_while, summarize_samples


@dataclass(frozen=True)
class Binaries:
    wind_registry: Path
    wind_agent: Path


def _wait_for_registry(registry_addr: str, timeout_secs: float = 5.0) -> None:
    # best-effort: registry is a TCP listener; we just sleep briefly
    deadline = time.time() + timeout_secs
    while time.time() < deadline:
        time.sleep(0.1)
        return


def _subscriber_services_args(services: list[str]) -> list[str]:
    argv: list[str] = []
    for svc in services:
        argv.extend(["--service", svc])
    return argv


def _merge_subscriber_hists(sub_summaries: list[dict[str, Any]]) -> Histogram:
    merged = Histogram(counts=[])
    for sub in sub_summaries:
        merged = merged.merge(Histogram.from_json(sub.get("latency_hist")))
    return merged


def run_a1_once(
    *,
    bins: Binaries,
    registry_addr: str,
    service: str,
    payload_bytes: int,
    hz: float,
    duration_secs: int,
    poisson: bool,
    seed: int,
    out_dir: Path,
) -> dict[str, Any]:
    procs: list[ManagedProcess] = []
    try:
        reg = ManagedProcess(
            argv=[str(bins.wind_registry), "--bind", registry_addr],
            env=None,
            cwd=None,
            stdout_path=out_dir / "registry.stdout.log",
            stderr_path=out_dir / "registry.stderr.log",
        )
        reg.start()
        procs.append(reg)
        _wait_for_registry(registry_addr)

        pub_mode = "poisson" if poisson else "deterministic"
        publisher = ManagedProcess(
            argv=[
                str(bins.wind_agent),
                "publisher",
                "--service",
                service,
                "--registry",
                registry_addr,
                "--duration-secs",
                str(duration_secs),
                "--mode",
                pub_mode,
                "--hz",
                str(hz),
                "--payload-bytes",
                str(payload_bytes),
                "--seed",
                str(seed),
            ],
            env=None,
            cwd=None,
            stdout_path=out_dir / "publisher.stdout.log",
            stderr_path=out_dir / "publisher.stderr.log",
        )
        publisher.start()
        procs.append(publisher)

        time.sleep(0.8)

        subscriber = ManagedProcess(
            argv=[
                str(bins.wind_agent),
                "subscriber",
                "--service",
                service,
                "--registry",
                registry_addr,
                "--duration-secs",
                str(duration_secs),
            ],
            env=None,
            cwd=None,
            stdout_path=out_dir / "subscriber.stdout.log",
            stderr_path=out_dir / "subscriber.stderr.log",
        )
        subscriber.start()
        procs.append(subscriber)

        pids = [p.pid for p in procs if p.pid is not None]
        samples = collect_while(
            pids=[int(pid) for pid in pids],
            should_continue=lambda: subscriber.poll() is None,
            interval_s=1.0,
        )

        subscriber.wait(timeout_secs=duration_secs + 10)
        publisher.terminate()

        sub_res = subscriber.result().json_summary or {}
        hist = Histogram.from_json(sub_res.get("latency_hist"))
        return {
            "subscriber": sub_res,
            "latency": summarize_hist(hist),
            "resources": summarize_samples(samples),
        }
    finally:
        terminate_all(reversed(procs))


def run_a2_once(
    *,
    bins: Binaries,
    registry_addr: str,
    service: str,
    subscribers: int,
    payload_bytes: int,
    hz: float,
    duration_secs: int,
    seed: int,
    out_dir: Path,
) -> dict[str, Any]:
    procs: list[ManagedProcess] = []
    try:
        reg = ManagedProcess(
            argv=[str(bins.wind_registry), "--bind", registry_addr],
            env=None,
            cwd=None,
            stdout_path=out_dir / "registry.stdout.log",
            stderr_path=out_dir / "registry.stderr.log",
        )
        reg.start()
        procs.append(reg)
        _wait_for_registry(registry_addr)

        publisher = ManagedProcess(
            argv=[
                str(bins.wind_agent),
                "publisher",
                "--service",
                service,
                "--registry",
                registry_addr,
                "--duration-secs",
                str(duration_secs),
                "--mode",
                "deterministic",
                "--hz",
                str(hz),
                "--payload-bytes",
                str(payload_bytes),
                "--seed",
                str(seed),
            ],
            env=None,
            cwd=None,
            stdout_path=out_dir / "publisher.stdout.log",
            stderr_path=out_dir / "publisher.stderr.log",
        )
        publisher.start()
        procs.append(publisher)

        time.sleep(0.8)

        subscribers_procs: list[ManagedProcess] = []
        for i in range(subscribers):
            sub = ManagedProcess(
                argv=[
                    str(bins.wind_agent),
                    "subscriber",
                    "--registry",
                    registry_addr,
                    "--duration-secs",
                    str(duration_secs),
                    "--service",
                    service,
                ],
                env=None,
                cwd=None,
                stdout_path=out_dir / f"subscriber-{i:04d}.stdout.log",
                stderr_path=out_dir / f"subscriber-{i:04d}.stderr.log",
            )
            sub.start()
            subscribers_procs.append(sub)
            procs.append(sub)

        pids = [p.pid for p in procs if p.pid is not None]
        samples = collect_while(
            pids=[int(pid) for pid in pids],
            should_continue=lambda: any(p.poll() is None for p in subscribers_procs),
            interval_s=1.0,
        )

        for sub in subscribers_procs:
            sub.wait(timeout_secs=duration_secs + 10)

        publisher.terminate()

        subs = [p.result().json_summary or {} for p in subscribers_procs]
        merged_hist = _merge_subscriber_hists(subs)
        return {
            "subscribers": subs,
            "latency": summarize_hist(merged_hist),
            "resources": summarize_samples(samples),
        }
    finally:
        terminate_all(reversed(procs))


def run_a3_once(
    *,
    bins: Binaries,
    registry_addr: str,
    services: list[str],
    payload_bytes: int,
    hz_per_publisher: float,
    duration_secs: int,
    seed: int,
    out_dir: Path,
) -> dict[str, Any]:
    procs: list[ManagedProcess] = []
    try:
        reg = ManagedProcess(
            argv=[str(bins.wind_registry), "--bind", registry_addr],
            env=None,
            cwd=None,
            stdout_path=out_dir / "registry.stdout.log",
            stderr_path=out_dir / "registry.stderr.log",
        )
        reg.start()
        procs.append(reg)
        _wait_for_registry(registry_addr)

        publishers: list[ManagedProcess] = []
        for i, svc in enumerate(services):
            pub = ManagedProcess(
                argv=[
                    str(bins.wind_agent),
                    "publisher",
                    "--service",
                    svc,
                    "--registry",
                    registry_addr,
                    "--duration-secs",
                    str(duration_secs),
                    "--mode",
                    "deterministic",
                    "--hz",
                    str(hz_per_publisher),
                    "--payload-bytes",
                    str(payload_bytes),
                    "--seed",
                    str(seed + i),
                ],
                env=None,
                cwd=None,
                stdout_path=out_dir / f"publisher-{i:04d}.stdout.log",
                stderr_path=out_dir / f"publisher-{i:04d}.stderr.log",
            )
            pub.start()
            publishers.append(pub)
            procs.append(pub)

        time.sleep(1.0)

        subscriber = ManagedProcess(
            argv=[
                str(bins.wind_agent),
                "subscriber",
                "--registry",
                registry_addr,
                "--duration-secs",
                str(duration_secs),
                *_subscriber_services_args(services),
            ],
            env=None,
            cwd=None,
            stdout_path=out_dir / "subscriber.stdout.log",
            stderr_path=out_dir / "subscriber.stderr.log",
        )
        subscriber.start()
        procs.append(subscriber)

        pids = [p.pid for p in procs if p.pid is not None]
        samples = collect_while(
            pids=[int(pid) for pid in pids],
            should_continue=lambda: subscriber.poll() is None,
            interval_s=1.0,
        )

        subscriber.wait(timeout_secs=duration_secs + 10)

        for pub in publishers:
            pub.terminate()

        sub_res = subscriber.result().json_summary or {}
        hist = Histogram.from_json(sub_res.get("latency_hist"))
        return {
            "subscriber": sub_res,
            "latency": summarize_hist(hist),
            "resources": summarize_samples(samples),
        }
    finally:
        terminate_all(reversed(procs))


def run_a4_once(
    *,
    bins: Binaries,
    registry_addr: str,
    services: list[str],
    subscribers: int,
    publishers_per_subscriber: int,
    payload_bytes: int,
    hz_per_publisher: float,
    duration_secs: int,
    seed: int,
    out_dir: Path,
) -> dict[str, Any]:
    procs: list[ManagedProcess] = []
    try:
        reg = ManagedProcess(
            argv=[str(bins.wind_registry), "--bind", registry_addr],
            env=None,
            cwd=None,
            stdout_path=out_dir / "registry.stdout.log",
            stderr_path=out_dir / "registry.stderr.log",
        )
        reg.start()
        procs.append(reg)
        _wait_for_registry(registry_addr)

        publishers: list[ManagedProcess] = []
        for i, svc in enumerate(services):
            pub = ManagedProcess(
                argv=[
                    str(bins.wind_agent),
                    "publisher",
                    "--service",
                    svc,
                    "--registry",
                    registry_addr,
                    "--duration-secs",
                    str(duration_secs),
                    "--mode",
                    "deterministic",
                    "--hz",
                    str(hz_per_publisher),
                    "--payload-bytes",
                    str(payload_bytes),
                    "--seed",
                    str(seed + i),
                ],
                env=None,
                cwd=None,
                stdout_path=out_dir / f"publisher-{i:04d}.stdout.log",
                stderr_path=out_dir / f"publisher-{i:04d}.stderr.log",
            )
            pub.start()
            publishers.append(pub)
            procs.append(pub)

        time.sleep(1.0)

        sub_procs: list[ManagedProcess] = []
        n_pubs = len(services)
        for i in range(subscribers):
            chosen = [services[(i + j) % n_pubs] for j in range(min(publishers_per_subscriber, n_pubs))]
            sub = ManagedProcess(
                argv=[
                    str(bins.wind_agent),
                    "subscriber",
                    "--registry",
                    registry_addr,
                    "--duration-secs",
                    str(duration_secs),
                    *_subscriber_services_args(chosen),
                ],
                env=None,
                cwd=None,
                stdout_path=out_dir / f"subscriber-{i:04d}.stdout.log",
                stderr_path=out_dir / f"subscriber-{i:04d}.stderr.log",
            )
            sub.start()
            sub_procs.append(sub)
            procs.append(sub)

        pids = [p.pid for p in procs if p.pid is not None]
        samples = collect_while(
            pids=[int(pid) for pid in pids],
            should_continue=lambda: any(p.poll() is None for p in sub_procs),
            interval_s=1.0,
        )

        for sub in sub_procs:
            sub.wait(timeout_secs=duration_secs + 10)

        for pub in publishers:
            pub.terminate()

        subs = [p.result().json_summary or {} for p in sub_procs]
        merged_hist = _merge_subscriber_hists(subs)
        return {
            "subscribers": subs,
            "latency": summarize_hist(merged_hist),
            "resources": summarize_samples(samples),
        }
    finally:
        terminate_all(reversed(procs))


def run_b1_once(
    *,
    bins: Binaries,
    registry_addr: str,
    service: str,
    lambda_hz: float,
    duration_secs: int,
    seed: int,
    out_dir: Path,
) -> dict[str, Any]:
    procs: list[ManagedProcess] = []
    try:
        reg = ManagedProcess(
            argv=[str(bins.wind_registry), "--bind", registry_addr],
            env=None,
            cwd=None,
            stdout_path=out_dir / "registry.stdout.log",
            stderr_path=out_dir / "registry.stderr.log",
        )
        reg.start()
        procs.append(reg)
        _wait_for_registry(registry_addr)

        publisher = ManagedProcess(
            argv=[
                str(bins.wind_agent),
                "publisher",
                "--service",
                service,
                "--registry",
                registry_addr,
                "--duration-secs",
                str(duration_secs),
                "--mode",
                "poisson",
                "--hz",
                str(lambda_hz),
                "--payload-profile",
                "iot",
                "--payload-bytes",
                "256",
                "--seed",
                str(seed),
            ],
            env=None,
            cwd=None,
            stdout_path=out_dir / "publisher.stdout.log",
            stderr_path=out_dir / "publisher.stderr.log",
        )
        publisher.start()
        procs.append(publisher)

        time.sleep(0.8)

        subscriber = ManagedProcess(
            argv=[
                str(bins.wind_agent),
                "subscriber",
                "--service",
                service,
                "--registry",
                registry_addr,
                "--duration-secs",
                str(duration_secs),
            ],
            env=None,
            cwd=None,
            stdout_path=out_dir / "subscriber.stdout.log",
            stderr_path=out_dir / "subscriber.stderr.log",
        )
        subscriber.start()
        procs.append(subscriber)

        pids = [p.pid for p in procs if p.pid is not None]
        samples = collect_while(
            pids=[int(pid) for pid in pids],
            should_continue=lambda: subscriber.poll() is None,
            interval_s=1.0,
        )

        subscriber.wait(timeout_secs=duration_secs + 10)
        publisher.terminate()

        sub_res = subscriber.result().json_summary or {}
        hist = Histogram.from_json(sub_res.get("latency_hist"))
        return {
            "subscriber": sub_res,
            "latency": summarize_hist(hist),
            "resources": summarize_samples(samples),
        }
    finally:
        terminate_all(reversed(procs))


def run_b2_once(
    *,
    bins: Binaries,
    registry_addr: str,
    services: list[str],
    subscribers: int,
    publishers_per_subscriber: int,
    lambda_hz_per_publisher: float,
    duration_secs: int,
    seed: int,
    out_dir: Path,
) -> dict[str, Any]:
    procs: list[ManagedProcess] = []
    try:
        reg = ManagedProcess(
            argv=[str(bins.wind_registry), "--bind", registry_addr],
            env=None,
            cwd=None,
            stdout_path=out_dir / "registry.stdout.log",
            stderr_path=out_dir / "registry.stderr.log",
        )
        reg.start()
        procs.append(reg)
        _wait_for_registry(registry_addr)

        publishers: list[ManagedProcess] = []
        for i, svc in enumerate(services):
            pub = ManagedProcess(
                argv=[
                    str(bins.wind_agent),
                    "publisher",
                    "--service",
                    svc,
                    "--registry",
                    registry_addr,
                    "--duration-secs",
                    str(duration_secs),
                    "--mode",
                    "poisson",
                    "--hz",
                    str(lambda_hz_per_publisher),
                    "--payload-profile",
                    "iot",
                    "--payload-bytes",
                    "256",
                    "--seed",
                    str(seed + i),
                ],
                env=None,
                cwd=None,
                stdout_path=out_dir / f"publisher-{i:04d}.stdout.log",
                stderr_path=out_dir / f"publisher-{i:04d}.stderr.log",
            )
            pub.start()
            publishers.append(pub)
            procs.append(pub)

        time.sleep(1.0)

        sub_procs: list[ManagedProcess] = []
        n_pubs = len(services)
        for i in range(subscribers):
            chosen = [services[(i + j) % n_pubs] for j in range(min(publishers_per_subscriber, n_pubs))]
            sub = ManagedProcess(
                argv=[
                    str(bins.wind_agent),
                    "subscriber",
                    "--registry",
                    registry_addr,
                    "--duration-secs",
                    str(duration_secs),
                    *_subscriber_services_args(chosen),
                ],
                env=None,
                cwd=None,
                stdout_path=out_dir / f"subscriber-{i:04d}.stdout.log",
                stderr_path=out_dir / f"subscriber-{i:04d}.stderr.log",
            )
            sub.start()
            sub_procs.append(sub)
            procs.append(sub)

        pids = [p.pid for p in procs if p.pid is not None]
        samples = collect_while(
            pids=[int(pid) for pid in pids],
            should_continue=lambda: any(p.poll() is None for p in sub_procs),
            interval_s=1.0,
        )

        for sub in sub_procs:
            sub.wait(timeout_secs=duration_secs + 10)

        for pub in publishers:
            pub.terminate()

        subs = [p.result().json_summary or {} for p in sub_procs]
        merged_hist = _merge_subscriber_hists(subs)
        return {
            "subscribers": subs,
            "latency": summarize_hist(merged_hist),
            "resources": summarize_samples(samples),
        }
    finally:
        terminate_all(reversed(procs))
