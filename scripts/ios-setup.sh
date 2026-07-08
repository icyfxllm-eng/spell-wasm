#!/usr/bin/env bash
#
# One-time iOS project tweaks that Capacitor's generated project doesn't set.
# Run ON THE MAC, once, right after `npx cap add ios`:
#
#   npx cap add ios && bash scripts/ios-setup.sh
#
# Idempotent — safe to re-run (e.g. after regenerating the ios/ folder). It:
#   1. Adds the microphone / speech usage strings to Info.plist.
#   2. Configures the audio session so word audio plays even when the phone's
#      ring/silent switch is on — for a hear-it-spell-it game, silencing the
#      audio would silence the whole game.
set -euo pipefail
cd "$(dirname "$0")/.."

PLIST="ios/App/App/Info.plist"
APPDELEGATE="ios/App/App/AppDelegate.swift"

if [ ! -f "$PLIST" ] || [ ! -f "$APPDELEGATE" ]; then
  echo "ERROR: iOS project not found. Run 'npx cap add ios' first (on a Mac)." >&2
  exit 1
fi

echo "==> Info.plist usage strings"
add_string_key() {
  if /usr/libexec/PlistBuddy -c "Print :$1" "$PLIST" >/dev/null 2>&1; then
    echo "    $1 already present"
  else
    /usr/libexec/PlistBuddy -c "Add :$1 string $2" "$PLIST"
    echo "    added $1"
  fi
}
# The mic / speech features are inert in the iOS WKWebView (SpeechRecognition
# isn't available there), but include the strings so review static analysis is
# happy if any mic-capable code path is ever reached.
add_string_key "NSMicrophoneUsageDescription" "Spell can listen so you can say your answer out loud."
add_string_key "NSSpeechRecognitionUsageDescription" "Spell uses speech recognition to check answers you speak aloud."

echo "==> AVAudioSession (.playback) in AppDelegate"
if grep -q "SPELL_AUDIO_SESSION" "$APPDELEGATE"; then
  echo "    already patched"
else
  # Ensure AVFoundation is imported.
  perl -0pi -e 's/(import UIKit)/$1\nimport AVFoundation/ unless /import AVFoundation/;' "$APPDELEGATE"
  # Insert the session setup at the top of didFinishLaunchingWithOptions.
  perl -0pi -e 's/(didFinishLaunchingWithOptions[^\{]*\{)/$1\n        \/\/ SPELL_AUDIO_SESSION: play word audio even with the ring\/silent switch on (audio IS the game).\n        do {\n            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)\n            try AVAudioSession.sharedInstance().setActive(true)\n        } catch { print("AVAudioSession error: \\(error)") }/s unless /SPELL_AUDIO_SESSION/;' "$APPDELEGATE"

  if grep -q "SPELL_AUDIO_SESSION" "$APPDELEGATE"; then
    echo "    patched AppDelegate.swift"
  else
    echo "    WARNING: could not auto-patch AppDelegate.swift — add this manually" >&2
    echo "    inside application(_:didFinishLaunchingWithOptions:), after the opening brace:" >&2
    cat >&2 <<'SNIPPET'
        do {
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
            try AVAudioSession.sharedInstance().setActive(true)
        } catch { print("AVAudioSession error: \(error)") }
    (and add `import AVFoundation` at the top)
SNIPPET
  fi
fi

echo
echo "Done. Still to do in Xcode (GUI, one-time):"
echo "  • Signing: select your Team; bundle id net.spellgame.app"
echo "  • Add capability: Associated Domains -> applinks:spellgame.net"
echo "  • Then: npx capacitor-assets generate --ios   (icons + splash)"
echo "  • Replace TEAMID in .well-known/apple-app-site-association and redeploy the site"
