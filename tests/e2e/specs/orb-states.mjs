// orb-states.spec — CC-VISUAL-ECLIPSE Phase B F3 (AT-9, AT-10 in a running
// session, and the AT-8 instrument), behind visual.orb.
//
// The audio stub here is a real clip: a WAV generated below (silence, a tone,
// silence), so the orb's side read can decode it and the corona has a voice to
// follow. Off, nothing changes: no attribute, no second fetch of the clip.
import { openApp, typeOnKeyboard, assert } from '../harness.mjs';

// The flag, plus a silent browser voice that FINISHES. The harness's stub
// never fires its events, so a word whose server clip is held back (an
// audio-clarity FAIL routes it to the device voice) would speak forever.
const FLAG_ON = () => {
  localStorage.setItem('spell_flag_visual_orb', 'on');
  try {
    Object.defineProperty(window, 'speechSynthesis', {
      configurable: true,
      get: () => ({
        speak(u) { setTimeout(() => { u.onstart && u.onstart(); setTimeout(() => u.onend && u.onend(), 250); }, 10); }, // sleep-ok: a fake voice's own duration
        cancel() {}, pause() {}, resume() {},
        getVoices: () => [], speaking: false, pending: false, paused: false,
        addEventListener() {}, removeEventListener() {},
      }),
    });
  } catch (_) { /* the flag alone still drives the server-clip path */ }
};
const STATES = ['idle', 'fetching', 'speaking', 'awaiting', 'correct', 'wrong', 'milestone'];

/** 16-bit mono PCM WAV: lead silence, a 440 Hz tone, tail silence. */
function wav({ rate = 22050, lead = 0.12, tone = 0.45, tail = 0.12 } = {}) {
  const n = Math.round(rate * (lead + tone + tail));
  const buf = Buffer.alloc(44 + n * 2);
  buf.write('RIFF', 0); buf.writeUInt32LE(36 + n * 2, 4); buf.write('WAVE', 8);
  buf.write('fmt ', 12); buf.writeUInt32LE(16, 16); buf.writeUInt16LE(1, 20); buf.writeUInt16LE(1, 22);
  buf.writeUInt32LE(rate, 24); buf.writeUInt32LE(rate * 2, 28); buf.writeUInt16LE(2, 32); buf.writeUInt16LE(16, 34);
  buf.write('data', 36); buf.writeUInt32LE(n * 2, 40);
  for (let i = 0; i < n; i++) {
    const t = i / rate;
    const on = t >= lead && t < lead + tone;
    buf.writeInt16LE(on ? Math.round(Math.sin(2 * Math.PI * 440 * t) * 0.6 * 32767) : 0, 44 + i * 2);
  }
  return buf;
}
const CLIP = wav();

/** Serve the clip for every /api/speak, optionally late, and count requests. */
async function serveClip(ctx, { delayMs = 0 } = {}) {
  // `fetches` holds the page's own fetch() calls: the orb's side read, and the
  // existing warm-up of the NEXT word. The <audio> element's requests are type
  // `media` (it may make more than one). `sideReads(word)` keeps the ones for
  // the word being played, which only the orb asks for.
  const hits = { n: 0, fetches: [] };
  hits.sideReads = (word) => hits.fetches.filter((u) => new URL(u).searchParams.get('word') === word).length;
  await ctx.route('**/api/speak**', async (r) => {
    hits.n++;
    if (r.request().resourceType() === 'fetch') hits.fetches.push(r.request().url());
    if (delayMs) await new Promise((res) => setTimeout(res, delayMs)); // sleep-ok: the server's latency IS the condition under test
    await r.fulfill({ status: 200, contentType: 'audio/wav', body: CLIP, headers: { 'cache-control': 'public, max-age=31536000, immutable' } });
  });
  return hits;
}

/** Record every value #orbWrap[data-orb] takes, from now on. */
const watchStates = (page) => page.evaluate(() => {
  window.__orbSeen = [];
  const w = document.getElementById('orbWrap');
  const push = () => { const v = w.getAttribute('data-orb'); if (v && window.__orbSeen.at(-1) !== v) window.__orbSeen.push(v); };
  push();
  new MutationObserver(push).observe(w, { attributes: true, attributeFilter: ['data-orb'] });
  window.__orbEnv = [];
  const sample = () => { const v = parseFloat(w.style.getPropertyValue('--orb-env')); if (!Number.isNaN(v)) window.__orbEnv.push(v); requestAnimationFrame(sample); };
  requestAnimationFrame(sample);
});
const seen = (page) => page.evaluate(() => window.__orbSeen);
const seam = (page, f) => page.evaluate((f) => window.__spelltest[f](), f);

