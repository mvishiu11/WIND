from __future__ import annotations

import json
import os
import signal
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


@dataclass(frozen=True)
class ProcResult:
    returncode: int
    json_summary: dict[str, Any] | None


def _read_json_summary(stdout_path: Path) -> dict[str, Any] | None:
    if not stdout_path.exists():
        return None

    last_obj: dict[str, Any] | None = None
    for line in stdout_path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(obj, dict):
            last_obj = obj
    return last_obj


class ManagedProcess:
    def __init__(
        self,
        *,
        argv: list[str],
        env: dict[str, str] | None,
        cwd: Path | None,
        stdout_path: Path,
        stderr_path: Path,
    ) -> None:
        self.argv = argv
        self.env = env
        self.cwd = cwd
        self.stdout_path = stdout_path
        self.stderr_path = stderr_path
        self._proc: subprocess.Popen[str] | None = None

    @property
    def pid(self) -> int | None:
        return None if self._proc is None else self._proc.pid

    def start(self) -> None:
        self.stdout_path.parent.mkdir(parents=True, exist_ok=True)
        self.stderr_path.parent.mkdir(parents=True, exist_ok=True)

        stdout_f = self.stdout_path.open("w", encoding="utf-8")
        stderr_f = self.stderr_path.open("w", encoding="utf-8")

        merged_env = os.environ.copy()
        if self.env:
            merged_env.update(self.env)

        self._proc = subprocess.Popen(
            self.argv,
            cwd=str(self.cwd) if self.cwd else None,
            env=merged_env,
            stdout=stdout_f,
            stderr=stderr_f,
            text=True,
        )

    def poll(self) -> int | None:
        if not self._proc:
            return None
        return self._proc.poll()

    def terminate(self, *, timeout_secs: float = 5.0) -> None:
        if not self._proc:
            return

        if self._proc.poll() is not None:
            return

        try:
            os.kill(self._proc.pid, signal.SIGTERM)
        except ProcessLookupError:
            return

        deadline = time.time() + timeout_secs
        while time.time() < deadline:
            if self._proc.poll() is not None:
                return
            time.sleep(0.05)

        try:
            os.kill(self._proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            return

    def wait(self, *, timeout_secs: float | None = None) -> int:
        if not self._proc:
            raise RuntimeError("process not started")
        return self._proc.wait(timeout=timeout_secs)

    def result(self) -> ProcResult:
        rc = -1
        if self._proc is not None and self._proc.poll() is not None:
            rc = int(self._proc.returncode or 0)

        summary = _read_json_summary(self.stdout_path)
        return ProcResult(returncode=rc, json_summary=summary)


def terminate_all(processes: Iterable[ManagedProcess]) -> None:
    for proc in processes:
        proc.terminate()
