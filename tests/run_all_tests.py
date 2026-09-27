#!/usr/bin/env python3
"""shbt-power master test runner (paper/main.tex §8).

Pipeline:
  1. cargo test --workspace (Rust unit + integration suites)
  2. shbt-power-audit -> verification_matrix.json, assert 70/70 PASS
  3. PyO3 binding smoke test via pytest (if built)
"""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]


def run(cmd: list[str], **kw) -> int:
    print(f"\n=== {' '.join(cmd)} ===", flush=True)
    return subprocess.run(cmd, cwd=REPO_ROOT, **kw).returncode


def check_audit() -> int:
    report = json.loads((REPO_ROOT / "verification_matrix.json").read_text())
    print(f"audit: {report['passed']}/{report['total_gates']} PASS")
    for d in report.get("discrepancies", []):
        print(f"  discrepancy: {d}")
    return 0 if report["all_pass"] else 1


def test_pyo3() -> int:
    try:
        import shbt_power_py
    except ImportError:
        print("shbt_power_py not built — skipping PyO3 smoke test")
        return 0
    twin = shbt_power_py.PyShbtDigitalTwin()
    net = twin.run(200_000)
    assert twin.phase == 4, twin.phase
    assert abs(net - 7832.903) < 0.05, net
    print(f"PyO3 smoke test OK: net={net:.3f} MW, phase={twin.phase}")
    return 0


def main() -> int:
    rc = run(["cargo", "test", "--workspace"])
    if rc != 0:
        return rc
    rc = run(["cargo", "run", "--release", "-p", "shbt-power-audit"])
    if rc != 0:
        return rc
    rc = check_audit()
    if rc != 0:
        return rc
    return test_pyo3()


if __name__ == "__main__":
    sys.exit(main())
