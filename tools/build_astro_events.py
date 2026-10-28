#!/usr/bin/env python3
"""CC-THEMES-COSMOS F4: generate src/astro_events.rs, offline.

Computes every principal Moon phase (new, first quarter, full, last quarter)
from Dec 2025 to Jan 2041 with Jean Meeus, Astronomical Algorithms (2nd ed.,
1998), ch. 49, and classifies each full moon's umbral eclipse with ch. 54
(umbral magnitude >= 1.0 means total). This is our own computation of a
published algorithm; no third-party data is copied into the table.

Cross-check (the build FAILS on any disagreement): the computed total lunar
eclipses must equal, date for date, the totals in NASA GSFC's decade tables
(Espenak), saved offline in tests/fixtures/astro/nasa-lunar-eclipses-2026-2040.txt.
NASA eclipse data is a US government work (public domain); the pages say
"Permission is freely granted to reproduce this data when accompanied by an
acknowledgment."

No network access. Usage:
    python3 tools/build_astro_events.py           # write src/astro_events.rs
    python3 tools/build_astro_events.py --check   # fail if the file is stale
"""
import math
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "src", "astro_events.rs")
NASA = os.path.join(ROOT, "tests", "fixtures", "astro", "nasa-lunar-eclipses-2026-2040.txt")
FIRST_YEAR, LAST_YEAR = 2026, 2040


def sind(x):
    return math.sin(math.radians(x))


def cosd(x):
    return math.cos(math.radians(x))


def delta_t_seconds(year):
    # Espenak & Meeus polynomial for 2005-2050 (NASA GSFC "Polynomial Expressions for Delta T").
    t = year - 2000
    return 62.92 + 0.32217 * t + 0.005589 * t * t


def args(k):
    T = k / 1236.85
    E = 1 - 0.002516 * T - 0.0000074 * T * T
    M = 2.5534 + 29.10535670 * k - 0.0000014 * T**2 - 0.00000011 * T**3
    Mp = 201.5643 + 385.81693528 * k + 0.0107582 * T**2 + 0.00001238 * T**3 - 0.000000058 * T**4
    F = 160.7108 + 390.67050284 * k - 0.0016118 * T**2 - 0.00000227 * T**3 + 0.000000011 * T**4
    Om = 124.7746 - 1.56375588 * k + 0.0020672 * T**2 + 0.00000215 * T**3
    return T, E, M, Mp, F, Om


def phase_jde(k):
    """Meeus ch. 49: JDE (TT) of the phase with lunation number k (k + 0, .25, .5, .75)."""
    T, E, M, Mp, F, Om = args(k)
    jde = 2451550.09766 + 29.530588861 * k + 0.00015437 * T**2 - 0.000000150 * T**3 + 0.00000000073 * T**4
    frac = round((k - math.floor(k)) * 4) % 4
    if frac in (0, 2):
        c = (-0.40720, 0.17241, 0.01608, 0.01039, 0.00739, -0.00514, 0.00208) if frac == 0 else \
            (-0.40614, 0.17302, 0.01614, 0.01043, 0.00734, -0.00515, 0.00209)
        corr = (c[0] * sind(Mp) + c[1] * E * sind(M) + c[2] * sind(2 * Mp) + c[3] * sind(2 * F)
                + c[4] * E * sind(Mp - M) + c[5] * E * sind(Mp + M) + c[6] * E * E * sind(2 * M)
                - 0.00111 * sind(Mp - 2 * F) - 0.00057 * sind(Mp + 2 * F) + 0.00056 * E * sind(2 * Mp + M)
                - 0.00042 * sind(3 * Mp) + 0.00042 * E * sind(M + 2 * F) + 0.00038 * E * sind(M - 2 * F)
                - 0.00024 * E * sind(2 * Mp - M) - 0.00017 * sind(Om) - 0.00007 * sind(Mp + 2 * M)
                + 0.00004 * sind(2 * Mp - 2 * F) + 0.00004 * sind(3 * M) + 0.00003 * sind(Mp + M - 2 * F)
                + 0.00003 * sind(2 * Mp + 2 * F) - 0.00003 * sind(Mp + M + 2 * F) + 0.00003 * sind(Mp - M + 2 * F)
                - 0.00002 * sind(Mp - M - 2 * F) - 0.00002 * sind(3 * Mp + M) + 0.00002 * sind(4 * Mp))
    else:
        corr = (-0.62801 * sind(Mp) + 0.17172 * E * sind(M) - 0.01183 * E * sind(Mp + M) + 0.00862 * sind(2 * Mp)
                + 0.00804 * sind(2 * F) + 0.00454 * E * sind(Mp - M) + 0.00204 * E * E * sind(2 * M)
                - 0.00180 * sind(Mp - 2 * F) - 0.00070 * sind(Mp + 2 * F) - 0.00040 * sind(3 * Mp)
                - 0.00034 * E * sind(2 * Mp - M) + 0.00032 * E * sind(M + 2 * F) + 0.00032 * E * sind(M - 2 * F)
                - 0.00028 * E * E * sind(Mp + 2 * M) + 0.00027 * E * sind(2 * Mp + M) - 0.00017 * sind(Om)
                - 0.00005 * sind(Mp - M - 2 * F) + 0.00004 * sind(2 * Mp + 2 * F) - 0.00004 * sind(Mp + M + 2 * F)
                + 0.00004 * sind(Mp - 2 * M) + 0.00003 * sind(Mp + M - 2 * F) + 0.00003 * sind(3 * M)
                + 0.00002 * sind(2 * Mp - 2 * F) + 0.00002 * sind(Mp - M + 2 * F) - 0.00002 * sind(3 * Mp + M))
        W = (0.00306 - 0.00038 * E * cosd(M) + 0.00026 * cosd(Mp) - 0.00002 * cosd(Mp - M)
             + 0.00002 * cosd(Mp + M) + 0.00002 * cosd(2 * F))
        corr += W if frac == 1 else -W
    a = [299.77 + 0.107408 * k - 0.009173 * T * T, 251.88 + 0.016321 * k, 251.83 + 26.651886 * k,
         349.42 + 36.412478 * k, 84.66 + 18.206239 * k, 141.74 + 53.303771 * k, 207.14 + 2.453732 * k,
         154.84 + 7.306860 * k, 34.52 + 27.261239 * k, 207.19 + 0.121824 * k, 291.34 + 1.844379 * k,
         161.72 + 24.198154 * k, 239.56 + 25.513099 * k, 331.55 + 3.592518 * k]
    w = [0.000325, 0.000165, 0.000164, 0.000126, 0.000110, 0.000062, 0.000060, 0.000056, 0.000047,
         0.000042, 0.000040, 0.000037, 0.000035, 0.000023]
    corr += sum(wi * sind(ai) for wi, ai in zip(w, a))
    return jde + corr


