"""CC-TELEMETRY-FOUNDATION v1.1 R8 -- one structured line per /api/speak.

F5 wants TTS latency, cache hit rate and error rate per language, and the
server logged none of it (census item 5). This writes exactly one JSON line per
request:

    {"lang": "ru", "variant": "normal", "cache": "hit", "status": 200, "ms": 3.1}

WHAT IS NOT IN IT: no IP, no word, no pinyin, no user agent, no timestamp. The
day is the file name (UTC), and that is all the report needs.

WHERE: one file per UTC day, `speak-YYYY-MM-DD.jsonl`, in SPEAK_METRICS_DIR, or
by default `../logs/speak` beside the backend when that `logs` directory exists
(the Mac mini layout). Anywhere else, dev checkouts and tests, it is off unless
set explicitly.

Each line is one O_APPEND write of well under PIPE_BUF, so gunicorn's two
workers can't interleave partial lines. Old files are pruned by
scripts/speak_report.py --prune (90 days, D6).

Never in the way: any failure to write is swallowed. A metrics problem must
never turn into a failed /api/speak.
"""
import json
import os
import time

VARIANTS = ("normal", "slow")
CACHE = ("hit", "miss")


def metrics_dir():
    d = os.environ.get("SPEAK_METRICS_DIR")
    if d:
        return d
    logs = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "logs")
    return os.path.join(logs, "speak") if os.path.isdir(logs) else None


def line(lang, variant, cache, status, ms):
    """The record, validated to closed values; None if anything is off-list."""
    if variant not in VARIANTS or cache not in CACHE:
        return None
    if not (isinstance(lang, str) and lang.isascii() and lang.isalpha() and 2 <= len(lang) <= 3):
        lang = "other"
    return json.dumps(
        {"lang": lang, "variant": variant, "cache": cache, "status": int(status), "ms": round(float(ms), 1)},
        separators=(",", ":"),
    )


def record(lang, variant, cache, status, ms, now=None):
    try:
        d = metrics_dir()
        if not d:
            return
        text = line(lang, variant, cache, status, ms)
        if text is None:
            return
        os.makedirs(d, exist_ok=True)
        day = time.strftime("%Y-%m-%d", time.gmtime(now if now is not None else time.time()))
        fd = os.open(os.path.join(d, f"speak-{day}.jsonl"), os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o644)
        try:
            os.write(fd, (text + "\n").encode())
        finally:
            os.close(fd)
    except Exception:  # noqa: BLE001 -- never in the way (I1)
        pass
