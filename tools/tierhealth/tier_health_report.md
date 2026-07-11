## TL;DR — the bug does not reproduce on current `main`

This read-only diagnostic ran against exactly what `words::tier_for` ships
today. Every one of the order's three candidate causes is **absent** on
current data:

- **C1 tier overlap** — 0% adjacent-tier Jaccard and 0 easy∩expert for all
  17 languages (see per-language tables). Tiers are genuinely disjoint.
- **C2 English-shaped runtime formula** — N/A by construction: word data is
  pre-tiered into separate arrays; difficulty is *not* computed at selection
  time, so no formula runs per-language at runtime.
- **C3 pools too small** — smallest tier is 40 words (th/others), zh 150–200.
  The anti-repeat window (`pool/4≤50`) is fully covered; the 500-draw trace
  shows min-repeat-gap ≥ window and **0 fallback/relax events** everywhere.

### Before → After (the reported bug is the pre-fix state)

Eric's device report matches the state **before** two fixes already on `main`:

| | Mandarin (zh) per tier | anti-repeat window | felt result |
|--|--|--|--|
| **before** (pre `2dd1fed`) | **15 words** | `cap(15)=3` | recycles every ~3 draws → 'same words across difficulties' |
| **after** (current `main`) | **150–200 words** | `cap=37–50` | min repeat gap 39–51, 0 relax |

Fixes on `main`: `007e4a4` (Layer-1 re-tier of zh/ja/ko/th → disjoint tiers)
and `2dd1fed` (zh 60→700 HSK words). Current source tree = build 38; the App
Store submission remains build 24 (unchanged). **Most likely explanation:**
Eric tested a TestFlight build predating these commits — the same stale-build
cause as the Japanese-keyboard report. **Recommended action: ship a fresh
build and re-test th/zh before writing any selection fix.**

STOP-gate (D1): the report identifies the cause per language — it is *not* a
current-code defect. Per D6, no selection-logic change is warranted (English
traces identically clean; there is no shared-logic bug to fix). A cheap,
additive regression guard (D2 CI tier-health check, preserving I4) is proposed
as the safe next step but not yet applied — awaiting Eric's go.

---

## Summary — which of §1's causes applies, per language

| lang | worst adjacent overlap | smallest tier | verdict |
|--|--|--|--|
| en | 0% | 50 | OK |
| es | 0% | 50 | OK |
| fr | 0% | 40 | OK |
| de | 0% | 40 | OK |
| pt | 0% | 40 | OK |
| it | 0% | 40 | OK |
| nl | 0% | 40 | OK |
| pl | 0% | 40 | OK |
| sv | 0% | 40 | OK |
| nb | 0% | 40 | OK |
| tr | 0% | 40 | OK |
| vi | 0% | 46 | OK |
| ko | 0% | 44 | OK |
| ja | 0% | 41 | OK |
| fil | 0% | 46 | OK |
| zh | 0% | 150 | OK |
| th | 0% | 41 | OK |

Legend: **C1** = tier overlap · **C3** = pool too small · both can co-occur.
C2 (English-shaped runtime formula) is N/A: data is pre-tiered into separate
arrays, so difficulty is not computed at selection time — a collapse shows up
as C1 (bad tier assignment in the data) or C3 (thin pool), not C2.

---

# tier_health_report.md — EA difficulty-tier collapse diagnostic

Read-only. Source: `tools/tierhealth/tiers_dump.json` = exactly what
`words::tier_for(lang, tier)` ships (dumped by the `tier_dump::dump` test).

Flags: adjacent-tier Jaccard > 20% (I2), any tier < 12 words (anti-repeat).

---

## en

| tier | count | sample (first 10) |
|--|--|--|
| easy | 50 | bed, bird, book, cat, corn, cup, desk, dog, door, duck |
| medium | 50 | adventure, afternoon, balloon, bicycle, brother, building, business, calendar, children, chocolate |
| hard | 50 | accommodate, acquaintance, acquire, apparent, archipelago, arctic, awkward, believe, bureaucracy, catastrophe |
| expert | 50 | aberration, amanuensis, anachronism, apotheosis, bourgeoisie, brouhaha, cacophony, chiaroscuro, chrysanthemum, colonel |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (en): OK — tiers distinct and deep enough**

