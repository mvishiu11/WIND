from __future__ import annotations

import os
import time
from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class ProcSample:
    t_s: float
    rss_kb: int | None
    utime_ticks: int | None
    stime_ticks: int | None


def _read_file(path: str) -> str | None:
    try:
        with open(path, "r", encoding="utf-8") as f:
            return f.read()
    except OSError:
        return None


def sample_proc(pid: int) -> ProcSample:
    t_s = time.monotonic()

    rss_kb: int | None = None
    status = _read_file(f"/proc/{pid}/status")
    if status:
        for line in status.splitlines():
            if line.startswith("VmRSS:"):
                parts = line.split()
                if len(parts) >= 2:
                    try:
                        rss_kb = int(parts[1])
                    except ValueError:
                        rss_kb = None
                break

    utime_ticks: int | None = None
    stime_ticks: int | None = None
    stat = _read_file(f"/proc/{pid}/stat")
    if stat:
        parts = stat.split()
        if len(parts) > 15:
            try:
                utime_ticks = int(parts[13])
                stime_ticks = int(parts[14])
            except ValueError:
                utime_ticks = None
                stime_ticks = None

    return ProcSample(t_s=t_s, rss_kb=rss_kb, utime_ticks=utime_ticks, stime_ticks=stime_ticks)


def collect_while(
    *,
    pids: list[int],
    should_continue: callable[[], bool],
    interval_s: float = 0.5,
) -> dict[int, list[dict[str, Any]]]:
    out: dict[int, list[dict[str, Any]]] = {pid: [] for pid in pids}

    while should_continue():
        for pid in pids:
            s = sample_proc(pid)
            out[pid].append(
                {
                    "t_s": s.t_s,
                    "rss_kb": s.rss_kb,
                    "utime_ticks": s.utime_ticks,
                    "stime_ticks": s.stime_ticks,
                }
            )
        time.sleep(interval_s)

    return out


def summarize_samples(samples: dict[int, list[dict[str, Any]]]) -> dict[str, Any]:
    clk_tck = os.sysconf(os.sysconf_names.get("SC_CLK_TCK", "SC_CLK_TCK"))
    summary: dict[str, Any] = {"clk_tck": clk_tck, "processes": {}}

    for pid, rows in samples.items():
        max_rss = None
        for r in rows:
            v = r.get("rss_kb")
            if isinstance(v, int):
                max_rss = v if max_rss is None else max(max_rss, v)

        summary["processes"][str(pid)] = {
            "max_rss_kb": max_rss,
            "samples": rows,
        }

    return summary
