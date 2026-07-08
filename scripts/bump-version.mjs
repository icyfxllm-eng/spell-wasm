#!/usr/bin/env node
// Single source of truth for the app version. Sets `versionName` (the
// human "X.Y.Z") everywhere it must match and bumps the integer
// `versionCode` (Android requires it to strictly increase every upload).
//
//   node scripts/bump-version.mjs 1.2.0          # set version, +1 versionCode
//   node scripts/bump-version.mjs 1.2.0 --dry-run
//
// Keeps in lockstep: package.json "version", android versionName/versionCode.
// (iOS build number is bumped here too once the ios/ project exists.)
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const version = process.argv[2];
const dryRun = process.argv.includes("--dry-run");

if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("usage: node scripts/bump-version.mjs <X.Y.Z> [--dry-run]");
  process.exit(1);
}

const changes = [];

// package.json
const pkgPath = join(root, "package.json");
const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
changes.push([pkgPath, () => {
  pkg.version = version;
  writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
}, `version ${pkg.version} -> ${version}`]);

// android/app/build.gradle — versionName + versionCode(+1)
const gradlePath = join(root, "android", "app", "build.gradle");
if (existsSync(gradlePath)) {
  let gradle = readFileSync(gradlePath, "utf8");
  const codeMatch = gradle.match(/versionCode\s+(\d+)/);
  if (!codeMatch) {
    console.error("could not find versionCode in build.gradle");
    process.exit(1);
  }
  const nextCode = parseInt(codeMatch[1], 10) + 1;
  changes.push([gradlePath, () => {
    gradle = gradle
      .replace(/versionCode\s+\d+/, `versionCode ${nextCode}`)
      .replace(/versionName\s+"[^"]*"/, `versionName "${version}"`);
    writeFileSync(gradlePath, gradle);
  }, `versionName -> ${version}, versionCode ${codeMatch[1]} -> ${nextCode}`]);
}

// iOS build number (only if the project has been generated on the Mac)
const iosPbx = join(root, "ios", "App", "App.xcodeproj", "project.pbxproj");
if (existsSync(iosPbx)) {
  let pbx = readFileSync(iosPbx, "utf8");
  changes.push([iosPbx, () => {
    pbx = pbx
      .replace(/MARKETING_VERSION = [^;]+;/g, `MARKETING_VERSION = ${version};`)
      .replace(/CURRENT_PROJECT_VERSION = (\d+);/g,
        (_, n) => `CURRENT_PROJECT_VERSION = ${parseInt(n, 10) + 1};`);
    writeFileSync(iosPbx, pbx);
  }, `iOS MARKETING_VERSION -> ${version}, CURRENT_PROJECT_VERSION +1`]);
}

for (const [path, apply, desc] of changes) {
  console.log(`${dryRun ? "[dry-run] " : ""}${path.replace(root + "/", "")}: ${desc}`);
  if (!dryRun) apply();
}
console.log(dryRun ? "\nNo files written (--dry-run)." : "\nDone. Commit, then tag: git tag v" + version);
