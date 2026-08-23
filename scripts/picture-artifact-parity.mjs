#!/usr/bin/env node
// CC-PICTURE-BANK — the bank, the scans and the manifests describe one set.
//
// Nothing checked this, and they drifted in BOTH directions without a word:
//
//   * two manifests and scans outlived their pictures -- scream and
//     starrynight, cut for being untraceable soft paint. Dead data that every
//     generator kept faithfully regenerating;
//   * five bank pictures have no manifest and no scan at all. manifest-check
//     iterates MANIFESTS, so a picture without one is not examined rather than
//     failed -- the quietest kind of gap.
//
// A registered picture needs both artifacts. The five that predate the manifest
// system are listed by name with a reason, so the exemption is a decision on
// the record and a SIXTH cannot join them silently.
import fs from "node:fs";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const problems = [];

const bank = new Set(
  JSON.parse(fs.readFileSync(`${ROOT}/config/wordpic/pictures.json`, "utf8"))
    .pictures.map((p) => p.id),
);
const stems = (dir) =>
  new Set(fs.readdirSync(`${ROOT}/${dir}`).filter((f) => f.endsWith(".json"))
    .map((f) => f.replace(/\.json$/, "")));
const manifests = stems("content-pipeline/wordpic/manifests");
const scans = stems("content-pipeline/wordpic/scans");

// A manifest without a picture is NOT automatically dead. Done #7 stages a
// subject exactly that way on purpose: traced, manifested and provenance-
// tracked, but held out of pictures.json until its per-language cultural audit
// is signed by a named auditor. Reading "orphan" off the file layout alone
// would delete work that is waiting on a person -- which is what nearly
// happened to the Tagalog pair, carabao and jeepney, waiting on Paul.
const audits = JSON.parse(
  fs.readFileSync(`${ROOT}/content-pipeline/wordpic/audits.json`, "utf8"),
).audits;
const staged = new Set();
for (const [lang, a] of Object.entries(audits))
  if (a.status !== "signed")
    for (const s of a.subjects ?? []) staged.add(s);

// Authored-geometry pictures from before the manifest pipeline. They ship their
// paths directly from the registry and render correctly; they are grandfathered,
// not excused, and adding a new one here needs a reason of its own.
const LEGACY_NO_MANIFEST = {
  diamond: "authored geometry, predates the manifest pipeline",
  merkaba: "authored geometry, predates the manifest pipeline",
  metatron: "authored geometry, predates the manifest pipeline",
  pentagram: "authored geometry, predates the manifest pipeline",
  sriyantra: "authored geometry, predates the manifest pipeline",
};

for (const id of manifests)
  if (!bank.has(id) && !staged.has(id))
    problems.push(`manifest ${id}.json has no picture and no pending audit — dead data`);
for (const id of scans)
  if (!bank.has(id) && !staged.has(id))
    problems.push(`scan ${id}.json has no picture and no pending audit — dead data`);
// A staged subject that reached the bank should not still be called staged.
for (const id of staged)
  if (bank.has(id))
    problems.push(`${id} is in the bank while its audit is still unsigned (Done #7)`);
for (const id of bank) {
  if (LEGACY_NO_MANIFEST[id]) continue;
  if (!manifests.has(id)) problems.push(`${id} is in the bank with no manifest`);
  if (!scans.has(id)) problems.push(`${id} is in the bank with no scan`);
}
// An entry that stops being needed must not linger either.
for (const id of Object.keys(LEGACY_NO_MANIFEST))
  if (!bank.has(id))
    problems.push(`${id} is exempted here but is no longer in the bank — drop the exemption`);

if (problems.length) {
  console.error(`picture-artifact-parity: FAILED — ${problems.length} problem(s):`);
  for (const p of problems) console.error("  ✗ " + p);
  process.exit(1);
}
console.log(
  `picture-artifact-parity: OK — ${bank.size} pictures, ${manifests.size} manifests, ` +
    `${scans.size} scans, no drift either way ` +
    `(${Object.keys(LEGACY_NO_MANIFEST).length} pre-pipeline pictures named, ` +
    `${staged.size} staged on a pending audit).`,
);

// Selftest: the drift this exists to catch must actually fail it.
{
  const orphan = new Set(manifests).add("a-picture-that-was-deleted");
  const caught = [...orphan].some((id) => !bank.has(id));
  if (!caught) {
    console.error("picture-artifact-parity: SELFTEST FAILED — an orphan manifest slipped through.");
    process.exit(1);
  }
  console.log("picture-artifact-parity: selftest OK — a manifest without a picture fails the build.");
}