---

## es

| tier | count | sample (first 10) |
|--|--|--|
| easy | 50 | agua, ave, azul, boca, casa, cielo, dedo, dos, flor, fuego |
| medium | 50 | abeja, araña, ballena, bosque, botella, caballo, camino, camisa, castillo, cebolla |
| hard | 50 | aeropuerto, aguacate, alfombra, almuerzo, armario, ascensor, aventura, biblioteca, bicicleta, bolígrafo |
| expert | 50 | adyacente, ahínco, almohada, ambigüedad, arquitectónico, ataúd, bilingüe, cigüeña, circunferencia, desarrollar |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (es): OK — tiers distinct and deep enough**

---

## fr

| tier | count | sample (first 10) |
|--|--|--|
| easy | 40 | ami, arbre, blanc, bleu, café, chaise, chat, chaud, chien, ciel |
| medium | 40 | avion, banane, bateau, carotte, chapeau, chemise, cheval, château, cousin, couteau |
| hard | 40 | anniversaire, ascenseur, aventure, aéroport, bibliothèque, bicyclette, boulangerie, calendrier, casserole, ceinture |
| expert | 40 | accueil, aquarelle, chrysanthème, chuchotement, coquillage, cueillir, dictionnaire, dénouement, embarras, exhaustif |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (fr): OK — tiers distinct and deep enough**

---

## de

| tier | count | sample (first 10) |
|--|--|--|
| easy | 40 | Auge, Auto, Baum, Berg, Blume, Brot, Buch, Bär, Dorf, Ei |
| medium | 40 | Aufzug, Banane, Brille, Bruder, Elefant, Fahrrad, Familie, Fenster, Fisch, Fluss |
| hard | 40 | Abenteuer, Apotheke, Bibliothek, Bäckerei, Computer, Dinosaurier, Erdbeere, Erfahrung, Fahrkarte, Fernseher |
| expert | 42 | Aufmerksamkeit, Bewusstsein, Bürgersteig, Eichhörnchen, Entschuldigung, Enttäuschung, Freundschaft, Frühstück, Fußgängerzone, Gemütlichkeit |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (de): OK — tiers distinct and deep enough**

---

## pt

| tier | count | sample (first 10) |
|--|--|--|
| easy | 40 | azul, boca, bola, casa, cinto, cão, céu, dedo, dois, faca |
| medium | 40 | abacate, abelha, baleia, banana, caderno, caminho, camisa, castelo, cavalo, cebola |
| hard | 40 | abóbora, adjacente, almofada, armário, aventura, bicicleta, borboleta, brinquedo, cachecol, chaminé |
| expert | 40 | ambiguidade, arquitetônico, biblioteca, bilíngue, brócolis, calendário, circunferência, computador, desenvolver, desidratação |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (pt): OK — tiers distinct and deep enough**

---

## it

| tier | count | sample (first 10) |
|--|--|--|
| easy | 40 | acqua, ape, blu, bocca, cane, carta, casa, cielo, dito, due |
| medium | 40 | anatra, arancia, armadio, avocado, balena, banana, calzino, camicia, camino, carota |
| hard | 40 | adiacente, ascensore, avventura, bilingue, broccoli, cammino, cappello, castello, cavallo, cipolla |
| expert | 40 | ambiguità, architettonico, biblioteca, bicicletta, bottiglia, calendario, chilometro, chitarra, circonferenza, coccodrillo |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (it): OK — tiers distinct and deep enough**

---

## nl

| tier | count | sample (first 10) |
|--|--|--|
| easy | 40 | auto, beer, berg, boek, boom, bril, dag, deur, dorp, eend |
| medium | 43 | aardbei, appel, banaan, blauw, bloem, brief, broer, brood, familie, fiets |
| hard | 40 | aardappel, ademhaling, apotheek, avontuur, bakkerij, chocolade, computer, dinosaurus, eekhoorn, ervaring |
| expert | 41 | aanwezigheid, belangrijk, bibliotheek, buitengewoon, gebeurtenis, gebruikelijk, gemakkelijk, gemeenschap, gezelligheid, handschoen |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (nl): OK — tiers distinct and deep enough**

