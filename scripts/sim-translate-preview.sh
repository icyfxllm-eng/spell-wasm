#!/usr/bin/env bash
#
# Spell Translate preview in the iOS Simulator (CC-TRANSLATE-SCREEN).
#
# Installs a SEPARATE app, "Spell TR" (net.spellgame.app.trpreview), beside any
# normal Spell install. It is the testseam build inside the real iOS shell,
# with the translate flag on and Spanish + Japanese gloss rows treated as
# audited through translate::seam_set_audited. That seam flips a flag on rows
# that already exist and cannot author one, so every word shown is a real,
# UNREVIEWED row. Nothing here can reach a production or TestFlight build: the
# seam is compiled out of those, and this script never touches the caller's
# checkout. It builds in a throwaway worktree and deletes it afterwards.
#
#   bash scripts/sim-translate-preview.sh                  # origin/main, iPhone 17 Pro
#   REF=my-branch SIM="iPhone 17e" bash scripts/sim-translate-preview.sh
#   TR_LANGS="es" bash scripts/sim-translate-preview.sh    # Spanish only
#
# The cargo target lives in ~/.cache/spell-tr-preview, never shared with
# another checkout, so builds after the first one are incremental.
set -euo pipefail

REPO="${REPO:-$(cd "$(dirname "$0")/.." && pwd)}"
REF="${REF:-origin/main}"
SIM="${SIM:-iPhone 17 Pro}"
TR_LANGS="${TR_LANGS:-es ja}"
BUNDLE="net.spellgame.app.trpreview"
CACHE="$HOME/.cache/spell-tr-preview"
WT="$(mktemp -d "${TMPDIR:-/tmp}/spell-tr-preview.XXXXXX")/wt"

cleanup() { git -C "$REPO" worktree remove --force "$WT" 2>/dev/null || true; }
trap cleanup EXIT

echo "==> worktree at $REF"
git -C "$REPO" fetch -q origin || true
git -C "$REPO" worktree add -q --detach "$WT" "$REF"
ln -s "$REPO/node_modules" "$WT/node_modules"
# Lexicons and other gitignored build inputs under data/.
(cd "$REPO" && git status --ignored --short data | sed -n 's/^!! //p' | sed 's#/$##') |
  while read -r p; do
    [ -e "$WT/$p" ] || { mkdir -p "$WT/$(dirname "$p")"; ln -s "$REPO/$p" "$WT/$p"; }
  done

cd "$WT"
mkdir -p "$CACHE/target"
ln -s "$CACHE/target" target
bash scripts/build-web-test.sh

echo "==> injecting the preview switches into dist-test/index.html"
LANGS_JS="$(printf '"%s",' $TR_LANGS)"
python3 - "dist-test/index.html" "[${LANGS_JS%,}]" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1]); langs = sys.argv[2]
tag = f"""<script>
/* Spell TR preview only -- injected by scripts/sim-translate-preview.sh */
try {{ localStorage.setItem('spell_flag_translate', 'on'); }} catch (e) {{}}
(function arm() {{
  var t = window.__spelltest;
  if (!t || !t.translateAudit) return setTimeout(arm, 50);
  {langs}.forEach(function (l) {{ t.translateAudit(l, true); }});
  console.warn('[tr-preview] translate on, audited: ' + {langs}.join(','));
}})();
</script>"""
src = p.read_text()
assert "<head>" in src
p.write_text(src.replace("<head>", "<head>\n" + tag, 1))
PY

echo "==> cap sync ios (webDir dist-test, this worktree only)"
python3 - <<'PY'
import json, pathlib
p = pathlib.Path("capacitor.config.json"); c = json.loads(p.read_text())
c["webDir"] = "dist-test"
p.write_text(json.dumps(c, indent=2))
PY
npx cap sync ios >/dev/null

echo "==> xcodebuild (Debug, $SIM)"
UDID="$(xcrun simctl list devices available | grep -F "    $SIM (" | head -1 | sed -E 's/.*\(([0-9A-F-]{36})\).*/\1/')"
[ -n "$UDID" ] || { echo "no simulator named '$SIM'"; exit 1; }
xcodebuild -project ios/App/App.xcodeproj -scheme App -configuration Debug \
  -destination "id=$UDID" -derivedDataPath "$CACHE/DerivedData" \
  CODE_SIGN_IDENTITY=- CODE_SIGNING_REQUIRED=NO CODE_SIGNING_ALLOWED=NO \
  -quiet build

# Make the built copy its own app so it never replaces a real Spell install:
# new id and name, no widget extension (its id must nest under the app's),
# then an ad-hoc signature for the Simulator.
APP="$CACHE/Spell TR.app"
rm -rf "$APP"
cp -R "$CACHE/DerivedData/Build/Products/Debug-iphonesimulator/App.app" "$APP"
rm -rf "$APP/PlugIns"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $BUNDLE" "$APP/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName Spell TR" "$APP/Info.plist" 2>/dev/null ||
  /usr/libexec/PlistBuddy -c "Add :CFBundleDisplayName string Spell TR" "$APP/Info.plist"
codesign --force --deep --sign - "$APP"
echo "==> install + launch $BUNDLE on $SIM"
# Boot (or confirm booted) and WAIT: a bare "simctl boot" returns before the
# device can take an install.
xcrun simctl bootstatus "$UDID" -b >/dev/null
open -a Simulator
xcrun simctl install "$UDID" "$APP"
xcrun simctl launch "$UDID" "$BUNDLE" >/dev/null
echo "Spell TR is running on $SIM. Open the hub: Spell Translator tiles last."
echo "Audited for this preview (unreviewed rows): $TR_LANGS"
