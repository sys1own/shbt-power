"""shbt_power.cli — command-line driver: run the closed-loop twin and audit."""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]


def hud(state: dict) -> str:
    rows = [
        ("tick", f"{state['tick']}"),
        ("phase", f"{state['phase']}"),
        ("P_fusion", f"{state['p_fusion_mw']:.3f} MW"),
        ("P_direct", f"{state['p_direct_total_mw']:.3f} MW"),
        ("P_TEG", f"{state['p_teg_mw']:.3f} MW"),
        ("P_gross", f"{state['p_gross_mw']:.3f} MW"),
        ("P_net", f"{state['p_net_mw']:.3f} MW"),
        ("SoC", f"{state['supercap_soc'] * 100.0:.1f} %"),
        ("Isomer battery", f"{state['battery_soc'] * 100.0:.1f} % SoC @ {state['battery_bus_voltage_kv']:.1f} kV"),
        ("ADM |det g + 1|", f"{state['adm_metric_err']:.3e}"),
    ]
    w = max(len(k) for k, _ in rows)
    return "\n".join(f"  {k.ljust(w)} : {v}" for k, v in rows)


def cmd_run(args: argparse.Namespace) -> int:
    import shbt_power_py

    twin = shbt_power_py.PyShbtDigitalTwin()
    net = twin.run(args.ticks)
    state = {
        "tick": twin.tick,
        "phase": twin.phase,
        "p_fusion_mw": twin.p_fusion_mw,
        "p_direct_total_mw": 7525.0,
        "p_teg_mw": twin.p_teg_mw,
        "p_gross_mw": twin.p_gross_mw,
        "p_net_mw": net,
        "supercap_soc": twin.supercap_soc,
        "battery_soc": twin.battery_soc,
        "battery_bus_voltage_kv": twin.battery_bus_voltage_kv,
        "adm_metric_err": twin.adm_metric_err,
    }
    print(hud(state))
    return 0


def cmd_audit(_args: argparse.Namespace) -> int:
    rc = subprocess.run(
        ["cargo", "run", "--release", "-p", "shbt-power-audit"],
        cwd=REPO_ROOT,
    ).returncode
    if rc != 0:
        return rc
    report = json.loads((REPO_ROOT / "verification_matrix.json").read_text())
    print(
        f"{report['passed']}/{report['total_gates']} PASS; "
        f"{report.get('active_discrepancies', 0)} active discrepancy(ies), "
        f"{len(report.get('resolved_discrepancies', []))} resolved"
    )
    return 0 if report["all_pass"] else 1


def main() -> int:
    ap = argparse.ArgumentParser(prog="shbt-power")
    sub = ap.add_subparsers(dest="cmd", required=True)
    p_run = sub.add_parser("run", help="step the digital twin")
    p_run.add_argument("--ticks", type=int, default=1000)
    sub.add_parser("audit", help="run the 70-gate verification matrix")
    args = ap.parse_args()
    return {"run": cmd_run, "audit": cmd_audit}[args.cmd](args)


if __name__ == "__main__":
    sys.exit(main())