async function play(page) {
  await page.click('#orbWrap');
  await page.waitForFunction(() => !!window.__spelltest.currentWord(), null, { timeout: 5000 });
}

export async function run(browser, base, suite) {
  await suite.test('F3 off (default): no orb state, no listener, no side read of the clip', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en' });
    try {
      const hits = await serveClip(ctx);
      await play(page);
      await page.waitForTimeout(900); // sleep-ok: proving a NON-event (no second fetch, no attribute)
      const r = await page.evaluate(() => ({
        attr: document.getElementById('orbWrap').getAttribute('data-orb'),
        body: document.body.classList.contains('visual-orb'),
        env: document.getElementById('orbWrap').style.getPropertyValue('--orb-env'),
      }));
      assert(r.attr === null && !r.body && r.env === '', `the off path drew the orb: ${JSON.stringify(r)}`);
      assert(hits.n >= 1, 'the clip was never requested, so this proves nothing');
      const word = await page.evaluate(() => window.__spelltest.currentWord());
      assert(hits.sideReads(word) === 0, `the clip for "${word}" was side-read with the flag off: ${hits.fetches}`);
    } finally { await ctx.close(); }
  });

  await suite.test('F3 on: a slow clip goes fetching -> speaking -> awaiting, the corona follows the voice', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      const hits = await serveClip(ctx, { delayMs: 450 });
      assert((await page.getAttribute('#orbWrap', 'data-orb')) === 'idle', 'the orb does not start idle');
      // A word routed to the device voice has no clip to follow; move past it.
      for (let i = 0; i < 4; i++) {
        await watchStates(page);
        await play(page);
        await page.waitForFunction(() => window.__orbSeen.includes('awaiting'), null, { timeout: 8000 });
        if (/server clip/.test(await page.$eval('#audioSourceNote', (e) => e.textContent))) break;
        const w = await page.evaluate(() => window.__spelltest.currentWord());
        await typeOnKeyboard(page, w);
        await page.click('#checkBtn');
        await page.waitForFunction((w) => window.__spelltest.currentWord() !== w, w, { timeout: 8000 });
      }
      const s = await seen(page);
      const i = (st) => s.indexOf(st);
      assert(i('fetching') >= 0 && i('fetching') < i('speaking') && i('speaking') < i('awaiting'), `states ran ${s}`);
      const env = await page.evaluate(() => window.__orbEnv);
      assert(env.length > 5, `only ${env.length} corona frames`);
      assert(Math.max(...env) > 0.8 && Math.min(...env) < 0.1, `the corona did not follow the voice: min ${Math.min(...env)} max ${Math.max(...env)}`);
      const word = await page.evaluate(() => window.__spelltest.currentWord());
      assert(hits.sideReads(word) >= 1, `the envelope did not come from a side read of "${word}": ${hits.fetches}`);
      assert((await seam(page, 'orbRefused')) === 0, 'an undefined transition was refused');
    } finally { await ctx.close(); }
  });

  await suite.test('F3 on: a clip that answers inside 120 ms never shows fetching (no flicker)', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await serveClip(ctx);
      await play(page);
      await page.waitForFunction(() => window.__spelltest.orbState() === 'awaiting', null, { timeout: 8000 });
      // Replay: the clip is now in the HTTP cache and the element reused.
      await watchStates(page);
      await page.click('#orbWrap');
      await page.waitForFunction(() => window.__orbSeen.includes('speaking'), null, { timeout: 5000 });
      await page.waitForFunction(() => window.__orbSeen.includes('awaiting'), null, { timeout: 5000 });
      const s = await seen(page);
      assert(!s.includes('fetching'), `a cached replay flickered through fetching: ${s}`);
    } finally { await ctx.close(); }
  });

  await suite.test('F3 on: correct flares, wrong dims, and a whole session refuses no transition', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await serveClip(ctx);
      await play(page);
      const word = await page.evaluate(() => window.__spelltest.currentWord());
      await typeOnKeyboard(page, word);
      await page.click('#checkBtn');
      await page.waitForFunction(() => document.getElementById('orbWrap').getAttribute('data-orb') === 'correct', null, { timeout: 8000 });
      const flare = await page.$eval('#orbWrap .orb', (e) => getComputedStyle(e).animationName);
      assert(flare === 'orbFlare', `correct plays ${flare}`);
      // Next word, then a miss.
      await page.waitForFunction((w) => window.__spelltest.currentWord() && window.__spelltest.currentWord() !== w, word, { timeout: 8000 }).catch(() => {});
      if ((await page.evaluate(() => window.__spelltest.currentWord())) === word) await play(page);
      const next = await page.evaluate(() => window.__spelltest.currentWord());
      await typeOnKeyboard(page, next + 'q');
      await page.click('#checkBtn');
      await page.waitForFunction(() => document.getElementById('orbWrap').getAttribute('data-orb') === 'wrong', null, { timeout: 8000 });
      const dim = await page.$eval('#orbWrap .orb', (e) => getComputedStyle(e).animationName);
      assert(dim === 'orbDim', `wrong plays ${dim}, not the dim`);
      assert((await seam(page, 'orbRefused')) === 0, 'an undefined transition was refused');
    } finally { await ctx.close(); }
  });

  await suite.test('AT-9: with Reduce Motion, no transform animation runs in any of the 7 states', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      const moving = [];
      for (const st of STATES) {
        const r = await page.evaluate((st) => {
          const w = document.getElementById('orbWrap');
          w.setAttribute('data-orb', st);
          w.style.setProperty('--orb-env', '0.9');
          const els = [w, ...w.querySelectorAll('.orb, .ring, .orb-glyph')];
          const out = [];
          for (const a of document.getAnimations()) {
            if (!els.includes(a.effect?.target)) continue;
            const kf = a.effect.getKeyframes();
            if (kf.some((k) => k.transform && k.transform !== 'none')) out.push(`${a.animationName || 'anim'} on ${a.effect.target.className}`);
          }
          for (const e of els) {
            const t = getComputedStyle(e).transform;
            if (t && t !== 'none' && e.classList.contains('ring')) out.push(`static ${t} on ring`);
          }
          return out;
        }, st);
        if (r.length) moving.push(`${st}: ${r.join(', ')}`);
      }
      assert(!moving.length, `transform under Reduce Motion:\n  ${moving.join('\n  ')}`);
    } finally { await ctx.close(); }
  });

  await suite.test('V-D23: the idle breath rests while the page is hidden', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      const hidden = await page.evaluate(() => {
        Object.defineProperty(document, 'hidden', { configurable: true, get: () => true });
        document.dispatchEvent(new Event('visibilitychange'));
        const w = document.getElementById('orbWrap');
        return { rest: w.getAttribute('data-orb-rest'), play: getComputedStyle(w.querySelector('.orb')).animationPlayState };
      });
      assert(hidden.rest === 'hidden' && hidden.play === 'paused', `hidden: ${JSON.stringify(hidden)}`);
      const back = await page.evaluate(() => {
        Object.defineProperty(document, 'hidden', { configurable: true, get: () => false });
        document.dispatchEvent(new Event('visibilitychange'));
        return document.getElementById('orbWrap').getAttribute('data-orb-rest');
      });
      assert(back === null, `back on screen the orb still rests (${back})`);
    } finally { await ctx.close(); }
  });

  await suite.test('AT-8 instrument: the frame report carries the orb sync line', async () => {
    const { ctx, page } = await openApp(browser, base, { lang: 'en', init: FLAG_ON });
    try {
      await serveClip(ctx);
      const report = () => page.evaluate(() => {
        const logo = document.getElementById('brandMark');
        for (let i = 0; i < 5; i++) logo.click();
        document.getElementById('devFrameReport').click();
        const t = document.getElementById('devFrameReportText').textContent;
        document.getElementById('devMenuClose').click();
        return t;
      });
      // A word routed to the device voice has no envelope to time, so try a
      // few words until one plays a server clip.
      let r = '';
      for (let i = 0; i < 4; i++) {
        await play(page);
        await page.waitForFunction(() => window.__spelltest.orbState() === 'awaiting', null, { timeout: 8000 });
        r = await report();
        if (/orb sync \(AT-8\): \d+ words/.test(r)) break;
        const w = await page.evaluate(() => window.__spelltest.currentWord());
        await typeOnKeyboard(page, w);
        await page.click('#checkBtn');
        await page.waitForFunction((w) => window.__spelltest.currentWord() !== w, w, { timeout: 8000 });
      }
      assert(/orb sync \(AT-8\): \d+ words \| p95 [\d.]+ ms/.test(r), `no orb sync line:\n${r}`);
      assert(/refused moves 0/.test(r), `the report shows refused moves:\n${r}`);
    } finally { await ctx.close(); }
  });
}
