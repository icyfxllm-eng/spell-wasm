#!/usr/bin/env bash
#
# Build the AUDIT PREVIEW web bundle into dist-audit/ — a private build where the
# ComingSoon languages (ar/fa/ur/ru + the already-banked es/fr/de/…) are unlocked
# and playable, so native speakers can review rendering, keyboards, feedback, and
# the DRAFT word banks (assets/words-draft/, unverified CC BY).
#
# NEVER production. The `audit_preview` feature is what unlocks everything; with it
# off (scripts/build-web.sh, the TestFlight build) RTL_SUPPORTED is false, the
# languages stay gated, and the draft banks are not compiled in. This script also
# stamps a persistent "UNVERIFIED PREVIEW" banner into the page so no one mistakes
# an audit build for the real app.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
DIST="$ROOT/dist-audit"

echo "==> regenerate draft banks -> src/word_data_audit.rs"
python3 scripts/build-draft-wordbanks.py

echo "==> cargo build (release, wasm32, --features audit_preview)"
cargo build --release --target wasm32-unknown-unknown --features audit_preview

echo "==> wasm-bindgen -> pkg-audit/"
wasm-bindgen target/wasm32-unknown-unknown/release/spell_wasm.wasm \
  --out-dir "$ROOT/pkg-audit" --target web --no-typescript

echo "==> assembling dist-audit/"
rm -rf "$DIST"
mkdir -p "$DIST"
cp index.html audio-native.js native-language-kit.js manifest.json sw.js "$DIST/"
cp -r icons "$DIST/icons"
cp -r fonts "$DIST/fonts"
cp -r pkg-audit "$DIST/pkg"

# Stamp the banner: a fixed, unmissable strip + a <title> marker. Injected here (not
# in index.html) so it exists ONLY in the audit bundle, never in the shipped page.
echo "==> stamping UNVERIFIED PREVIEW banner"
python3 - "$DIST/index.html" <<'PY'
import sys
p = sys.argv[1]
html = open(p, encoding="utf-8").read()
banner = (
  '<style>#auditPreviewBanner{position:fixed;top:0;left:0;right:0;z-index:99999;'
  'background:#b03a5b;color:#fff;font:600 12px/1.4 -apple-system,system-ui,sans-serif;'
  'text-align:center;padding:5px 10px;letter-spacing:.02em}'
  'body{padding-top:26px!important}</style>'
  '<div id="auditPreviewBanner">AUDIT PREVIEW · UNVERIFIED DRAFT CONTENT · NOT FOR RELEASE</div>'
)
html = html.replace("<body>", "<body>" + banner, 1) if "<body>" in html else banner + html
html = html.replace("<title>", "<title>[AUDIT] ", 1)
open(p, "w", encoding="utf-8").write(html)
print("   banner + title marker stamped")
PY

