# Word-list provenance pipeline (CC-WORDLIST-SOURCES).
#
#   make wordlist LANG=es     # fetch -> verify -> unmunch -> filter -> emit
#   make credits              # regenerate credits.json from sources/registry.json
#   make license-gate         # run the CI license gate locally
#
# REVIEW-GATED: these targets produce wordlists/<lang>.txt + a manifest for
# Eric's review. They do NOT touch assets/words/ or src/ (the shipped list).

LANG ?= es

.PHONY: wordlist credits license-gate

wordlist:
	./scripts/wordlist.sh $(LANG)

credits:
	node scripts/gen-credits.mjs

license-gate:
	node scripts/license-gate.mjs
