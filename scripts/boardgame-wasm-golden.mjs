#!/usr/bin/env node
// CC-BOARD-GAME A15 / I3 -- the engine's numbers on wasm32 equal the host's.
//
//   node scripts/boardgame-wasm-golden.mjs
//
// Builds tools/boardgame-golden (the real engine sources, included by path)
// for wasm32-unknown-unknown, loads it in node, and compares its three witnesses (the stream, the 84-tile Full game, the 42-tile Sprint game)
// with the constants pinned in src/boardgame/tests.rs. The wasm module has no
// imports, so no wasm-bindgen or browser is involved.
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const CRATE = join(ROOT, 'tools', 'boardgame-golden');

execFileSync('cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown', '--manifest-path', join(CRATE, 'Cargo.toml')], {
  stdio: 'inherit',
});
const wasm = readFileSync(join(CRATE, 'target', 'wasm32-unknown-unknown', 'release', 'boardgame_golden.wasm'));
const { instance } = await WebAssembly.instantiate(wasm, {});

const tests = readFileSync(join(ROOT, 'src', 'boardgame', 'tests.rs'), 'utf8');
const pinned = (name) => BigInt(tests.match(new RegExp(`${name}: u64 = (\\d+);`))[1]);

let bad = 0;
for (const [fn, konst] of [['golden_rng_1000', 'GOLDEN_RNG_1000'], ['golden_game', 'GOLDEN_GAME'], ['golden_game_sprint', 'GOLDEN_GAME_SPRINT']]) {
  const got = BigInt.asUintN(64, instance.exports[fn]());
  const want = pinned(konst);
  console.log(`${fn}: wasm32 ${got}  host ${want}  ${got === want ? 'OK' : 'MISMATCH'}`);
  if (got !== want) bad++;
}
if (bad) {
  console.error('A15 FAIL: the engine is not bit-identical on wasm32');
  process.exit(1);
}
console.log('A15 PASS: wasm32 matches the host');
