#!/usr/bin/env node
// CC-WORD-PICTURE v6 — manifest schema + provenance + bundle-scan gate.
// The geometry law (L9 sweep: overlaps/fill/floor/frame across 15 languages
// × 5 seeds) lives in `cargo test --lib wordpic_layout` — this script covers
// the data-side invariants that don't need the solver.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const BANDS = { easy: [1, 8], medium: [1, 20], hard: [1, 45], expert: [1, 200] };
const ARCHES = new Set(["stack", "arc", "spiral", "zigzag", "wave", "radial", "outline", "line"]);
let problems = [];
const m = JSON.parse(fs.readFileSync(`${ROOT}/config/wordpic/pictures.json`, "utf8"));

const byPack = {};
for (const p of m.pictures) (byPack[p.pack ?? "starter"] ??= []).push(p);
if ((byPack.starter ?? []).length !== 10)
  problems.push(`starter pack must be exactly 10 (got ${(byPack.starter ?? []).length})`);
for (const [pk, ps] of Object.entries(byPack))
  if (ps.length > 10) problems.push(`pack '${pk}' exceeds the 10-picture cap (D13): ${ps.length}`);
const seen = new Set();
for (const p of m.pictures) {
  const [lo, hi] = BANDS[p.tier] ?? [0, 0];
  if (p.paths.length < lo || p.paths.length > hi)
    problems.push(`${p.id}: ${p.paths.length} paths outside ${p.tier} band ${lo}-${hi}`);
  const key = `${p.tier}:${p.subject}`;
  if (seen.has(key)) problems.push(`${p.id}: duplicate subject '${p.subject}' at tier ${p.tier} (D11)`);
  seen.add(key);
  const BANNED_FEAT = new Set(["decorative", "filler", "background texture"]);
  for (const q of p.paths) {
    if (!q.feature || !q.feature.trim())
      problems.push(`${p.id}: unlabeled stroke (v7 F2 — name the feature or delete it)`);
    else if (BANNED_FEAT.has(q.feature.toLowerCase()))
      problems.push(`${p.id}: banned feature label '${q.feature}' (v7 F2)`);
    if (!ARCHES.has(q.arch)) problems.push(`${p.id}: unknown archetype '${q.arch}'`);
    if (!(q.band >= 1 && q.band <= 4)) problems.push(`${p.id}: band out of range`);
    if (!(q.budget[0] >= 2 && q.budget[1] >= q.budget[0])) problems.push(`${p.id}: bad budget`);
    if (q.mode === "flow" && !q.d) problems.push(`${p.id}: flow path missing geometry`);
    if (q.mode === "stack" && !(q.size > 0)) problems.push(`${p.id}: stack missing size`);
  }
  // v7.4 required-features: subjects may declare interior features their
  // trace MUST contain — missing one fails lint, not review.
  if (p.requiredFeatures) {
    const have = new Set(p.paths.map((q) => (q.feature || "").toLowerCase()));
    for (const rf of p.requiredFeatures)
      if (!have.has(rf.toLowerCase()))
        problems.push(`${p.id}: required feature '${rf}' missing from trace (v7.4)`);
  }
  if (p.tier === "expert") {
    const pr = p.provenance ?? {};
    for (const k of ["title", "artist", "source", "sourceUrl", "pdBasis", "retrieved"])
      if (!pr[k]) problems.push(`${p.id}: provenance missing '${k}' (I4)`);
  }
  // Archetype lint (D12, soft): warn when one archetype dominates a picture.
  const counts = {};
  for (const q of p.paths) counts[q.arch] = (counts[q.arch] ?? 0) + 1;
  const top = Object.entries(counts).sort((a, b) => b[1] - a[1])[0];
  if (top && top[1] / p.paths.length > 0.75 && p.paths.length >= 8)
    console.warn(`  lint: ${p.id} is ${Math.round((top[1] / p.paths.length) * 100)}% '${top[0]}' strokes`);
}
// I4 bundle scan: the PD reference must never reach the shipped bundle.
for (const dir of ["dist", "ios/App/App/public"]) {
  const full = `${ROOT}/${dir}`;
  if (fs.existsSync(full)) {
    const stack = [full];
    while (stack.length) {
      const d = stack.pop();
      for (const e of fs.readdirSync(d, { withFileTypes: true })) {
        const fp = `${d}/${e.name}`;
        if (e.isDirectory()) stack.push(fp);
        else if (/mona-lisa\.(jpg|jpeg|png)$/i.test(e.name))
          problems.push(`PD reference in shipped bundle: ${fp} (I4)`);
      }
    }
  }
}
if (problems.length) {
  console.error(`wordpic-check: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error("  ✗ " + p);
  process.exit(1);
}
console.log(`wordpic-check: OK — ${m.pictures.length} pictures, schema/provenance/bundle green (geometry law = cargo test wordpic_layout).`);
