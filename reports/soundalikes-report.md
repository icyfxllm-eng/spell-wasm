# Sound-alike tables: shared census (generated)

CC-SENSE-CUE v1.1 F8. Regenerate with `python3 tools/soundalikes_report.py`.

| Lang | Status | Voice matches | Generator | Bank rows | true sets | accent sets | Confirmed | Loopback flags | Bonus-eligible words | Impossible removes | Proof test |
|---|---|---|---|---|---|---|---|---|---|---|---|
| en | verified | yes | build_collisions.key_en@1 | 3,158 | 253 | 27 | 19 | 36 | 278 | 307 | pass |
| es | verified | yes | build_collisions.key_es@1 | 6,267 | 7 | 0 | 0 | 1 | 9 | 9 | pass |
| fr | missing | yes | - | 6,279 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| de | missing | yes | - | 6,268 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| pt | missing | yes | - | 6,272 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| pl | missing | yes | - | 6,215 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| vi | missing | yes | - | 1,189 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| ko | missing | yes | - | 3,163 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| ja | missing | yes | - | 4,979 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| fil | missing | yes | - | 4,047 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| zh | missing | yes | - | 5,810 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| ru | verified | yes | build_collisions.key_ru@1 | 6,832 | 29 | 4 | 2 | 4 | 58 | 66 | pass |
| ar | missing | yes | - | 6,290 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| sw | missing | yes | - | 2,818 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| hi | missing | yes | - | 2,095 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| en+retier | verified | yes | build_collisions.key_en@1+retier | 4,368 | 273 | 28 | 19 | 39 | 305 | 335 | pass |

Status stays `missing` for every language until the F5 voice loopback has run (Phase B). `none-by-construction` is granted only by the proof test with a spoken-key generator; no language has one yet beyond en, es and ru, whose banks do contain sound-alikes.
