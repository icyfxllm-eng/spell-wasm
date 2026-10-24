// hotfix-diag.spec — CC-ECLIPSE-HOTFIX F4 / I8: the mic diag readout.
//
// This runs against the TEST build, which keeps the readout behind its
// dev-menu switch; every release build carries none of it (I7: scripts/
// cut-diag.py, gated by diag-dev-door-check and seam-absence-check). The
// native plugin is faked so capture events can be fired: the dev door is ON
// in every case here, because that is how Eric's phone had it in 298.
import { readFileSync } from 'node:fs';
import { domSettled, assert } from '../harness.mjs';

const EN = JSON.parse(readFileSync(new URL('../../../src/i18n/locales/en.json', import.meta.url), 'utf8'));
import { openThemed, drawerRows, openRow } from '../hotfix-lib.mjs';

/** A NativeLanguageKit that records listeners and lets the test fire events. */
const FAKE_PLUGIN = `(() => {
  const subs = {};
  const plugin = new Proxy({
    addListener(evt, fn) { (subs[evt] ||= []).push(fn); return { remove() { subs[evt] = (subs[evt] || []).filter((f) => f !== fn); } }; },
    // An installed on-device recognizer, so the voice mic shows.
    speechCapabilities() { return Promise.resolve({ available: true, supportsOnDevice: true, locale: 'en-US', state: 'installed', engine: 'sf' }); },
    // Like NativeLanguageKitPlugin: a capture that hears no speech ends itself
    // after noSpeechTimeout (6 s, S4: unchanged) through the normal stop path,
    // a final with no token. Any 1110 inside it was swallowed natively.
    startLetterCapture() {
      window.__captureAt = performance.now();
      if (window.__silentEnd) clearTimeout(window.__silentEnd);
      window.__silentEnd = setTimeout(() => { window.__endedAt = performance.now(); (subs.letterFinal || []).slice().forEach((f) => f({ token: '', end: true })); }, 6000);
      return Promise.resolve();
    },
    stopLetterCapture() { (subs.letterFinal || []).slice().forEach((f) => f({ token: '', end: true })); return Promise.resolve(); },
  }, { get: (t, k) => (k in t ? t[k] : () => Promise.resolve({})) });
  window.__fireNative = (evt, data) => (subs[evt] || []).slice().forEach((f) => f(data));
  window.Capacitor = { isNativePlatform: () => true, getPlatform: () => 'ios', Plugins: { NativeLanguageKit: plugin } };
  localStorage.setItem('spell_dev_entitlements', 'on');
})();`;

const shown = (page) => page.evaluate(() => {
  const el = document.getElementById('voiceSpellDiag');
  return !!el && !el.hidden && el.getClientRects().length > 0 && /\S/.test(el.textContent);
});

async function capture(page) {
  await page.evaluate(() => {
    window.SpellNativeLang.startLetterCapture({ lang: 'en' }, () => {}, () => {}, () => {}, () => {});
  });
  await page.evaluate(() => {
    window.__fireNative('letterDiag', { info: 'rms=0.031 thr=0.02 bufs=12' });
    window.__fireNative('letterToken', { token: 'a' });
  });
  await domSettled(page);
}

async function openWith(browser, base, extra) {
  const { ctx, page } = await openThemed(browser, base, 'eclipse', { extra });
  await ctx.addInitScript(FAKE_PLUGIN);
  await page.reload({ waitUntil: 'load' });
  await page.waitForFunction(() => window.__spelltest && window.SpellNativeLang && window.__fireNative, null, { timeout: 30000 });
  await domSettled(page);
  return { ctx, page };
}

export async function run(browser, base, suite) {
  await suite.test('I8: with the switch off (default), a live capture paints no diag, on any route', async () => {
    const { ctx, page } = await openWith(browser, base, {});
    const bad = [];
    try {
      await capture(page);
      if (await shown(page)) bad.push('the readout painted on the play screen with the switch off');
      const rows = await drawerRows(page);
      for (const row of rows) {
        await page.goto(page.url(), { waitUntil: 'load' });
        await page.waitForFunction(() => window.__spelltest && window.__fireNative, null, { timeout: 30000 });
        await capture(page);
        await openRow(page, row);
        if (await shown(page)) bad.push(`${row}: a diag node is visible`);
      }
    } finally { await ctx.close(); }
    assert(!bad.length, bad.join('; '));
  });

  await suite.test('F4: with the switch on, the readout shows during a capture and goes when the mic stops or the route changes', async () => {
    const { ctx, page } = await openWith(browser, base, { spell_diag_bar: 'on' });
    try {
      await capture(page);
      assert(await shown(page), 'the readout did not show with the switch on during a capture');
      assert(/diag: /.test(await page.textContent('#voiceSpellDiag')), 'the readout lost its diag line');
      await page.evaluate(() => window.__fireNative('letterFinal', { token: '', end: true }));
      await domSettled(page);
      assert(!(await shown(page)), 'the readout stayed up after the mic stopped');
      // A route change mid-capture: the new screen stops the mic (on_new_word /
      // close), the plugin ends the session, and the readout goes with it.
      await capture(page);
      assert(await shown(page), 'the readout did not come back for a second capture');
      await openRow(page, 'boardgame');
      await page.evaluate(() => window.SpellNativeLang.stopLetterCapture());
      await domSettled(page);
      assert(!(await shown(page)), 'the readout followed the player into Board Game');
    } finally { await ctx.close(); }
  });

  await suite.test('I11 (S4): 6 s after a tap with no speech, the mic is enabled and not listening, and the retry hint shows', async () => {
    const { ctx, page } = await openWith(browser, base, {});
    try {
      await page.click('#orbWrap');
      await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
      await page.waitForSelector('#voiceSpellMic:not(.btn-hide)', { timeout: 5000 });
      await page.click('#voiceSpellMic');
      await page.waitForFunction(() => document.getElementById('voiceSpellMic').classList.contains('listening'), null, { timeout: 2000 });
      const hint = EN['voiceSpell.didntCatch'];
      await page.waitForFunction((hint) => {
        const m = document.getElementById('voiceSpellMic');
        return window.__endedAt && !m.classList.contains('listening') && !m.disabled
          && document.getElementById('voiceSpellStatus').textContent === hint;
      }, hint, { timeout: 8000, polling: 20 });
      const t = await page.evaluate(() => ({ since: performance.now() - window.__captureAt, after: performance.now() - window.__endedAt }));
      assert(t.since >= 6000, `reset before the 6 s self-end (${t.since.toFixed(0)} ms)`);
      assert(t.after <= 300, `the mic took ${t.after.toFixed(0)} ms after the self-end to reset (I11: within 300 ms)`);
      assert(await page.$eval('#voiceSpellMic', (m) => getComputedStyle(m).opacity === '1'), 'the mic is dimmed after the reset');
    } finally { await ctx.close(); }
  });

  await suite.test('F4: the switch lives in the dev menu and defaults off', async () => {
    const { ctx, page } = await openWith(browser, base, {});
    try {
      const sw = await page.$('#devMenu #devDiagBar');
      assert(sw, 'no Mic diag bar switch in the dev menu');
      assert(!(await sw.isChecked()), 'the switch defaults on');
    } finally { await ctx.close(); }
  });
}