---

## pl

| tier | count | sample (first 10) |
|--|--|--|
| easy | 40 | banan, brat, dom, dąb, jajko, kot, koń, krowa, kwiat, las |
| medium | 43 | apteka, biały, cebula, chleb, chmura, ciasto, ciekawy, ciotka, drzewo, drzwi |
| hard | 40 | czekolada, czerwony, dinozaur, dziecko, hipopotam, kalendarz, kilometr, komputer, konieczny, koszula |
| expert | 43 | biblioteka, chłopiec, doświadczenie, dziewczyna, gospodarka, jednocześnie, możliwość, nadzwyczajny, natychmiast, niespodzianka |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (pl): OK — tiers distinct and deep enough**

---

## sv

| tier | count | sample (first 10) |
|--|--|--|
| easy | 40 | and, barn, berg, bil, blå, bok, bord, bror, by, dag |
| medium | 40 | ansvar, apotek, bageri, banan, bröd, cykel, dator, dörr, ekorre, familj |
| hard | 40 | andning, ansvarig, apelsin, björn, blomma, choklad, elefant, farbror, fjäril, flicka |
| expert | 40 | bibliotek, dinosaurie, erfarenhet, flodhäst, flygplats, födelsedag, förväntan, gemenskap, glasögon, hastighet |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (sv): OK — tiers distinct and deep enough**

---

## nb

| tier | count | sample (first 10) |
|--|--|--|
| easy | 40 | and, barn, bil, blå, bok, bord, bror, dag, dør, egg |
| medium | 40 | ansvar, apotek, avtale, bakeri, banan, blomst, brød, ekorn, fjell, frosk |
| hard | 40 | appelsin, bjørn, briller, bursdag, dinosaur, elefant, erfaring, eventyr, familie, fjernsyn |
| expert | 40 | ansvarlig, bibliotek, bringebær, datamaskin, enestående, fellesskap, forskjellige, forventning, hastighet, ingrediens |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (nb): OK — tiers distinct and deep enough**

---

## tr

| tier | count | sample (first 10) |
|--|--|--|
| easy | 40 | aile, amca, araba, armut, at, ay, ayak, ayna, ayı, beyaz |
| medium | 40 | ateş, bahçe, balık, dikkat, doğal, eczane, gitar, havuç, hemen, kabuk |
| hard | 40 | ahududu, ağaç, ağız, beklenti, bisiklet, bıçak, deneyim, dinozor, domates, eldiven |
| expert | 40 | anlaşma, arkadaşlık, asansör, ayakkabı, ayçiçeği, bağımsızlık, bilgisayar, buzdolabı, doğumgünü, genellikle |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (tr): OK — tiers distinct and deep enough**

---

## vi

| tier | count | sample (first 10) |
|--|--|--|
| easy | 47 | bay, bàn, bát, bò, búa, bút, bơi, cam, cao, cau |
| medium | 46 | chim, chó, cầu, cửa, dừa, ghế, gió, gấu, khoai, khô |
| hard | 46 | biết, biển, buồn, bánh, bảng, chân, chơi, chảo, chậm, giàu |
| expert | 46 | bướm, chuyện, chuối, chuột, cười, giường, hưởng, hồng, khoảng, khuyên |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (vi): OK — tiers distinct and deep enough**

---

## ko

| tier | count | sample (first 10) |
|--|--|--|
| easy | 45 | 감, 강, 거리, 국, 귀, 꿈, 나라, 나무, 낮, 눈 |
| medium | 44 | 가방, 가슴, 가을, 가족, 개, 거실, 겨울, 계단, 과일, 교실 |
| hard | 44 | 건물, 구급차, 국자, 기차역, 도서관, 도시락, 동물, 무지개, 물고기, 사진기 |
| expert | 44 | 경찰서, 공원, 극장, 냄비, 냉장고, 놀이터, 뒤집개, 맷돌, 미술관, 박물관 |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (ko): OK — tiers distinct and deep enough**

