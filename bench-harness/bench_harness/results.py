from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class RunPaths:
    root: Path
    raw_dir: Path

    @staticmethod
    def create(base_dir: Path, run_id: str) -> "RunPaths":
        root = base_dir / run_id
        raw_dir = root / "raw"
        raw_dir.mkdir(parents=True, exist_ok=True)
        return RunPaths(root=root, raw_dir=raw_dir)

    def write_json(self, rel: str, obj: Any) -> Path:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(obj, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return path
