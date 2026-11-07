#!/usr/bin/env bash
# CC-LAUNCH-EN A4.9 / I4.7 / item 5: no hardcoded price ships. Prices come only
# from StoreKit's displayPrice. The one allowed home is the local StoreKit
# test configuration (ios/App/App/StoreKit/SpellGame.storekit), which is a price
# table by design and never ships in the bundle.
set -euo pipefail
cd "$(dirname "$0")/.."
hits=$(git grep -nE '\$ ?(4\.99|12\.99|7\.99)|(4\.99|12\.99|7\.99) ?(USD|\$)' -- \
  src index.html privacy.html notices.html '*.js' ios/App/App store fastlane \
  ':!*.storekit' ':!node_modules' || true)
if [ -n "$hits" ]; then
  echo "price-grep: FAIL. Hardcoded price found:"
  echo "$hits"
  exit 1
fi
echo "price-grep: OK"
