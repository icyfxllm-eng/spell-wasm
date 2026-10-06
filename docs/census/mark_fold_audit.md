# Mark-fold audit worksheet — Spell It letters-only verdict

Spell It grades what the player SAID. If a mark cannot be spoken as a letter, the
verdict has to ignore it — but ignoring a mark that distinguishes two real words
means accepting the wrong one. This worksheet is the evidence for deciding, per
language and per mark, which it is.

For each mark: how many words IN THE SHIPPING BANK stop being distinguishable if
that mark is folded away. A count of 0 is not proof the mark is decorative — it
means this bank contains no pair that collides, and the auditor still has to say
whether the mark is a LETTER (never fold) or an ACCENT (safe to fold).

`audited: false` in config/mark-fold-audit.json until a native speaker signs each
language. While false, nothing folds and Spell It stays accent-strict — which is
today's behaviour, where an accented word cannot be voice-spelled at all.

## Spanish (`es`) — 6097 words

| mark | collisions | examples | letter or accent? |
|---|---|---|---|
| `◌́` COMBINING ACUTE ACCENT | **78** | publicas/públicas; miercoles/miércoles; posicion/posición; acorde/acordé | _______ |
| `◌̃` COMBINING TILDE | **0** | — | _______ |
| `◌̈` COMBINING DIAERESIS | **0** | — | _______ |

> **Spanish needs a decision before anything else.** Folding `◌́` makes 78 bank words ambiguous, so a mark-blind verdict would accept a different real word as correct.

## French (`fr`) — 6109 words

| mark | collisions | examples | letter or accent? |
|---|---|---|---|
| `◌́` COMBINING ACUTE ACCENT | **111** | place/placé; situe/situé; trouve/trouvé; deja/déja | _______ |
| `◌̀` COMBINING GRAVE ACCENT | **4** | derriere/derrière; frere/frère; scene/scène; maniere/manière | _______ |
| `◌̂` COMBINING CIRCUMFLEX ACCENT | **9** | notre/nôtre; chateau/château; entraine/entraîne; hopital/hôpital | _______ |
| `◌̧` COMBINING CEDILLA | **2** | francais/français; francaise/française | _______ |
| `◌̈` COMBINING DIAERESIS | **0** | — | _______ |
| `œ` LATIN SMALL LIGATURE OE | **3** | soeur/sœur; soeurs/sœurs; oeuvre/œuvre | _______ |

> **French needs a decision before anything else.** Folding `◌́` makes 111 bank words ambiguous, so a mark-blind verdict would accept a different real word as correct.

## Portuguese (`pt`) — 6110 words

| mark | collisions | examples | letter or accent? |
|---|---|---|---|
| `◌́` COMBINING ACUTE ACCENT | **22** | tambem/também; familia/família; noticias/notícias; visita/visitá | _______ |
| `◌̃` COMBINING TILDE | **0** | — | _______ |
| `◌̧` COMBINING CEDILLA | **1** | forca/força | _______ |
| `◌̂` COMBINING CIRCUMFLEX ACCENT | **6** | camera/câmera; recebe/recebê; mes/mês; porque/porquê | _______ |

> **Portuguese needs a decision before anything else.** Folding `◌́` makes 22 bank words ambiguous, so a mark-blind verdict would accept a different real word as correct.

## German (`de`) — 6144 words

| mark | collisions | examples | letter or accent? |
|---|---|---|---|
| `◌̈` COMBINING DIAERESIS | **42** | alter/älter; zustande/zustände; farben/färben; kurzen/kürzen | _______ |
| `ß` LATIN SMALL LETTER SHARP S | **0** | — | _______ |

> **German needs a decision before anything else.** Folding `◌̈` makes 42 bank words ambiguous, so a mark-blind verdict would accept a different real word as correct.

## Polish (`pl`) — 6175 words

| mark | collisions | examples | letter or accent? |
|---|---|---|---|
| `◌́` COMBINING ACUTE ACCENT | **9** | robic/robić; znalezc/znaleźć; czesc/cześć; pomoc/pomóc | _______ |
| `ł` LATIN SMALL LETTER L WITH STROKE | **2** | stalo/stało; stale/stałe | _______ |
| `◌̨` COMBINING OGONEK | **65** | praca/pracą; sprawe/sprawę; osoba/osobą; muzyka/muzyką | _______ |
| `◌̇` COMBINING DOT ABOVE | **9** | mozna/można; kazda/każda; zadnych/żadnych; poniewaz/ponieważ | _______ |

> **Polish needs a decision before anything else.** Folding `◌̨` makes 65 bank words ambiguous, so a mark-blind verdict would accept a different real word as correct.

