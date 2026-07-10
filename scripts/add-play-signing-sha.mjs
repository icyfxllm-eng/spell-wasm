#!/usr/bin/env node
// Add the Google Play App Signing SHA-256 to .well-known/assetlinks.json so
// Android App Links (https://spellgame.net/app) open the store-installed app.
// Play re-signs your upload with its OWN key, so its SHA-256 must be listed here
// IN ADDITION to the upload-key + debug fingerprints already present.
//
// Get the SHA: Play Console → your app → Test and release → Setup → App signing
//   → "App signing key certificate" → SHA-256 fingerprint (copy it).
//
// Usage (accepts colons or not, any case):
//   node scripts/add-play-signing-sha.mjs AB:CD:...:EF
//   node scripts/add-play-signing-sha.mjs abcd...ef
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const raw = process.argv[2];
if (!raw) {
  console.error('usage: node scripts/add-play-signing-sha.mjs <SHA-256>');
  process.exit(1);
}

// Normalize to colon-separated uppercase hex pairs (assetlinks format).
const hex = raw.replace(/[^0-9a-fA-F]/g, '').toUpperCase();
if (hex.length !== 64) {
  console.error(`SHA-256 must be 32 bytes (64 hex chars); got ${hex.length} after stripping separators.`);
  process.exit(1);
}
const fingerprint = hex.match(/.{2}/g).join(':');

const file = join(dirname(fileURLToPath(import.meta.url)), '..', '.well-known', 'assetlinks.json');
const data = JSON.parse(readFileSync(file, 'utf8'));
const target = data[0]?.target;
if (!target || !Array.isArray(target.sha256_cert_fingerprints)) {
  console.error('assetlinks.json has an unexpected shape — inspect it manually.');
  process.exit(1);
}

const list = target.sha256_cert_fingerprints;
if (list.includes(fingerprint)) {
  console.log(`already present — no change:\n  ${fingerprint}`);
  process.exit(0);
}
list.push(fingerprint);
writeFileSync(file, JSON.stringify(data, null, 2) + '\n', 'utf8');
console.log(`added Play App Signing SHA-256 (${list.length} fingerprints total):\n  ${fingerprint}`);
console.log('\nNext: redeploy spellgame.net (git pull && docker compose up -d --build),');
console.log('then verify: adb shell pm get-app-links net.spellgame.app  (state should be "verified").');