def umbral_magnitude(k):
    """Meeus ch. 54 for a full moon (k = integer + 0.5): umbral magnitude, or None if no eclipse is possible."""
    T, E, M, Mp, F, Om = args(k)
    if abs(sind(F)) > 0.36:
        return None
    F1 = F - 0.02665 * sind(Om)
    P = (0.2070 * E * sind(M) + 0.0024 * E * sind(2 * M) - 0.0392 * sind(Mp) + 0.0116 * sind(2 * Mp)
         - 0.0073 * E * sind(Mp + M) + 0.0067 * E * sind(Mp - M) + 0.0118 * sind(2 * F1))
    Q = (5.2207 - 0.0048 * E * cosd(M) + 0.0020 * E * cosd(2 * M) - 0.3299 * cosd(Mp)
         - 0.0060 * E * cosd(Mp + M) + 0.0041 * E * cosd(Mp - M))
    W = abs(cosd(F1))
    gamma = (P * cosd(F1) + Q * sind(F1)) * (1 - 0.0048 * W)
    u = 0.0059 + 0.0046 * E * cosd(M) - 0.0182 * cosd(Mp) + 0.0004 * cosd(2 * Mp) - 0.0005 * cosd(M + Mp)
    return (1.0128 - u - abs(gamma)) / 0.5450


def jd_to_unix_minutes(jde):
    # TT -> UT with an approximate Delta T (seconds; irrelevant at day resolution).
    year = 2000 + (jde - 2451545.0) / 365.25
    jd_ut = jde - delta_t_seconds(year) / 86400.0
    return int(round((jd_ut - 2440587.5) * 1440))


