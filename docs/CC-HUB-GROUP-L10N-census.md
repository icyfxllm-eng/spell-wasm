# CC-HUB-GROUP-L10N v1.1 §0 census — report only

Run 2026-09-28 against `main` at `e21ba2d4` (shipped build 254). Nothing changed.

**One HALT: Swahili.** Plus a diagnosis that differs from the file's, and a
conflict in the Appendix worth settling before any string is written.

---

## C1 — the headers are ALREADY keys. The bug is elsewhere.

`src/drawer.rs:53-57`:

```rust
const PLAY: [(modes::Group, &str); 3] = [
    (modes::Group::SpellIt,     "nav.spellIt"),
    (modes::Group::WordPuzzles, "nav.wordPuzzles"),
    (modes::Group::Meaning,     "nav.meaning"),
];
```

and `body()` renders `t(s.header_key)` — the same `t()`, on the same line of
code style, as the row names right beneath it. `group` is not a display
string either: it is `modes::Group`, an exhaustive Rust enum (`src/modes.rs`),
serialized as `spell_it` / `word_puzzles` / `meaning`.

So F1's stated hypothesis — "the `group` field holds a display string
instead of a key" — is not what happened. **The keys exist, the lookup is
already shared, and the call path is already single.**

**What actually happened is worse, and worth naming precisely: all fifteen
locales contain all three keys, and all fifteen hold the English string.**

| | `nav.spellIt` | `nav.wordPuzzles` | `nav.meaning` |
|---|---|---|---|
| en | Spell it | Word puzzles | Meaning |
| es | Spell it | Word puzzles | Meaning |
| ru | Spell it | Word puzzles | Meaning |
| …all 15 | Spell it | Word puzzles | Meaning |

The repo's existing parity gate (`scripts/i18n-check.mjs`, "884 keys parity
across 15 locales") checks that every key is **present** and non-empty. These
are. A key whose value is the English string is indistinguishable, to that
gate, from a translated one — which is exactly how three headers shipped in
English to fourteen languages without anything going red.

