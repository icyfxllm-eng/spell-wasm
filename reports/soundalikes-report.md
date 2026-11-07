# Sound-alike tables: shared census (generated)

CC-SENSE-CUE v1.1 F8. Regenerate with `python3 tools/soundalikes_report.py`.

| Lang | Status | Voice matches | Generator | Bank rows | true sets | accent sets | Confirmed | Loopback flags | Bonus-eligible words | Impossible removes | Proof test |
|---|---|---|---|---|---|---|---|---|---|---|---|
| en | verified | yes | build_collisions.key_en@1 | 3,157 | 252 | 27 | 19 | 36 | 277 | 306 | pass |
| es | verified | yes | build_collisions.key_es@1 | 6,264 | 9 | 0 | 0 | 1 | 13 | 13 | pass |
| fr | missing | yes | soundalike_keys.key_fr@1 | 6,271 | 1165 | 1 | 0 | 83 | 2499 | 2501 | not runnable |
| de | missing | yes | - | 6,258 | 22 | 0 | 0 | 0 | 22 | 22 | not runnable |
| pt | missing | yes | soundalike_keys.key_pt@1 | 6,258 | 25 | 1 | 0 | 2 | 16 | 17 | not runnable |
| pl | missing | yes | - | 6,210 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| vi | missing | yes | - | 1,186 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| ko | missing | yes | - | 3,163 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| ja | missing | yes | - | 4,745 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| fil | missing | yes | - | 4,011 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| zh | missing | yes | - | 5,808 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| ru | verified | yes | build_collisions.key_ru@1 | 6,808 | 8 | 4 | 1 | 5 | 16 | 24 | pass |
| ar | missing | yes | - | 6,279 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| sw | missing | yes | - | 2,826 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| hi | missing | yes | - | 2,088 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| fa | missing | yes | - | 8,340 | 0 | 0 | 0 | 0 | 0 | 0 | not runnable |
| en+retier | verified | yes | build_collisions.key_en@1+retier | 4,366 | 272 | 28 | 19 | 39 | 304 | 334 | pass |

Status stays `missing` for every language until the F5 voice loopback has run (Phase B). `none-by-construction` is granted only by the proof test with a spoken-key generator; no language has one yet beyond en, es and ru, whose banks do contain sound-alikes.
