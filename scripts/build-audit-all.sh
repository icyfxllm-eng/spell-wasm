#!/usr/bin/env bash
#
# Build one AUDIT PREVIEW bundle per non-English language: dist-audit-<lang>/, each
# locked to its language (see scripts/build-web-audit.sh). The list is read from
# the registry (consts::LANGS_BASE), so a newly registered language gets a build
# without editing this file. Pass codes to build a subset: build-audit-all.sh fr de
set -euo pipefail

cd "$(dirname "$0")/.."

if [ "$#" -gt 0 ]; then
  LANGS=("$@")
else
  LANGS=()
  while IFS= read -r l; do LANGS+=("$l"); done < <(python3 - <<'PY'
import re
src = open("src/consts.rs", encoding="utf-8").read()
consts = dict(re.findall(r'pub const ([A-Z]+): &str = "([a-z]+)";', src))
base = src.split("const LANGS_BASE", 1)[1].split("];", 1)[0]
for name in re.findall(r'^\s*\(([A-Z]+),', base, re.M):
    if consts[name] != "en":
        print(consts[name])
PY
)
fi

echo "==> audit builds for: ${LANGS[*]}"
for l in "${LANGS[@]}"; do
  AUDIT_LANG="$l" bash scripts/build-web-audit.sh
done
echo "==> built ${#LANGS[@]} audit bundles: $(printf 'dist-audit-%s/ ' "${LANGS[@]}")"