That matters for F3: a completeness gate as specified ("non-empty value for
each language") **would also have passed on this bug**. It needs to be a
sameness gate, not an emptiness gate — a non-English locale whose value
equals the English one is the failure being fixed. I will build it that way
unless you say otherwise.

## C2 — interface language. No HALT.

Row names resolve through `crate::i18n::t(&r.name_key)`, which reads the
interface locale (`spellgame.locale`), not the study language. Headers use
the identical call. I6 holds by construction today and will continue to.

## C3 — fifteen shipped uiLangs, and one has no Appendix row → **HALT**

From `src/i18n/locales/`, not hand-typed:

`ar de en es fil fr hi ja ko pl pt ru sw vi zh`

- **`sw` (Swahili) is shipped and has no row in the Appendix.** §2 says HALT
  on exactly this. Swahili is a live language with a 2,845-word bank and its
  own Azure voice; it is not a stub. I need four strings for it, or your
  instruction to leave sw in English for now.
- **`tl` in the Appendix vs `fil` shipped.** Same language, different code —
  and F2's own "Filipino goes to Paul" says the `tl` row is meant for it. I
  will map `tl` → `fil` unless you object; flagging rather than assuming
  because the codes differ.
- **`it` and `fa` are in the Appendix but not shipped.** Italian has no
  locale file; Persian has a word bank (`fa`, 8,340 words) but no UI locale.
  Both rows are simply unused. Not a HALT, and nothing to do.

## C4 — every other drawer string is already localized

The three Play headers are the **only** untranslated strings in the drawer:

| key | en | es | ru |
|---|---|---|---|
| `aria.yourWords` | Your words | Tus palabras | Твои слова |
| `settings.title` | Settings | Ajustes | Настройки |
| `credits.title` | Credits | Créditos | Благодарности |
| `top.signIn` | 👤 Sign in | 👤 Entrar | 👤 Войти |
| `acct.logout` | Log out | Cerrar sesión | Выйти |
| `top.misses` | Misses | Fallos | Ошибки |
| `top.myWords` | ＋ My words | ＋ Mis palabras | ＋ Мои слова |

Nothing to report under §7's "report, do not fix" — there is nothing else.

## C5 — nothing else reads `group`. No HALT.

`grep` for `.group` and `Group::` outside `modes.rs` and `drawer.rs` returns
nothing. It is never displayed, never persisted, never in a deep link, and
never an analytics label. Changing its representation breaks nothing —
though per C1 it is already an enum, so there is nothing to change.

## C6 — where The Climb's name appears, and a conflict

**The mode's internal id is `climb`, not "The Climb".** Nothing to preserve
there; F5's caveat does not bite.

Two *different* strings carry the name today, and they do not agree:

| Surface | Source | en | es | ru | zh | ar |
|---|---|---|---|---|---|---|
| drawer row | `top.theClimb` (registry nameKey) | 🏔 The Climb | **🏔 The Climb** | 🏔 Восхождение | **🏔 The Climb** | 🏔 التسلُّق |
| level selector + session pill | `level.climb` | Climb → | Ascenso → | Восхождение → | 攀登 → | التسلُّق ← |

So The Climb is **already localized in the session pill in every language**,
and the drawer row is the inconsistent one — translated in ru and ar, still
English in es and zh. F5 is half-done already, and not in the half the file
assumes.

**The Appendix conflicts with what ships.** Spanish: Appendix says *La
Escalada*, the pill says *Ascenso*. Arabic: Appendix *التسلّق* vs shipped
*التسلُّق* — the same word with different diacritics. Adopting the Appendix
verbatim would put two different Spanish words for The Climb one tap apart,
in the drawer row and the pill directly beneath it. That is the same class of
mistake D2 was signed to avoid.

**Recommendation:** reuse the shipped `level.climb` translations for the
drawer name (minus the trailing arrow), so the two surfaces agree, and send
*those* to the auditor rather than the Appendix drafts. es would read
*Ascenso*, ru *Восхождение*, zh *攀登*, ar *التسلُّق*. If you prefer the
Appendix wording, then `level.climb` has to change with it — say which and I
will make both match.

The name is also embedded inside nine other localized sentences
(`toast.newRecord`, `toast.loginToPost`, `fd.nameTitle`, `ghost.screen.none`,
`ghost.screen.start`, `tools.racing.desc`, `tools.racing.avail`,
`tools.shields.avail`, `acct.deleteSmall`, `widget.streak.desc`). All are in
the `en` table, so I8 is satisfiable — but they are prose, not names, and
re-wording them is not what F5 asks for. I will leave them.

**One real I8 hit:** `src/consts.rs:14` holds `("climb", "Climb \u{2192}")`.
That label is **dead** — `build_level_options` (`game.rs:848`) renders
`t("level.{v}")` and `offered_levels` destructures `(v, _)`, discarding it.
It is the only occurrence of the name outside the string table, and the fix
is to delete the unused column rather than translate it.

## D7 — Jr already has a pattern, and it is not the default

`settings.kid` = **"Spell Jr"** in all fifteen locales, and `ws.jr` = "Jr" in
all fifteen. Jr branding is treated exactly like SpellDoku: a name that does
not translate. So D7's default ("Escalada Jr") would break the established
pattern twice over — it translates the branding element and it uses the
Appendix's Spanish rather than the shipped one. Following the pattern C6
reveals, as D7 instructs, gives **"Ascenso Jr"**: localized noun, English
`Jr`. Reporting as required.

## I9 — already true, just unmarked

`sd.name` = "SpellDoku", `ws.name` = "Spell Search", `xw.name` = "Spell
Cross" are byte-identical in all fifteen locales today. F6 adds the
`do_not_translate` flag and the auditor-sheet note; the strings need no
change.

---

## What I need from you

1. **Swahili** (the HALT): four strings, or leave sw in English for now.
2. **The Climb's wording**: reuse the shipped `level.climb` translations so
   the row and the pill agree (my recommendation), or move both to the
   Appendix wording.
3. **`tl` → `fil`** — confirm, or tell me they are meant to be different.

Everything else in the file is buildable as written, with one change I would
make unasked and am flagging instead: F3's gate should fail a non-English
value that is *identical to English*, because the emptiness check it
specifies is the check that already passes on this bug.
