#!/usr/bin/env python3
"""D3 Proposal A (SIGNED by Eric): split "Around the World" into
Landmarks / Music / Symbols & Patterns / remainder-keeps-the-name.
Zodiac stays in sky (Eric). Also repairs the inherited misfilings the
split surfaced: late-batch generic subjects that rode cultureN packs
into "world", plus eiffel/violin which belong on the new shelves.
Registry edits ONLY — the picker needs zero code (Feature 7 proven)."""
import json, pathlib

P = pathlib.Path("config/wordpic/pictures.json")
d = json.loads(P.read_text())

LANDMARKS = {"eiffel", "tajmahal", "bigben", "greatwall", "arcdetriomphe", "pyramids",
             "halongbay", "brandenburg", "colosseum", "stonehenge", "pagoda", "angkorwat",
             "chichenitza", "montsaintmichel", "terracotta", "torii"}
MUSIC = {"violin", "oud", "sitar", "balalaika", "djembe", "janggu", "bagpipes",
         "accordion", "grandpiano", "gramophone"}
SYMBOLS = {"anubis", "athenaowl", "girih", "gyenyame", "eyeofhorus", "ankh", "fleurdelis",
           "hamsa", "triskele", "compassrose", "scarab", "calavera", "nazcabird"}
# Late-batch generics that never belonged in "world":
TO_THINGS = {"tulip", "hourglass", "dartboard", "anchor", "stoplight", "snowflake",
             "puzzlepiece", "castle", "ferriswheel", "robot", "icecream", "crown",
             "umbrella", "key", "submarine", "crystalball", "clock"}
TO_ANIMALS = {"penguin"}

def home(pid):
    if pid in LANDMARKS: return "landmarks"
    if pid in MUSIC: return "music"
    if pid in SYMBOLS: return "symbols"
    if pid in TO_THINGS: return "things"
    if pid in TO_ANIMALS: return "animals"
    return None

moved = 0
for p in d["pictures"]:
    h = home(p["id"])
    if h is None:
        continue
    if "masters" in p["categories"]:
        # redfuji: masters stays canonical; the sibling shelf updates.
        p["categories"] = ["masters", h]
    else:
        p["categories"] = [h]
        p["canonicalCategory"] = h
    moved += 1

d["categoryList"] = [
    {"id": "learn", "nameKey": "wordpic.famLearn"},
    {"id": "world", "nameKey": "wordpic.famWorld"},
    {"id": "landmarks", "nameKey": "wordpic.famLandmarks"},
    {"id": "music", "nameKey": "wordpic.famMusic"},
    {"id": "symbols", "nameKey": "wordpic.famSymbols"},
    {"id": "animals", "nameKey": "wordpic.famAnimals"},
    {"id": "sky", "nameKey": "wordpic.famSky"},
    {"id": "things", "nameKey": "wordpic.famThings"},
    {"id": "masters", "nameKey": "wordpic.famMasters"},
]
P.write_text(json.dumps(d, ensure_ascii=False, indent=1))
import collections
c = collections.Counter()
for p in d["pictures"]:
    for cat in p["categories"]: c[cat] += 1
print("moved:", moved, "| shelves:", dict(c))