---

## ja

| tier | count | sample (first 10) |
|--|--|--|
| easy | 42 | あお, あか, あき, あさ, あし, あめ, いえ, いけ, いし, いす |
| medium | 42 | あした, あたま, おふろ, かえる, かぜ, かべ, きいろ, きつね, くるま, ちず |
| hard | 42 | いちご, うさぎ, えんぴつ, かいだん, かぞく, かばん, くすりや, くだもの, けしごむ, げんかん |
| expert | 41 | えいがかん, おうだんほどう, かんらんしゃ, がっこう, きって, きのう, きゅうきゅうしゃ, きょう, ぎんこう, くうこう |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (ja): OK — tiers distinct and deep enough**

---

## fil

| tier | count | sample (first 10) |
|--|--|--|
| easy | 46 | ahas, apoy, araw, asin, aso, asul, babae, baboy, bag, bago |
| medium | 46 | alaala, asukal, bawang, baño, bibig, bituin, biyaya, bubong, buhok, bundok |
| hard | 46 | aklatan, bintana, drayber, hardin, himala, kabayo, kalabaw, kamatis, kambing, kamote |
| expert | 46 | araw-araw, bahaghari, bahay-bahayan, bisikleta, bulaklak, dalangin, damdamin, damdaming, dingding, iskedyul |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (fil): OK — tiers distinct and deep enough**

---

## zh

| tier | count | sample (first 10) |
|--|--|--|
| easy | 150 | wo3men5, mei2you3, ta1men5, shen2me5, xian4zai4, zhi1dao4, shi2hou5, gong1zuo4, ni3men5, shi2jian1 |
| medium | 150 | zi4ji3, ke3yi3, yi3jing1, zhe4yang4, ru2guo3, yin1wei4, wen4ti2, dan4shi4, ke3neng2, suo3yi3 |
| hard | 200 | jiu4shi4, xu1yao4, kai1shi3, xi1wang4, fa1sheng1, wei4le5, zhi3shi4, shi4jie4, qing2kuang4, bi3sai4 |
| expert | 200 | wei2yi1, ru2ci3, ken3ding4, you3guan1, kong4zhi4, zu3zhi1, yong1you3, zui4zhong1, bu4zai4, si3wang2 |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (zh): OK — tiers distinct and deep enough**

---

## th

| tier | count | sample (first 10) |
|--|--|--|
| easy | 41 | กบ, กาว, งา, งู, ช้า, ดาว, ดิน, ดี, ตา, ทะเล |
| medium | 41 | กางเกง, กุ้ง, ขนม, ข้าว, ช้าง, ดินสอ, ถนน, ถั่ว, บ้าน, ปากกา |
| hard | 41 | ข้าวโพด, ครู, ควาย, จิ้งจก, ดอกไม้, ตลาด, ตุ๊กแก, ต้นไม้, นักเรียน, น้อง |
| expert | 41 | กรรไกร, กระดาษ, กระต่าย, กระรอก, กระเทียม, กระเป๋า, กล้วย, การศึกษา, คณิตศาสตร์, ความรัก |

Overlap (Jaccard / shared count) — adjacent pairs are the ones that matter:

| pair | Jaccard | shared | flag |
|--|--|--|--|
| easy∩medium | 0% | 0 |  |
| medium∩hard | 0% | 0 |  |
| hard∩expert | 0% | 0 |  |
| easy∩expert | 0% | 0 | ok |

**Verdict (th): OK — tiers distinct and deep enough**

---

## Selection-path trace (500 draws / lang×tier, current data)

Replicates `selection.rs` pure math (exclusion window = pool/4≤50, 3 sub-bands,
first-3 mid-band). `min gap` = closest two serves of one word; `relax` = times
`available()` fell back to the full band (the in-selection fallback). A min-gap
below the window would mean a visible near-term repeat.

