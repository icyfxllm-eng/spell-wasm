# Country → Language grant map — RATIONALE (review artifact for Eric)

This document explains **every grant** in `config/country-language-map.json` so
it can be red-penned. The map is the **single source of truth** for which
non-English languages are unlocked to **Full** for **free** in a given country
(the Rust core bundles it via `include_str!`; the Flask backend reads the *same*
file — never a second copy).

## Doctrine

- **Keys** = ISO 3166-1 alpha-2 country codes. **Values** = SpellGame language
  codes granted **Full** (free) in that country. Both the key list and each
  value array are **sorted**.
- **English is free everywhere anyway** (FREE_TIER ships English at Full for
  every user), so `en` is only listed where it is *co-official alongside another
  granted language* and we want that intent documented (CA, SG). Omitting `en`
  elsewhere changes nothing.
- Principle: each **non-English** language is granted in the country/countries
  where it is an **official language**. Multi-language countries grant **all**
  their shipped official languages.
- This is only about the **free regional grant**. The one-time **Complete**
  purchase raises *every* language to Full worldwide regardless of this map; the
  two unions together and neither subtracts.

## The 17 shipped languages

`en, es, fr, de, pt, it, nl, pl, sv, nb, tr, vi, ko, ja, fil, zh, th`
(the map must grant every **non-English** one in ≥1 country — a language with no
home country is a bug the CI check rejects).

## Per-language grants

- **Spanish (es)** — Spain plus Spanish-official Latin America & the Caribbean:
  `ES, MX, AR, CO, CL, PE, VE, EC, GT, CU, BO, DO, HN, PY, SV, NI, CR, PA, UY`
  and **GQ** (Equatorial Guinea, Spanish co-official). *Note:* country code `SV`
  is **El Salvador**; do not confuse it with language code `sv` (Swedish → `SE`).
  Territories such as Puerto Rico (`PR`) were left out of the first draft — flag
  if you want them in.
- **Portuguese (pt)** — `BR` and `PT`, per the decision. The broader lusophone
  set (`AO, MZ, CV, GW, ST, TL`) was intentionally **not** included in the draft;
  add if backend voice reach is confirmed there.
- **French (fr)** — `FR, MC` (Europe), plus the multilingual `BE, CA, CH, LU`,
  plus a first-draft **francophone-Africa** set: `SN, CI, ML, BF, NE, CM, CD, CG,
  GA, TG, BJ, MG`. This African list is the most likely place for red pen —
  trim/extend as you see fit.
- **German (de)** — `DE, AT, LI`, plus multilingual `CH, LU`.
- **Polish (pl)** — `PL`.
- **Russian (ru)** — `RU`, plus the post-Soviet states where Russian is an
  **official** language: `BY` (co-official), `KZ` and `KG` (official alongside the
  titular language). Tajikistan (`TJ`) recognizes Russian only as a language of
  interethnic communication, not an official one, so it is **left out** under this
  map's official-language rule — flag if you want it in. Ukraine (`UA`) is
  deliberately absent: Russian is not official there.
- **Arabic (ar)** — the **Arab League** members, where Arabic is official:
  `AE, BH, DJ, DZ, EG, IQ, JO, KM, KW, LB, LY, MA, MR, OM, PS, QA, SA, SD, SO,
  SY, TN, YE`. Per this map's multilingual rule, `DJ` (Djibouti) and `KM`
  (Comoros) also grant `fr`, which is co-official in both — the first time those
  two countries appear in the map at all. Several of these have no Apple
  storefront (see the storefront asymmetry note above).
- **Persian (fa)** — `IR` only. **Web-grant-only by construction** (no Apple
  storefront). Afghanistan (`AF`) is deliberately **excluded**: its Persian is
  Dari, and D3 chose Iranian Persian (`fa-IR`) over Dari. Tajik (`TJ`) is a
  different language in a different script.
- **Urdu (ur)** — `PK`. **India (`IN`) maps to nothing in this pass** (D6):
  Hindi is not in the lineup, and defaulting India to Urdu would be wrong for the
  overwhelming majority of its users.
- **Vietnamese (vi)** — `VN`.
- **Korean (ko)** — `KR`.
- **Japanese (ja)** — `JP`.
- **Filipino (fil)** — `PH`. (English is co-official there but is free anyway.)
- **Chinese (zh)** — `TW, HK, MO`, plus multilingual `SG`, plus **`CN`
  (see flag below)**.

## Multilingual countries (grant ALL shipped official languages)

| Country | Grants | Note |
|---|---|---|
| `CH` Switzerland | `de, fr, it` | (Romansh not shipped) |
| `BE` Belgium | `fr, nl` | (German-speaking community small; not granted) |
| `CA` Canada | `en, fr` | |
| `LU` Luxembourg | `de, fr` | (Luxembourgish not shipped) |
| `SG` Singapore | `en, zh` | (Malay/Tamil not shipped) |

## ⚠️ FLAG FOR ERIC — CN (China → zh)

`CN` is included in the draft as `["zh"]` **but is PENDING your decision on
backend reachability.** The Chinese TTS/voice path and `/api/*` endpoints must
be confirmed reachable from mainland China before this grant should ship. If the
backend is not reliably reachable inside the GFW, **remove the `CN` key** (Taiwan
`TW`, Hong Kong `HK`, Macau `MO`, and Singapore `SG` already cover Chinese
elsewhere, so `zh` keeps a home country even without `CN`).

## Open questions for red pen

1. Keep `CN`? (backend reachability — the one blocking flag above)
2. Extend Portuguese to the African lusophone countries?
3. Trim or extend the francophone-Africa French list?
4. Add Swedish to `FI`, Puerto Rico `PR` to Spanish, Cyprus to Turkish?
