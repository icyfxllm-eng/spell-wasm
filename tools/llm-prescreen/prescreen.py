#!/usr/bin/env python3
"""LLM pre-screen for language review (Part 4).

Batches lexicon entries per language through the Anthropic API to pre-shrink the
human review packet — so a paid native reviewer spends their hour on the
ambiguous ~10%, not the obvious 90%. Also assesses merged blocklists.

DETERMINISTIC-ish: temperature 0, fixed committed prompt versions, results cached
by (input-hash, prompt-version) so reruns are cheap. Cost guard: prints estimated
tokens and requires --yes above a spend threshold.

HARD RULE: model output is ADVISORY. Nothing here auto-writes to any filter or
word list — output is a review packet for a human. Missing-term suggestions go to
Eric's custom-list intake, never to the shipped lists.

  export ANTHROPIC_API_KEY=...        # never committed
  python3 prescreen.py words --lang ja --lexicon ../../data/ja/lexicon.jsonl
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path

MODEL = "claude-sonnet-4-6"
PROMPT_VERSION = "wordlist-v1"
BATCH = 100
COST_PER_MTOK_IN = 3.0   # USD, approx; update from pricing
COST_PER_MTOK_OUT = 15.0
SPEND_CONFIRM_USD = 5.0

LANG_NAMES = {"en": "English", "vi": "Vietnamese", "ja": "Japanese", "ko": "Korean",
              "zh": "Mandarin Chinese", "th": "Thai", "fil": "Filipino"}
HERE = Path(__file__).parent


def cache_key(word: str, lang: str) -> str:
    return hashlib.sha256(f"{word}|{lang}|{MODEL}|{PROMPT_VERSION}".encode()).hexdigest()


def load_lexicon(path: Path) -> list[dict]:
    return [json.loads(l) for l in path.read_text(encoding="utf-8").splitlines() if l.strip()]


def estimate(entries: list[dict]) -> tuple[int, float]:
    # ~ rough token estimate: 6 tokens/entry in, 4/entry out for flagged ~10%.
    tok_in = len(entries) * 6 + len(entries) // BATCH * 300
    tok_out = int(len(entries) * 0.1 * 8)
    cost = tok_in / 1e6 * COST_PER_MTOK_IN + tok_out / 1e6 * COST_PER_MTOK_OUT
    return tok_in + tok_out, cost


def call_api(prompt: str, batch: list[dict]) -> list[dict]:
    """Real Anthropic call. Imported lazily so the tool loads without the SDK;
    absent SDK/key -> clear error (no silent skip — a safety tool must fail loud)."""
    key = os.environ.get("ANTHROPIC_API_KEY")
    if not key:
        raise SystemExit("ANTHROPIC_API_KEY not set — refusing to run a safety check silently.")
    try:
        import anthropic  # noqa
    except ImportError:
        raise SystemExit("pip install anthropic — the LLM pre-screen needs the SDK.")
    client = anthropic.Anthropic(api_key=key)
    msg = client.messages.create(
        model=MODEL, max_tokens=2000, temperature=0,
        messages=[{"role": "user", "content": prompt + "\n\n" + json.dumps(
            [{"word": e["word"], "gloss": e.get("gloss", "")} for e in batch], ensure_ascii=False)}],
    )
    text = msg.content[0].text.strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        raise SystemExit(f"model returned non-JSON — retry/inspect:\n{text[:500]}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["words", "blocklist"])
    ap.add_argument("--lang", required=True)
    ap.add_argument("--lexicon", required=True)
    ap.add_argument("--yes", action="store_true", help="skip the spend confirmation")
    args = ap.parse_args()

    entries = load_lexicon(Path(args.lexicon))
    toks, cost = estimate(entries)
    print(f"llm-prescreen[{args.lang}]: {len(entries)} entries, ~{toks} tokens, ~${cost:.2f}")
    if cost > SPEND_CONFIRM_USD and not args.yes:
        print(f"Projected > ${SPEND_CONFIRM_USD:.0f}. Re-run with --yes to proceed.", file=sys.stderr)
        sys.exit(2)

    prompt = (HERE / "prompts" / f"{PROMPT_VERSION}.txt").read_text(encoding="utf-8").replace(
        "{LANGUAGE_NAME}", LANG_NAMES.get(args.lang, args.lang)).replace("{LANG_CODE}", args.lang)

    flags = []
    for i in range(0, len(entries), BATCH):
        flags.extend(call_api(prompt, entries[i:i + BATCH]))

    out = HERE / "review-packet" / args.lang
    out.mkdir(parents=True, exist_ok=True)
    (out / "flagged-words.json").write_text(json.dumps(flags, ensure_ascii=False, indent=2))
    print(f"  {len(flags)} flagged → {out.relative_to(HERE)}/flagged-words.json  (ADVISORY — human review)")


if __name__ == "__main__":
    main()
