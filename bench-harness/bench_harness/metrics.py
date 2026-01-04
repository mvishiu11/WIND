from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class Histogram:
    # list of [latency_us, count]
    counts: list[tuple[int, int]]

    @staticmethod
    def from_json(obj: Any) -> "Histogram":
        counts: list[tuple[int, int]] = []
        for item in obj or []:
            if not isinstance(item, list) or len(item) != 2:
                continue
            v, c = item
            if not isinstance(v, int) or not isinstance(c, int):
                continue
            counts.append((v, c))
        counts.sort(key=lambda x: x[0])
        return Histogram(counts=counts)

    def merge(self, other: "Histogram") -> "Histogram":
        merged: dict[int, int] = {}
        for v, c in self.counts:
            merged[v] = merged.get(v, 0) + c
        for v, c in other.counts:
            merged[v] = merged.get(v, 0) + c
        return Histogram(counts=sorted(merged.items(), key=lambda x: x[0]))

    def total(self) -> int:
        return sum(c for _, c in self.counts)

    def min(self) -> int | None:
        return None if not self.counts else self.counts[0][0]

    def max(self) -> int | None:
        return None if not self.counts else self.counts[-1][0]

    def value_at_quantile(self, q: float) -> int | None:
        if not self.counts:
            return None
        if q <= 0.0:
            return self.counts[0][0]
        if q >= 1.0:
            return self.counts[-1][0]

        target = int(self.total() * q)
        if target <= 0:
            target = 1

        running = 0
        for v, c in self.counts:
            running += c
            if running >= target:
                return v
        return self.counts[-1][0]


def summarize_hist(hist: Histogram) -> dict[str, Any]:
    return {
        "count": hist.total(),
        "min_us": hist.min(),
        "p50_us": hist.value_at_quantile(0.50),
        "p90_us": hist.value_at_quantile(0.90),
        "p95_us": hist.value_at_quantile(0.95),
        "p99_us": hist.value_at_quantile(0.99),
        "p999_us": hist.value_at_quantile(0.999),
        "max_us": hist.max(),
    }
