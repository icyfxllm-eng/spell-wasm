#!/usr/bin/env python3
"""CC-PICKER-SEARCH one-shot registry migration (documented choice: one-shot
script, NOT load-time auto-migration — old-format entries become a CI failure
once this lands).

Adds per picture: categories (>=1), canonicalCategory (member), aliases.
Adds top-level: categoryList [{id, nameKey}] — the picker's shelf order.

Category assignment mirrors the retired family_of() EXCEPT it is now DATA:
- the four real masterpieces (mona, rhino, starrynight, redfuji) keep
  canonical "masters" and CROSS-LIST into their natural sibling family
  (D2's reference pattern);
- spiderweb (in-house original, no artist) moves to animals — Feature 3
  makes artist-less Masterpieces impossible by construction. FLAGGED for
  Eric's on-device pass.
"""
import json, pathlib

P = pathlib.Path("config/wordpic/pictures.json")
d = json.loads(P.read_text())

MASTER_SIBLING = {"mona": "world", "rhino": "animals", "starrynight": "sky", "redfuji": "world"}

ALIASES = {
 "smiley": ["face", "smile"], "star": ["stars"], "fish": [], "house": ["home"],
 "cat": ["kitten"], "rocket": ["spaceship"], "snowman": [], "dragon": [],
 "eiffel": ["eiffel tower", "paris", "tower"], "mona": ["mona lisa", "la gioconda", "da vinci", "leonardo"],
 "dog": ["puppy"], "butterfly": [], "snail": [], "duck": [], "owl": [], "turtle": ["tortoise"],
 "elephant": [], "horse": ["pony"], "peacock": [], "rhino": ["rhinoceros", "durer", "dürer"],
 "balloon": [], "mug": ["cup"], "mushroom": [], "sailboat": ["boat", "sail"],
 "cactus": [], "moonstar": ["moon", "crescent"], "kite": [], "ladybug": ["ladybird"],
 "bicycle": ["bike"], "windmill": [], "hotair": ["hot air balloon", "balloon"],
 "lighthouse": [], "violin": ["fiddle"], "hummingbird": [], "rooster": ["chicken"],
 "seahorse": [], "wolf": [], "koi": ["carp", "koi fish"], "octopus": [], "oak": ["tree", "oak tree"],
 "starrynight": ["starry night", "van gogh", "gogh"], "crane": ["origami"],
 "dallah": ["coffee pot"], "basil": ["herb"], "panda": [], "calavera": ["skull", "day of the dead"],
 "tajmahal": ["taj mahal", "india"], "stork": [], "caravel": ["ship"], "bigben": ["big ben", "london", "clock tower"],
 "turtleship": ["turtle ship", "geobukseon"], "baobab": ["tree"], "nonla": ["conical hat", "non la"],
 "cuckoo": ["cuckoo clock", "clock"], "torii": ["gate", "shrine"], "hanbok": ["dress"],
 "matryoshka": ["nesting doll", "russian doll"], "greatwall": ["great wall", "china", "wall"],
 "oud": ["lute"], "orion": ["constellation"], "papelpicado": ["papel picado", "banner"],
 "redfuji": ["red fuji", "hokusai", "fuji", "mount fuji"], "anubis": ["egypt"],
 "athenaowl": ["athena", "owl"], "bamboo": [], "girih": ["pattern"], "gyenyame": ["gye nyame", "adinkra"],
 "arcdetriomphe": ["arc de triomphe", "paris"], "rickshaw": [], "dhow": ["boat"],
 "galo": ["rooster", "barcelos"], "pyramids": ["pyramid", "giza", "egypt"],
 "eyeofhorus": ["eye of horus", "egypt"], "ankh": ["egypt"], "waweldragon": ["wawel", "dragon"],
 "doubledecker": ["double decker", "bus", "london"], "halongbay": ["ha long bay", "vietnam"],
 "fleurdelis": ["fleur de lis", "lily"], "manekineko": ["maneki neko", "lucky cat", "cat"],
 "acacia": ["tree"], "samovar": [], "lantern": [], "sitar": [], "brandenburg": ["brandenburg gate", "berlin"],
 "colosseum": ["rome", "coliseum"], "fan": ["folding fan"], "stonehenge": [], "pagoda": [],
 "daruma": ["doll"], "balalaika": [], "sombrero": ["hat"], "djembe": ["drum"],
 "angkorwat": ["angkor wat", "cambodia"], "accordion": [], "quetzal": ["bird"],
 "grandpiano": ["grand piano", "piano"], "chichenitza": ["chichen itza", "pyramid", "mexico"],
 "montsaintmichel": ["mont saint michel", "france"], "maasaishield": ["maasai", "shield"],
 "janggu": ["drum"], "koinobori": ["carp streamer", "koi"], "nazcabird": ["nazca", "bird"],
 "terracotta": ["terracotta warrior", "soldier"], "hamsa": ["hand"], "pinata": ["piñata"],
 "bagpipes": [], "triskele": ["triskelion", "spiral"], "mancala": ["board game"],
 "compassrose": ["compass", "rose"], "scarab": ["beetle", "egypt"], "gramophone": ["phonograph", "record player"],
 "kitsune": ["fox"], "tulip": ["flower"], "hourglass": ["sand timer"], "dartboard": ["darts", "bullseye"],
 "penguin": [], "anchor": ["boat anchor"], "stoplight": ["traffic light"], "snowflake": [],
 "aries": ["ram", "zodiac"], "taurus": ["bull", "zodiac"], "gemini": ["twins", "zodiac"],
 "cancer": ["crab", "zodiac"], "leo": ["lion", "zodiac"], "virgo": ["zodiac"],
 "libra": ["scales", "zodiac"], "scorpio": ["scorpion", "zodiac"], "sagittarius": ["archer", "zodiac"],
 "capricorn": ["goat", "zodiac"], "aquarius": ["water bearer", "zodiac"], "pisces": ["fish", "zodiac"],
 "puzzlepiece": ["puzzle", "jigsaw"], "spiderweb": ["spider web", "web", "spider"],
 "castle": [], "ferriswheel": ["ferris wheel"], "robot": [], "icecream": ["ice cream"],
 "crown": [], "umbrella": [], "key": [], "submarine": [], "crystalball": ["crystal ball"], "clock": [],
}

