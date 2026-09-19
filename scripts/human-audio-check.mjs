// CC-HUMAN-AUDIO build gate: every bundled human clip is provably the right one.
//
// The bundler (tools/human-audio/bundle.py) writes what ships; this re-checks it
// from the files alone, so a hand edit, a stale copy or a skipped step fails the
// build instead of reaching a child's ears.
//
// Laws:
//   1. every clip runtime.json names exists and its bytes hash to its name
//      (acceptance 12: hash-verified at build)
//   2. every shipped clip has an ACCEPT verdict in force, bound to the same
//      sha256 (I2, acceptance 8: verified-only)
//   3. every shipped clip's license is CC0, CC BY or CC BY-SA (D1, I4)
//   4. every shipped clip whose license requires attribution has a Credits
//      entry (F7, acceptance 2)
//   5. every shipped clip's recorded loudness is within tolerance of the
//      shared target (I7, acceptance 5 for human clips)
//   6. runtime.json and bundle.json agree, and no stray .m4a ships unlisted
//
//   node scripts/human-audio-check.mjs
//   node scripts/human-audio-check.mjs --selftest   # prove it bites
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const ROOT = path.dirname(new URL(import.meta.url).pathname) + "/..";
const ALLOWED = new Set(["CC0", "CC BY", "CC BY-SA"]);
const ATTRIBUTION = new Set(["CC BY", "CC BY-SA"]);

function check(dir, loudness) {
  const bad = [];
  const read = (p) => JSON.parse(fs.readFileSync(p, "utf8"));
  const rt = read(path.join(dir, "runtime.json"));
  const credits = read(path.join(dir, "credits.json"));
  const credited = new Set(credits.clips.map((c) => `${c.lang}\t${c.entry}\t${c.sha256}`));
  let n = 0;
  for (const [lang, l] of Object.entries(rt.langs || {})) {
    const ldir = path.join(dir, lang);
    const bundle = read(path.join(ldir, "bundle.json"));
    const verdicts = fs.existsSync(path.join(ldir, "verdicts.json")) ? read(path.join(ldir, "verdicts.json")) : { entries: {} };
    const byEntry = new Map(bundle.clips.map((c) => [c.entry, c]));
    if (l.base !== `human-audio/${lang}/`) bad.push(`${lang}: base ${l.base} is not human-audio/${lang}/`);
    for (const [entry, file] of Object.entries(l.clips)) {
      n++;
      const p = path.join(ldir, file);
      if (!fs.existsSync(p)) { bad.push(`${lang}/${entry}: ${file} is missing`); continue; }
      const sha = crypto.createHash("sha256").update(fs.readFileSync(p)).digest("hex");
      if (`${sha}.m4a` !== file) bad.push(`${lang}/${entry}: ${file} hashes to ${sha}`);
      const recs = verdicts.entries[entry] || [];
      const last = recs[recs.length - 1];
      if (!last || last.verdict !== "accept" || last.clip_sha256 !== sha)
        bad.push(`${lang}/${entry}: no accept verdict in force for these bytes (I2)`);
      const b = byEntry.get(entry);
      if (!b || b.file !== file) { bad.push(`${lang}/${entry}: runtime.json and bundle.json disagree`); continue; }
      if (!ALLOWED.has(b.license)) bad.push(`${lang}/${entry}: license ${b.license} not allowed (D1)`);
      if (ATTRIBUTION.has(b.license) && !credited.has(`${lang}\t${entry}\t${sha}`))
        bad.push(`${lang}/${entry}: ${b.license} clip has no Credits entry (F7)`);
      if (typeof b.final_lufs !== "number" || Math.abs(b.final_lufs - loudness.target_lufs) > loudness.tolerance_lu)
        bad.push(`${lang}/${entry}: loudness ${b.final_lufs} not within ±${loudness.tolerance_lu} of ${loudness.target_lufs} (I7)`);
    }
    if (bundle.clips.length !== Object.keys(l.clips).length) bad.push(`${lang}: bundle.json lists ${bundle.clips.length} clips, runtime.json ${Object.keys(l.clips).length}`);
    const listed = new Set(Object.values(l.clips));
    for (const f of fs.readdirSync(ldir)) if (f.endsWith(".m4a") && !listed.has(f)) bad.push(`${lang}: ${f} ships but is not listed`);
  }
  for (const [lang] of Object.entries(Object.fromEntries(fs.readdirSync(dir, { withFileTypes: true })
    .filter((d) => d.isDirectory()).map((d) => [d.name, 1]))))
    if (!(rt.langs || {})[lang] && fs.readdirSync(path.join(dir, lang)).some((f) => f.endsWith(".m4a")))
      bad.push(`${lang}: .m4a files ship but runtime.json lists no clips for ${lang}`);
  return { bad, n };
}

