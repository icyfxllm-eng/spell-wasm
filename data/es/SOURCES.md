# Sources — es

**STOP GATE:** verify each license's *current* text at ingest time (do not
trust this template). Record source, license, version/dump-date, URL, and any
required attribution. Present to Eric before ingesting.

| Source | License | URL | Version/date | Attribution |
|--------|---------|-----|--------------|-------------|
| kaikki.org (Wiktionary) — etymology for F5 word stories | **CC BY-SA 4.0** | https://kaikki.org/dictionary/rawdata.html | _TODO (dump date)_ | **REQUIRED — see below** |

### F5 Word Stories — CC BY-SA attribution (review gate, Decision D3)

Etymology text is derived from Wiktionary via kaikki.org and is licensed
**CC BY-SA 4.0**. It must not ship until Eric approves an attribution approach
and the `flags::word_stories()` flag is turned on. Exact notice text for both
options is in the PR summary; whichever is chosen goes here verbatim and into
`NOTICES.md`.

- [ ] Licenses verified
- [ ] Attribution strings captured (→ data/LICENSES.md, NOTICES.md)
- [ ] Presented to Eric