def family(p):
    pk = p["pack"]
    if pk == "numbers" or pk == "hanziNum" or pk.startswith("alpha"):
        return "learn"
    if pk.startswith("culture") or pk == "worldart":
        return "world"
    if pk == "animals":
        return "animals"
    if pk == "sky" or pk.startswith("zodiac"):
        return "sky"
    return "things"

for p in d["pictures"]:
    assert "categories" not in p, "already migrated"
    pid = p["id"]
    if pid in MASTER_SIBLING:
        p["categories"] = ["masters", MASTER_SIBLING[pid]]
        p["canonicalCategory"] = "masters"
    elif pid == "spiderweb":
        p["categories"] = ["animals"]
        p["canonicalCategory"] = "animals"
    else:
        f = family(p)
        p["categories"] = [f]
        p["canonicalCategory"] = f
    p["aliases"] = ALIASES.get(pid, [])

d["categoryList"] = [
    {"id": "learn", "nameKey": "wordpic.famLearn"},
    {"id": "world", "nameKey": "wordpic.famWorld"},
    {"id": "animals", "nameKey": "wordpic.famAnimals"},
    {"id": "sky", "nameKey": "wordpic.famSky"},
    {"id": "things", "nameKey": "wordpic.famThings"},
    {"id": "masters", "nameKey": "wordpic.famMasters"},
]
P.write_text(json.dumps(d, ensure_ascii=False, indent=1))
print("migrated:", len(d["pictures"]), "pictures; categoryList:", len(d["categoryList"]))