def civil(unix_minutes):
    days = unix_minutes // 1440
    # Howard Hinnant's civil_from_days.
    z = days + 719468
    era = z // 146097
    doe = z - era * 146097
    yoe = (doe - doe // 1460 + doe // 36524 - doe // 146096) // 365
    y = yoe + era * 400
    doy = doe - (365 * yoe + yoe // 4 - yoe // 100)
    mp = (5 * doy + 2) // 153
    d = doy - (153 * mp + 2) // 5 + 1
    m = mp + 3 if mp < 10 else mp - 9
    return (y + (1 if m <= 2 else 0), m, d)


def build():
    # k = -1.. covers Dec 2025; run until Jan 2041 so windows at both table edges have neighbours.
    k0 = math.floor((2025.9 - 2000) * 12.3685)
    kinds = {0: [], 1: [], 2: [], 3: []}
    totals = []
    k = k0
    while True:
        new = jd_to_unix_minutes(phase_jde(k))
        if civil(new) > (LAST_YEAR + 1, 1, 31):
            break
        for q in range(4):
            mins = jd_to_unix_minutes(phase_jde(k + q / 4))
            if (FIRST_YEAR - 1, 12, 1) <= civil(mins) <= (LAST_YEAR + 1, 1, 31):
                kinds[q].append(mins)
                if q == 2:
                    mag = umbral_magnitude(k + 0.5)
                    y, m, d = civil(mins)
                    if mag is not None and mag >= 1.0 and FIRST_YEAR <= y <= LAST_YEAR:
                        totals.append(((y, m, d), mag))
        k += 1
    for q in kinds:
        kinds[q].sort()
    return kinds, totals


def nasa_totals():
    out = []
    for line in open(NASA, encoding="utf-8"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        date, kind = line.split()
        if kind == "Total":
            y, m, d = (int(x) for x in date.split("-"))
            out.append((y, m, d))
    return out


def render(kinds, totals):
    fulls = kinds[2]
    for y in range(FIRST_YEAR, LAST_YEAR + 1):
        n = sum(1 for x in fulls if civil(x)[0] == y)
        if n < 12:
            sys.exit(f"build_astro_events: only {n} full moons in {y}")
    april = [civil(x) for x in fulls if civil(x)[1] == 4 and FIRST_YEAR <= civil(x)[0] <= LAST_YEAR]

    def mins(name, xs, what):
        body = "\n".join(f"    {x}, // {'%04d-%02d-%02d' % civil(x)} {x % 1440 // 60:02d}:{x % 60:02d} UTC" for x in xs)
        return f"/// {what}: minutes since 1970-01-01 00:00 UTC.\npub const {name}: &[i64] = &[\n{body}\n];\n"

    def dates(name, xs, what, notes=None):
        body = "\n".join(f"    ({y}, {m}, {d}),{'' if notes is None else ' // umbral magnitude %.3f' % notes[i]}"
                         for i, (y, m, d) in enumerate(xs))
        return f"/// {what}\npub const {name}: &[(i32, u32, u32)] = &[\n{body}\n];\n"

    header = f"""// GENERATED by tools/build_astro_events.py -- do not edit by hand.
// Regenerate: python3 tools/build_astro_events.py   (verify: --check)
//
// Algorithm source: Jean Meeus, Astronomical Algorithms, 2nd ed. (Willmann-Bell, 1998),
//   ch. 49 (phases of the Moon: mean phase + periodic and planetary corrections) and
//   ch. 54 (eclipses: umbral magnitude >= 1.0 is total). An algorithm, not data:
//   every instant below is our own computation. TT -> UT via the Espenak-Meeus Delta T polynomial.
// Cross-check source: NASA GSFC lunar eclipse decade tables, Fred Espenak
//   (eclipse.gsfc.nasa.gov/LEdecade/LEdecade2021.html, LEdecade2031.html), saved in
//   tests/fixtures/astro/nasa-lunar-eclipses-2026-2040.txt. A US government work (public
//   domain); the pages state "Permission is freely granted to reproduce this data when
//   accompanied by an acknowledgment." Eclipse Predictions by Fred Espenak, NASA's GSFC.
//   The generator fails unless its computed totals match NASA's totals exactly.
// Licence: redistribution permitted (own computation; cross-check data is public domain).
// Coverage: {FIRST_YEAR}-{LAST_YEAR} inclusive, plus Dec {FIRST_YEAR - 1} / Jan {LAST_YEAR + 1} neighbours.

pub const EVENTS_FIRST_YEAR: i32 = {FIRST_YEAR};
pub const EVENTS_LAST_YEAR: i32 = {LAST_YEAR};

"""
    return (header
            + mins("NEW_MOONS", kinds[0], "New moon instants") + "\n"
            + mins("FIRST_QUARTERS", kinds[1], "First quarter instants") + "\n"
            + mins("FULL_MOONS", fulls, "Full moon instants") + "\n"
            + mins("LAST_QUARTERS", kinds[3], "Last quarter instants") + "\n"
            + dates("APRIL_FULL_MOONS", april, "April full moons (UTC dates).") + "\n"
            + dates("TOTAL_LUNAR_ECLIPSES", [t[0] for t in totals],
                    "Total lunar eclipses (UTC date of the full moon); matches NASA GSFC.", [t[1] for t in totals]))


def main():
    kinds, totals = build()
    ours = [t[0] for t in totals]
    nasa = nasa_totals()
    if ours != nasa:
        sys.exit(f"build_astro_events: FAILED NASA cross-check\n  computed totals: {ours}\n  NASA totals:     {nasa}")
    text = render(kinds, totals)
    if "--check" in sys.argv:
        cur = open(OUT, encoding="utf-8").read() if os.path.exists(OUT) else ""
        if cur != text:
            sys.exit("build_astro_events: src/astro_events.rs is stale; rerun the generator")
        print("build_astro_events: --check OK")
        return
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"build_astro_events: wrote {os.path.relpath(OUT, ROOT)} "
          f"({len(kinds[2])} full moons, {len(ours)} total lunar eclipses, NASA cross-check OK)")


if __name__ == "__main__":
    main()