# Stamp the FLAG WIDGET: a floating control that turns the auditor's red-pen pass
# into a machine-usable cut list. It reads the just-revealed word from
# #feedback[data-audit-word] (set only under the audit_preview feature, game.rs),
# stores flags per language in localStorage, and exports a file that
# scripts/ingest-audit-flags.py folds into assets/words/exclusions/. Injected here,
# like the banner, so it lives ONLY in the audit bundle.
echo "==> stamping auditor flag widget"
python3 - "$DIST/index.html" <<'PY'
import sys
p = sys.argv[1]
html = open(p, encoding="utf-8").read()
widget = r'''<style>
#auditFlag{position:fixed;right:12px;bottom:12px;z-index:99998;display:flex;gap:6px;align-items:flex-end;flex-direction:column;font:600 13px/1.3 -apple-system,system-ui,sans-serif}
#auditFlag button{cursor:pointer;border:0;border-radius:8px;padding:9px 12px;color:#fff;background:#b03a5b;box-shadow:0 2px 8px rgba(0,0,0,.25)}
#auditFlag #afFlag{font-size:14px}
#afBadge{background:#222;border-radius:999px;padding:1px 7px;font-size:11px;margin-left:6px}
#afPanel{display:none;max-width:min(84vw,340px);max-height:52vh;overflow:auto;background:#fff;color:#222;border-radius:10px;padding:10px 12px;box-shadow:0 6px 24px rgba(0,0,0,.3)}
#afPanel h4{margin:0 0 6px;font-size:12px;letter-spacing:.03em;text-transform:uppercase;color:#b03a5b}
#afPanel .afLang{margin:8px 0 2px;font-weight:700}
#afPanel .afWord{display:flex;justify-content:space-between;gap:8px;padding:2px 0;font-weight:400}
#afPanel .afWord button{background:#ccc;color:#333;padding:0 6px;border-radius:6px;font-size:11px}
#afPanel .afRow{display:flex;gap:6px;margin-top:10px}
#afToast{position:fixed;right:12px;bottom:64px;z-index:99999;background:#222;color:#fff;padding:7px 11px;border-radius:8px;font:600 12px/1.3 system-ui;opacity:0;transition:opacity .2s;pointer-events:none}
</style>
<div id="auditFlag" dir="ltr">
  <div id="afPanel"></div>
  <div><button id="afToggle" type="button" title="Show flagged words">Flags<span id="afBadge">0</span></button>
  <button id="afFlag" type="button" title="Flag the word currently shown">&#9873; Flag word</button></div>
</div>
<div id="afToast"></div>
<script>
(function(){
  var KEY='audit_flags_v1';
  function load(){try{return JSON.parse(localStorage.getItem(KEY))||{}}catch(e){return {}}}
  function save(d){localStorage.setItem(KEY,JSON.stringify(d))}
  function total(d){return Object.keys(d).reduce(function(n,l){return n+d[l].length},0)}
  var badge=document.getElementById('afBadge'),panel=document.getElementById('afPanel'),toastEl=document.getElementById('afToast'),tt;
  function toast(m){toastEl.textContent=m;toastEl.style.opacity=1;clearTimeout(tt);tt=setTimeout(function(){toastEl.style.opacity=0},1600)}
  function current(){var fb=document.getElementById('feedback');if(!fb)return null;var w=fb.getAttribute('data-audit-word'),l=fb.getAttribute('data-audit-lang');return (w&&l)?{word:w,lang:l}:null}
  function render(){var d=load();badge.textContent=total(d);if(panel.style.display==='none')return;
    var h='<h4>Flagged &mdash; cut on review</h4>';var langs=Object.keys(d).sort();
    if(!langs.length)h+='<div style="font-weight:400">None yet. Play a word, then tap &#9873; after it is revealed.</div>';
    langs.forEach(function(l){if(!d[l].length)return;h+='<div class="afLang" dir="auto">'+l+'</div>';
      d[l].forEach(function(w){h+='<div class="afWord"><span dir="auto">'+w.replace(/[&<>]/g,function(c){return{'&':'&amp;','<':'&lt;','>':'&gt;'}[c]})+'</span><button data-l="'+l+'" data-w="'+encodeURIComponent(w)+'">remove</button></div>'})});
    h+='<div class="afRow"><button id="afExport" type="button">Export</button><button id="afClear" type="button" style="background:#888">Clear all</button></div>';
    panel.innerHTML=h;
    panel.querySelectorAll('.afWord button').forEach(function(b){b.onclick=function(){var d=load(),l=b.getAttribute('data-l'),w=decodeURIComponent(b.getAttribute('data-w'));d[l]=(d[l]||[]).filter(function(x){return x!==w});if(!d[l].length)delete d[l];save(d);render()}});
    document.getElementById('afExport').onclick=exportFlags;
    document.getElementById('afClear').onclick=function(){if(confirm('Clear all flagged words?')){save({});render()}};
  }
  function exportFlags(){var d=load();var out='# audit flags — words a native reviewer marked to cut\n# ingest with: python3 scripts/ingest-audit-flags.py audit-flags.txt\n';
    Object.keys(d).sort().forEach(function(l){if(d[l].length){out+='['+l+']\n'+d[l].slice().sort().join('\n')+'\n'}});
    var a=document.createElement('a');a.href=URL.createObjectURL(new Blob([out],{type:'text/plain'}));a.download='audit-flags.txt';a.click();toast('Exported '+total(d)+' flags')}
  document.getElementById('afFlag').onclick=function(){var c=current();if(!c){toast('Play a word first, then flag after it is revealed');return}
    var d=load();d[c.lang]=d[c.lang]||[];if(d[c.lang].indexOf(c.word)<0){d[c.lang].push(c.word);save(d);toast('Flagged: '+c.word)}else toast('Already flagged');render()};
  document.getElementById('afToggle').onclick=function(){panel.style.display=panel.style.display==='none'?'block':'none';render()};
  render();
})();
</script>'''
html = html.replace("</body>", widget + "</body>", 1) if "</body>" in html else html + widget
open(p, "w", encoding="utf-8").write(html)
print("   flag widget stamped")
PY

echo "==> dist-audit/ ready — UNVERIFIED PREVIEW, do not distribute as release"
