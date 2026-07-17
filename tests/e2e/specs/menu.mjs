// menu.spec — the language selector shows each language's endonym in its own
// script, and selecting a language switches the whole UI to match (menu
// integrity: selector is generated from the language registry, no drift).
import { openApp, assert } from '../harness.mjs';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

// This test needs BOTH an independent reference and the live registry, and the
// reason is worth spelling out, because each on its own is broken:
//
//   * Hardcoding every expected endonym is INDEPENDENT (it can catch a typo in
//     the registry) but DRIFTS. That is not hypothetical: Thai was cut from the
//     registry in 5fc69ff, this list kept demanding 'th: ไทย', and the suite sat
//     red asserting a language the product had deliberately removed.
//   * Reading the expectations from src/consts.rs fixes the drift and asserts
//     NOTHING about correctness — the menu is GENERATED from that same table, so
//     both sides move together. Verified: renaming English to "Anglais" in the
//     registry still passed. A tautology is worse than a stale test, because a
//     stale test at least tells you it is unhappy.
//
// So: the registry decides WHICH languages must appear (drift-proof), and the
// hardcoded table below decides HOW a language is spelled (independent). They
// are intersected — a cut language simply drops out of the check instead of
// breaking it, while a typo'd endonym is still caught.
const KNOWN_ENDONYMS = {
  en: 'English', es: 'Español', fr: 'Français', de: 'Deutsch', ko: '한국어',
  ja: '日本語', zh: '中文', vi: 'Tiếng Việt', fil: 'Filipino', pt: 'Português',
  pl: 'Polski', ru: 'Русский', ar: 'العربية', fa: 'فارسی', ur: 'اردو',
};

// src/consts.rs is the source of truth for the lineup (game::build_source_options
// iterates BUILTIN_LANGS to build the options). Endonyms are written as \u{XXXX}
// escapes there to keep the source ASCII, so decode those too.
function registryEndonyms() {
  const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
  const src = readFileSync(join(root, 'src', 'consts.rs'), 'utf8');
  const table = (src.match(/pub const BUILTIN_LANGS[^=]*=\s*\[([\s\S]*?)\n\];/) || [])[1];
  assert(table, 'could not find BUILTIN_LANGS in src/consts.rs — did the registry move?');
  const codes = Object.fromEntries(
    [...src.matchAll(/pub const ([A-Z]+): &str = "([a-z-]+)";/g)].map((m) => [m[1], m[2]])
  );
  const out = [];
  for (const m of table.matchAll(/\(\s*([A-Z]+)\s*,\s*"((?:[^"\\]|\\.)*)"\s*,/g)) {
    const code = codes[m[1]];
    assert(code, `BUILTIN_LANGS references ${m[1]} but no matching \`pub const ${m[1]}\``);
    out.push([code, m[2].replace(/\\u\{([0-9a-fA-F]+)\}/g, (_, h) => String.fromCodePoint(parseInt(h, 16)))]);
  }
  assert(out.length > 1, `parsed only ${out.length} languages from BUILTIN_LANGS — parser is broken`);
  return out;
}

export async function run(browser, base, suite) {
  await suite.test('menu: every language shows its own endonym', async () => {
    const { ctx, page } = await openApp(browser, base, {});
    try {
      const opts = await page.$$eval('#langSel option', (els) => els.map((e) => [e.value, e.textContent]));
      const expected = registryEndonyms();
      const map = Object.fromEntries(opts);

      // 1. Plumbing: every registry language reaches the menu, spelled as the
      //    registry spells it. Catches build_source_options dropping or
      //    mangling an entry. (Cannot catch a wrong endonym — see 3.)
      for (const [code, name] of expected) {
        assert(map[code] === name, `${code}: expected "${name}", got "${map[code]}"`);
      }

      // 2. The intersection must stay meaningful. Without this, renaming the
      //    language codes would empty the overlap and check 3 would vacuously
      //    pass while appearing to test something.
      const shared = expected.filter(([code]) => KNOWN_ENDONYMS[code]);
      assert(
        shared.length >= Math.min(10, expected.length),
        `only ${shared.length}/${expected.length} registry languages have a known endonym — ` +
        `the independent check has silently degraded; update KNOWN_ENDONYMS`
      );

      // 3. Correctness, independent of the registry: a language in the lineup is
      //    spelled the way its own speakers spell it. This is the only assertion
      //    here that survives someone editing consts.rs, because its reference
      //    does not come from consts.rs.
      for (const [code, name] of shared) {
        assert(
          name === KNOWN_ENDONYMS[code],
          `${code}: registry says "${name}", but the endonym is "${KNOWN_ENDONYMS[code]}". ` +
          `If the registry is right, correct KNOWN_ENDONYMS in this spec.`
        );
      }
      // Both directions: an option NOT in the registry is drift too — a language
      // cut from the registry must disappear from the menu, not linger.
      // ("My Words" appears only once custom words exist; this profile has none.)
      const extra = opts.map(([c]) => c).filter((c) => !expected.some(([e]) => e === c) && c !== 'mine');
      assert(extra.length === 0, `menu offers languages absent from the registry: ${extra.join(', ')}`);
      assert(opts.length >= expected.length, `menu shows ${opts.length} options, registry has ${expected.length}`);
    } finally { await ctx.close(); }
  });

  for (const [lang, tag] of [['ko', '철자'], ['ja', 'つづ'], ['zh', '拼'], ['th', 'สะกด'], ['fil', 'baybayin']]) {
    await suite.test(`menu: selecting ${lang} switches UI chrome`, async () => {
      const { ctx, page } = await openApp(browser, base, { lang });
      try {
        const brand = await page.$eval('.tag', (e) => e.textContent);
        assert(brand.includes(tag), `UI not switched to ${lang}: tag="${brand}"`);
        const htmlLang = await page.$eval('html', (e) => e.getAttribute('lang'));
        assert(htmlLang === lang, `<html lang> is "${htmlLang}", expected "${lang}"`);
      } finally { await ctx.close(); }
    });
  }

  // ---- CC-RTL F1: the study language reaches the play surface ----

  await suite.test('rtl F1: play surface carries the WORD\'s language + dir from the registry', async () => {
    // CC-RTL F1/D3. The play surfaces must carry the language of the word being
    // SPELLED, with `dir` read from the registry and nowhere else.
    //
    // NOTE, because it corrects an assumption worth writing down: in this app the
    // language picker ALSO switches the UI chrome, so <html lang> and the picker
    // agree and cannot be used to prove the plumbing. The real divergence is
    // `cur_lang` — Misses replays each word in ITS OWN language and Daily uses its
    // own locale, so the word on screen can be a different language from the
    // picker. That is why reflect_play_direction reads `cur_lang`, not `lang`.
    const { ctx, page } = await openApp(browser, base, { lang: 'es' });
    try {
      await page.click('#orbWrap').catch(() => {});
      await page.waitForTimeout(400);
      const got = await page.evaluate(() => {
        const q = (id) => {
          const e = document.getElementById(id);
          return { lang: e.getAttribute('lang'), dir: e.getAttribute('dir') };
        };
        return { picker: document.getElementById('langSel').value, letters: q('letters'), feedback: q('feedback'), meaning: q('meaning') };
      });
      for (const id of ['letters', 'feedback', 'meaning']) {
        assert(got[id].lang === got.picker, `#${id} lang=${got[id].lang}, expected the word's language (${got.picker})`);
        assert(got[id].dir === 'ltr', `#${id} dir=${got[id].dir}, expected ltr for Spanish`);
      }
    } finally { await ctx.close(); }
  });
}
