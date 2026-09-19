#!/bin/bash
# Install (or refresh) the daily telemetry retention job on the Mac mini:
#   ~/spellgame-server/bin/telemetry-purge.sh + speak_report.py
#   ~/Library/LaunchAgents/net.spellgame.telemetry-purge.plist (04:17 daily)
# Separate from services.sh on purpose: that script rewrites every service's
# plist, the backend's secrets included. This one touches only its own job.
set -euo pipefail
SRV="$HOME/spellgame-server"
LA="$HOME/Library/LaunchAgents"
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
LABEL=net.spellgame.telemetry-purge
mkdir -p "$SRV/bin" "$LA"
install -m 755 "$REPO/scripts/mac-server/telemetry-purge.sh" "$SRV/bin/telemetry-purge.sh"
install -m 644 "$REPO/scripts/speak_report.py" "$SRV/bin/speak_report.py"
cat > "$LA/$LABEL.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>$LABEL</string>
  <key>ProgramArguments</key><array><string>/bin/bash</string><string>$SRV/bin/telemetry-purge.sh</string></array>
  <key>WorkingDirectory</key><string>$SRV</string>
  <key>StartCalendarInterval</key><dict><key>Hour</key><integer>4</integer><key>Minute</key><integer>17</integer></dict>
  <key>RunAtLoad</key><false/>
  <key>StandardOutPath</key><string>$SRV/logs/$LABEL.log</string>
  <key>StandardErrorPath</key><string>$SRV/logs/$LABEL.log</string>
</dict></plist>
PLIST
plutil -lint "$LA/$LABEL.plist" >/dev/null
launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "$LA/$LABEL.plist"
echo "install-telemetry-purge: $LABEL loaded (daily 04:17); log $SRV/logs/$LABEL.log"
