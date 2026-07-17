// P0.2 driver: opens the spike, harvests window.__spike, prints the evaluation.
import { chromium } from 'playwright';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
const HERE = dirname(fileURLToPath(import.meta.url));
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
await page.goto('file://' + join(HERE, 'index.html'));
await page.waitForFunction(() => window.__spike);
const { results } = await page.evaluate(() => window.__spike);
await page.screenshot({ path: join(HERE, 'spike.png'), fullPage: false });
await browser.close();

console.log('=== P0.2 · JOINING (baseline = today\'s per-letter spans)');
console.log('   If per-letter spans broke joining, isolated forms make the word WIDER.');
const overhangs = results.map(r => r.baselineOverhangPct);
const broke = results.filter(r => r.baselineOverhangPx > 0.5);
console.log(`   baseline wider than joined text in ${broke.length}/${results.length} cases`);
console.log(`   overhang: min ${Math.min(...overhangs)}%  median ${overhangs.slice().sort((a,b)=>a-b)[Math.floor(overhangs.length/2)]}%  max ${Math.max(...overhangs)}%`);
console.log('');
console.log('=== P0.2 · APPROACH A (text node + Range-measured markers)');
console.log(`   markers produced for every cluster: ${results.every(r => r.aMarkerCount === r.clusters)}`);
console.log(`   every marker has real width:        ${results.every(r => r.aMarkersWithinText)}`);
console.log('   joining: intact BY CONSTRUCTION (one text node — nothing is split)');
console.log('');
console.log('=== P0.2 · APPROACH B (canvas + prefix-measured clusters)');
console.log('   Per-cluster advance error vs A\'s REAL shaped geometry.');
console.log('   (sum(advances) === measureText(word) always — differences telescope — so');
console.log('    total width proves nothing. Per-cluster placement is the question.)');
const mx = results.map(r => r.bMaxClusterErrPx);
const mn = results.map(r => r.bMeanClusterErrPx);
console.log(`   max per-cluster error:  min ${Math.min(...mx).toFixed(2)}px  median ${mx.slice().sort((a,b)=>a-b)[Math.floor(mx.length/2)].toFixed(2)}px  max ${Math.max(...mx).toFixed(2)}px`);
console.log(`   mean per-cluster error: median ${mn.slice().sort((a,b)=>a-b)[Math.floor(mn.length/2)].toFixed(2)}px`);
console.log(`   cases where a marker misplaces by >2px: ${mx.filter(d => d > 2).length}/${results.length}`);
console.log('');
console.log('=== per-word detail (32px)');
console.log('word      clus  joined  baseline  overhang   B-maxErr');
for (const r of results.filter(r => r.size === 32)) {
  console.log(`${r.word.padEnd(9)} ${String(r.clusters).padStart(3)}  ${String(r.joinedWidth).padStart(6)}  ${String(r.baselineWidth).padStart(7)}  ${String(r.baselineOverhangPct).padStart(6)}%  ${String(r.bMaxClusterErrPx).padStart(6)}`);
}
