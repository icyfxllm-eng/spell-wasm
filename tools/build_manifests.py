#!/usr/bin/env python3
"""CC-PICTURE-BANK feature 7 — the picture manifest.

The manifest is the single source of truth for a picture: what it is, where
it came from, which traced paths belong to which layer, and how many words of
which band each layer needs in each shipped language. Everything else in
CC-PICTURE-BANK hangs off it, and so does CC-LEARNING-ENGINE, which cannot
start until words carry a hazard vector.

Two rules drive the generated content.

**Word difficulty maps to path prominence** (the file's first governing idea).
Layers are assigned by arc length: the longest strokes are the outline anyone
can finish, and the short ones are the detail only a strong speller earns. So
the layer split is mechanical and re-derivable, not hand-curated per picture.

**Difficulty monotonicity is law** (D5). Layer k draws from band k, and bands
are difficulty quartiles of that language's pool, so the climb is monotone by
construction. CI still VERIFIES it rather than trusting this script -- a
generator and its own checker agreeing proves nothing.

Run: python3 tools/build_manifests.py
"""
from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools/difficulty-score"))
from extractors import EXTRACTORS  # noqa: E402

SCANS = ROOT / "content-pipeline/wordpic/scans"
POOLS = ROOT / "content-pipeline/wordpic/pools"
PROV = ROOT / "content-pipeline/wordpic/ref/provenance-f8.json"
OUT = ROOT / "content-pipeline/wordpic/manifests"

# outline -> features -> texture -> shading. A picture declares only as many
# as it has distinct prominence strata; a starter piece is honestly one layer
# rather than four layers of one path each.
LAYERS = ["outline", "features", "texture", "shading"]

# Audio gating by tier (feature 5). Starter/intermediate keep Replay + Slow;
# advanced loses Slow; expert and masterpiece hear the word once.
AUDIO_GATE = {"easy": "starter", "medium": "intermediate", "hard": "advanced", "expert": "expert"}


def difficulty(word: str, lang: str) -> float:
    """Score a word for banding: length plus this language's hazard features.

    Deliberately the same signal tools/difficulty-score already uses, so the
    bank and the tier pipeline cannot drift into two notions of "hard". The
    hazard half is thin for several languages -- that is exactly the gap
    CC-LEARNING-ENGINE D2 assigns to native audit, and it is visible here
    rather than hidden behind a tuned constant.
    """
    hazards = EXTRACTORS.get(lang, lambda w: [])(word)
    return len(word) + 2.0 * len(hazards)


def hazard_vector(word: str, lang: str) -> list[str]:
    """The per-word hazard tags CC-LEARNING-ENGINE feature 1 consumes."""
    return sorted(EXTRACTORS.get(lang, lambda w: [])(word))


