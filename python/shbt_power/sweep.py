"""shbt_power.sweep — five-regime parametric efficiency sensitivity analysis
(paper/main.tex §6, tab:sensitivity_analysis)."""

from __future__ import annotations

# (Q_fusion, eta_graser, eta_conv)
REGIMES = [
    ("Sub-Nominal", 150.0, 0.184, 0.800),
    ("Conservative", 250.0, 0.184, 0.825),
    ("Baseline", 350.0, 0.200, 0.860),
    ("Advanced High-Burn", 450.0, 0.221, 0.880),
    ("Theoretical Limit", 500.0, 0.221, 0.900),
]

P_BEAM_MW = 25.0
P_AUX_MW = 15.0
ETA_TEG = 0.33804


def evaluate(q_fusion: float, eta_graser: float, eta_conv: float) -> dict:
    p_fus = P_BEAM_MW * q_fusion
    direct = p_fus * eta_conv
    p_elec = P_BEAM_MW / eta_graser
    thermal_in = p_fus * (1.0 - eta_conv) + (p_elec - P_BEAM_MW)
    p_teg = thermal_in * ETA_TEG
    gross = direct + p_teg
    recirc = p_elec + P_AUX_MW
    net = gross - recirc
    return {
        "p_fusion_mw": p_fus,
        "p_direct_mw": direct,
        "p_teg_mw": p_teg,
        "p_gross_mw": gross,
        "p_recirc_mw": recirc,
        "p_net_mw": net,
        "eta_net": net / p_fus,
        "q_eng": direct / recirc,
        "q_eng_total": gross / recirc,
    }


def main() -> None:
    print(f"{'Regime':<20} {'P_net MW':>10} {'eta_net %':>9} {'Q_eng':>7} {'Q_eng_tot':>9}")
    for name, q, eg, ec in REGIMES:
        r = evaluate(q, eg, ec)
        print(f"{name:<20} {r['p_net_mw']:>10.2f} {r['eta_net']*100:>9.2f} "
              f"{r['q_eng']:>7.2f} {r['q_eng_total']:>9.2f}")


if __name__ == "__main__":
    main()
