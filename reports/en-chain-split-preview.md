# English sound-alike sets: what un-chaining would split (preview, nothing changed)

Written 2026-10-17. Grading is unchanged until you approve.

## The cause

`tools/build_collisions.py` groups words that share one CMUdict pronunciation, then **merges any two groups that share a member, over and over**. A word with two pronunciations links two groups, so unrelated words end up in one set. Example: *err* is said EH R (like *air*) and ER (like *or*), so *air* and *or* become "the same sound". The worst case is one 15-word set, and grading accepts any member for any other.

## The fix

Keep each shared-pronunciation group as its own set, with no merging. A word with two pronunciations sits in two sets and is accepted with the members of either, but two words that share no pronunciation are no longer linked.

## Effect on today's English table

- **Sets:** 263 merged sets become 280 unmerged ones. 11 sets split.
- **Pairs that stop being accepted:** 90. Pairs still accepted: 454.
- **Words that will sit in more than one set:** 19. The app code has to support that (today a word can be in only one set), so the change also touches `src/homophones.rs`.
- **Re-tier table:** 12 sets split, 91 pairs lost.

Spelluzzle and Spell Cross read these sets too, so their pinned boards will need re-pinning when the change ships.

## Every set that splits

| Today's set | Becomes | Pairs no longer accepted |
|---|---|---|
| air / ar / are / er / ere / err / eyre / heir / hour / oar / or / ore / our / r / ur | air / ere / err / eyre / heir · ar / are / our / r · are / er / err / or / ur · hour / our · oar / or / ore | 75: air–ar, air–are, air–er, air–hour, air–oar, air–or, air–ore, air–our, air–r, air–ur, ar–er, ar–ere … |
| fir / for / fore / four / fur | fir / for / fur · for / fore / four | 4: fir–fore, fir–four, fore–fur, four–fur |
| beau / bo / bough / bow | beau / bo / bow · bough / bow | 2: beau–bough, bo–bough |
| wen / when / win / wynn | wen / when · when / win / wynn | 2: wen–win, wen–wynn |
| an / ann / un | an / ann · an / un | 1: ann–un |
| been / bin / bun | been / bin · been / bun | 1: bin–bun |
| caller / color / colour | caller / color · color / colour | 1: caller–colour |
| one / wan / won | one / won · wan / won | 1: one–wan |
| read / red / reed | read / red · read / reed | 1: red–reed |
| root / rout / route | root / route · rout / route | 1: root–rout |
| saint / st / street | saint / st · st / street | 1: saint–street |

Only in the re-tier table:

- cache / cachet / cash → cache / cachet · cache / cash (1 pairs: cachet–cash)

## Something the fix does not touch

Some unmerged groups still come from CMUdict's casual or weak forms: *been/bun* (B AH N), *an/un*, *or/err/ur* (ER), *when/win*. Those are real in fast speech but doubtful for a single clearly spoken word. The loopback flagged several of them for the auditor; the same-sound sheet is the place to settle them.