| lang | tier | pool | window | min repeat gap | relax events |
|--|--|--|--|--|--|
| en | easy | 50 | 12 | 15 | 0 |
| en | medium | 50 | 12 | 15 | 0 |
| en | hard | 50 | 12 | 15 | 0 |
| en | expert | 50 | 12 | 15 | 0 |
| es | easy | 50 | 12 | 15 | 0 |
| es | medium | 50 | 12 | 15 | 0 |
| es | hard | 50 | 12 | 15 | 0 |
| es | expert | 50 | 12 | 15 | 0 |
| fr | easy | 40 | 10 | 12 | 0 |
| fr | medium | 40 | 10 | 12 | 0 |
| fr | hard | 40 | 10 | 12 | 0 |
| fr | expert | 40 | 10 | 12 | 0 |
| de | easy | 40 | 10 | 12 | 0 |
| de | medium | 40 | 10 | 12 | 0 |
| de | hard | 40 | 10 | 12 | 0 |
| de | expert | 42 | 10 | 12 | 0 |
| pt | easy | 40 | 10 | 12 | 0 |
| pt | medium | 40 | 10 | 12 | 0 |
| pt | hard | 40 | 10 | 12 | 0 |
| pt | expert | 40 | 10 | 12 | 0 |
| it | easy | 40 | 10 | 12 | 0 |
| it | medium | 40 | 10 | 12 | 0 |
| it | hard | 40 | 10 | 12 | 0 |
| it | expert | 40 | 10 | 12 | 0 |
| nl | easy | 40 | 10 | 12 | 0 |
| nl | medium | 43 | 10 | 11 | 0 |
| nl | hard | 40 | 10 | 12 | 0 |
| nl | expert | 41 | 10 | 12 | 0 |
| pl | easy | 40 | 10 | 12 | 0 |
| pl | medium | 43 | 10 | 11 | 0 |
| pl | hard | 40 | 10 | 12 | 0 |
| pl | expert | 43 | 10 | 11 | 0 |
| sv | easy | 40 | 10 | 12 | 0 |
| sv | medium | 40 | 10 | 12 | 0 |
| sv | hard | 40 | 10 | 12 | 0 |
| sv | expert | 40 | 10 | 12 | 0 |
| nb | easy | 40 | 10 | 12 | 0 |
| nb | medium | 40 | 10 | 12 | 0 |
| nb | hard | 40 | 10 | 12 | 0 |
| nb | expert | 40 | 10 | 12 | 0 |
| tr | easy | 40 | 10 | 12 | 0 |
| tr | medium | 40 | 10 | 12 | 0 |
| tr | hard | 40 | 10 | 12 | 0 |
| tr | expert | 40 | 10 | 12 | 0 |
| vi | easy | 47 | 11 | 12 | 0 |
| vi | medium | 46 | 11 | 12 | 0 |
| vi | hard | 46 | 11 | 12 | 0 |
| vi | expert | 46 | 11 | 12 | 0 |
| ko | easy | 45 | 11 | 12 | 0 |
| ko | medium | 44 | 11 | 12 | 0 |
| ko | hard | 44 | 11 | 12 | 0 |
| ko | expert | 44 | 11 | 12 | 0 |
| ja | easy | 42 | 10 | 12 | 0 |
| ja | medium | 42 | 10 | 12 | 0 |
| ja | hard | 42 | 10 | 12 | 0 |
| ja | expert | 41 | 10 | 12 | 0 |
| fil | easy | 46 | 11 | 12 | 0 |
| fil | medium | 46 | 11 | 12 | 0 |
| fil | hard | 46 | 11 | 12 | 0 |
| fil | expert | 46 | 11 | 12 | 0 |
| zh | easy | 150 | 37 | 39 | 0 |
| zh | medium | 150 | 37 | 39 | 0 |
| zh | hard | 200 | 50 | 51 | 0 |
| zh | expert | 200 | 50 | 51 | 0 |
| th | easy | 41 | 10 | 12 | 0 |
| th | medium | 41 | 10 | 12 | 0 |
| th | hard | 41 | 10 | 12 | 0 |
| th | expert | 41 | 10 | 12 | 0 |