def bands_for(lang: str, tier: str) -> list[list[str]]:
    """Split a language's pool into four difficulty BANDS, easiest first.

    Bands are score RANGES, not equal-count quartiles. Two words of the same
    difficulty belong in the same band -- which sounds like a detail and is
    actually what makes D5 satisfiable: with equal-count quartiles a tie
    straddling a boundary puts the same maximum in two adjacent bands, and
    "max strictly increases per layer" fails on a picture that is genuinely
    getting harder. Splitting on distinct scores makes the ranges disjoint,
    so the climb is monotone by construction and CI is checking the content
    rather than an artefact of how it was sliced.

    Uneven band sizes are the honest consequence: a pool whose words cluster
    at one length has a fat band and thin neighbours, and the manifest gate
    will say so when a layer asks for more words than its band holds.
    """
    words: list[str] = []
    for t in (tier, "easy"):
        f = POOLS / f"{lang}-{t}.json"
        if f.exists():
            words += [w for w in json.loads(f.read_text()) if " " not in w]
    words = sorted(set(words), key=lambda w: (difficulty(w, lang), w))
    if not words:
        return [[], [], [], []]
    scores = sorted({difficulty(w, lang) for w in words})
    if len(scores) < 4:
        # Fewer than four distinct difficulties means this pool cannot express
        # a four-step climb at all. Report it as empty upper bands rather than
        # faking strata; the gate turns that into a legible failure.
        cuts = scores
    else:
        # Lower BOUNDS, one per band. Anchoring the top cut on the maximum
        # score (the obvious-looking scores[-1]) makes band 3 a single point
        # holding only the hardest word, which then cannot fill a layer --
        # the failure this gate reported before the fix.
        cuts = sorted({scores[k * len(scores) // 4] for k in range(4)})
    out: list[list[str]] = [[] for _ in range(4)]
    for w in words:
        d = difficulty(w, lang)
        # highest cut this word clears
        k = max((i for i, c in enumerate(cuts) if d >= c), default=0)
        out[min(k, 3)].append(w)
    return out


def layer_split(paths: list[dict]) -> list[list[int]]:
    """Group path indices into prominence strata, longest arcs first.

    Micro features (eyes, pupils) are always the last declared layer: they are
    the finest detail in the picture and the file's own example of what only a
    strong speller earns.
    """
    micro = [i for i, e in enumerate(paths) if e.get("micro_feature")]
    rest = sorted((i for i, e in enumerate(paths) if not e.get("micro_feature")),
                  key=lambda i: -paths[i]["arc"])
    if not rest:
        return [micro] if micro else []
    # Strata by arc length, but never more strata than paths -- a one-path
    # picture declares one layer, not four empty ones.
    want = min(len(LAYERS) - (1 if micro else 0), len(rest))
    per = len(rest) / want
    groups = [rest[round(k * per):round((k + 1) * per)] for k in range(want)]
    groups = [g for g in groups if g]
    return groups + ([micro] if micro else [])


def main() -> int:
    prov = {r["subject"]: r for r in json.loads(PROV.read_text())}
    langs = sorted({p.stem.split("-")[0] for p in POOLS.glob("*.json")})
    OUT.mkdir(parents=True, exist_ok=True)
    written = 0
    for f in sorted(SCANS.glob("*.json")):
        sub = f.stem
        doc = json.loads(f.read_text())
        tier = doc["tier"]
        groups = layer_split(doc["paths"])
        if not groups:
            print(f"{sub:10} SKIP (no paths)")
            continue

        p = prov.get(sub)
        if not p:
            print(f"{sub:10} NO PROVENANCE — refusing to emit a manifest")
            return 1

        layers = []
        for k, path_ids in enumerate(groups):
            band = min(k, 3)
            per_lang = {}
            for lang in langs:
                bands = bands_for(lang, tier)
                # One word per path in this layer -- the layer is complete when
                # every stroke in it has been spelled.
                per_lang[lang] = {"words": len(path_ids), "band": band,
                                  "band_size": len(bands[band])}
            layers.append({
                "name": LAYERS[k] if k < len(LAYERS) else f"layer{k}",
                "path_ids": path_ids,
                "per_language": per_lang,
            })

        manifest = {
            "id": sub,
            "tier": tier,
            "source": {
                "title": p["title"],
                "license": p["license"],
                "url": p.get("sourceUrl", ""),
                # PD-Art per CC-PICTURE-BANK: masterpiece cards print this.
                "attribution": p.get("attribution", ""),
            },
            "parts": {"root": [i for g in groups for i in g]},
            "layers": layers,
            "audio_gate": AUDIO_GATE.get(tier, "expert"),
            "completion": {"all_layers": True},
            "pin_hash": doc.get("pin_hash", ""),
        }
        (OUT / f"{sub}.json").write_text(json.dumps(manifest, indent=1) + "\n")
        written += 1
        print(f"{sub:10} tier={tier:7} layers={len(layers)} "
              f"paths={sum(len(l['path_ids']) for l in layers)}")
    print(f"\nwrote {written} manifests -> {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