const loudness = JSON.parse(fs.readFileSync(`${ROOT}/config/audio-loudness.json`, "utf8"));

if (process.argv.includes("--selftest")) {
  // A gate that cannot fail reports success forever. Plant each violation.
  const clip = Buffer.from("fake m4a bytes");
  const sha = crypto.createHash("sha256").update(clip).digest("hex");
  const cases = {
    clean: {},
    "missing verdict (I2)": { verdict: null },
    "wrong bytes (hash)": { bytes: Buffer.from("other bytes") },
    "BY-SA without credit (F7)": { license: "CC BY-SA", credit: false },
    "NC license (D1)": { license: "CC BY-NC" },
    "too quiet (I7)": { lufs: loudness.target_lufs - 3 },
    "stray file": { stray: true },
  };
  let failed = 0;
  for (const [name, c] of Object.entries(cases)) {
    const d = fs.mkdtempSync(path.join(os.tmpdir(), "ha-check-"));
    fs.mkdirSync(path.join(d, "en"));
    fs.writeFileSync(path.join(d, "en", `${sha}.m4a`), c.bytes || clip);
    if (c.stray) fs.writeFileSync(path.join(d, "en", "stray.m4a"), clip);
    const license = c.license || "CC0";
    fs.writeFileSync(path.join(d, "runtime.json"), JSON.stringify({ langs: { en: { base: "human-audio/en/", clips: { apple: `${sha}.m4a` } } } }));
    fs.writeFileSync(path.join(d, "en", "bundle.json"), JSON.stringify({ clips: [{ entry: "apple", file: `${sha}.m4a`, license, final_lufs: c.lufs ?? loudness.target_lufs }] }));
    fs.writeFileSync(path.join(d, "en", "verdicts.json"), JSON.stringify({ entries: c.verdict === null ? {} : { apple: [{ verdict: "accept", clip_sha256: sha }] } }));
    const credit = ATTRIBUTION.has(license) && c.credit !== false;
    fs.writeFileSync(path.join(d, "credits.json"), JSON.stringify({ clips: credit ? [{ lang: "en", entry: "apple", sha256: sha }] : [] }));
    const { bad } = check(d, loudness);
    const want = name === "clean" ? 0 : 1;
    const ok = want === 0 ? bad.length === 0 : bad.length > 0;
    console.log(`  ${ok ? (want ? "caught " : "clean  ") : "MISSED "} ${name}${bad.length ? " — " + bad[0] : ""}`);
    if (!ok) failed++;
    fs.rmSync(d, { recursive: true });
  }
  if (failed) { console.error(`human-audio-check selftest: ${failed} case(s) wrong`); process.exit(1); }
  console.log("human-audio-check selftest: OK");
  process.exit(0);
}

const { bad, n } = check(`${ROOT}/assets/human-audio`, loudness);
if (bad.length) {
  console.error("human-audio-check: FAILED");
  for (const b of bad.slice(0, 40)) console.error("  " + b);
  process.exit(1);
}
console.log(`human-audio-check: OK — ${n} bundled clip(s), all verified, hashed, licensed, credited and on target`);
