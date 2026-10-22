# Outside-twin sweep (Eric, 2026-10-22)

Question: how many Impossible pool words have a rival spelling in a FULL dictionary that is not in our bank?
Method: unmunched Hunspell dictionaries (wooorm/dictionaries; es ru fr de pt pl; ko stems), coarse sound key per language
(rules_nets.py, plus coarse es/ru keys in `tools/impossible/outside_twins.py`), proper nouns dropped (not de), pure
inflections (one word is the other with a changed ending) split out. fil and sw have no open dictionary: not swept.

| lang | pool | any outside same-key word | not a mere inflection |
|---|---|---|---|
| es | 2995 | 289 (9.6%) | 131 (4.4%) |
| ru | 2997 | 183 (6.1%) | 97 (3.2%) |
| fr | 1827 | 1198 (65.6%) | 663 (36.3%) |
| de | 2963 | 150 (5.1%) | 138 (4.7%) |
| pt | 2619 | 1242 (47.4%) | 768 (29.3%) |
| pl | 2930 | 668 (22.8%) | 338 (11.5%) |
| ko | 1399 | 157 (11.2%) | 157 (11.2%) |

These are CANDIDATES, an upper bound. The keys are deliberately coarse, so most hits are different words that sound
close (ko merges plain/tense/aspirated; es accent pairs such as ame/amé are different words with different stress).
Sampled precision is low: a minority are real rival spellings. Fixed from this sweep: Russian ё/е (both accepted).
Open: German reform pairs (Xylophon/Xylofon, Nußknacker/Nussknacker), and whether to run the TTS+whisper loopback on the
candidate pairs to separate real twins from near-misses.
