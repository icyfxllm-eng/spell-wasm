# Word-list provenance pipeline (CC-WORDLIST-SOURCES).
#
#   make wordlist LANG=es       # fetch -> verify -> unmunch -> filter -> emit (+ surface index)
#   make surface-index LANG=es  # fetch -> verify -> unmunch -> raw provenance surface index
#   make provenance LANG=es     # validate shipped curated list against the raw source index
#   make credits                # regenerate credits.json from sources/registry.json
#   make license-gate           # run the CI license gate locally (registry + provenance)
#
# REVIEW-GATED: these targets produce wordlists/<lang>.txt, a raw provenance index
# (sources/<lang>/surface-index.txt) and a provenance report for Eric's review.
# They do NOT touch assets/words/ or src/ (the shipped curated list is untouched).

LANG ?= es

.PHONY: wordlist surface-index provenance credits license-gate

wordlist:
	./scripts/wordlist.sh $(LANG)

surface-index:
	./scripts/surface-index.sh $(LANG)

provenance:
	node scripts/provenance-validate.mjs --lang $(LANG)

credits:
	node scripts/gen-credits.mjs

license-gate:
	node scripts/license-gate.mjs
