#!/usr/bin/env python3
"""CC-PRACTICE v2 D10 — augment the v1 curricula with the interaction layer:

  * traps[].decoys      — tile-tray decoy hints (D3), authored per trap class
  * traps[].template    — micro-interaction refs (D5): TAP_SILENT_UNIT /
                          HEAR_PICK / TAP_STACK; traps without a fitting
                          template keep the plain intro card (spec fallback)
  * choiceBeats         — D8: at positions 4/8/12/16 (each new trap block),
                          words[at] vs a same-class alt from the bank;
                          curated emoji pair per beat
  * coach               — D4: per-language coach line pools (≤40 total),
                          agent-drafted, riding the standing Fiverr audit

v1 word selections / trap intros are PRESERVED — this script only adds.
Idempotent: re-running regenerates the v2 fields from scratch.
Validate: node scripts/practice-check.mjs
"""
import importlib.util
import json
import os
import re
import unicodedata

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "config", "practice")

spec = importlib.util.spec_from_file_location(
    "v1gen", os.path.join(ROOT, "scripts", "build-practice-curricula.py"))
v1 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(v1)  # main-guarded; only defs run

NFC = lambda s: unicodedata.normalize("NFC", s)

# ---- D3: authored decoy hints (single typing units). Missing entries are
# fine — the runtime falls back to the language's unit inventory. -------------
DECOYS = {
    "en": {"silent-e": ["a", "u"], "double-consonant": ["s", "t"], "ie-ei": ["a", "y"],
           "hard-c-k": ["q", "x"], "wh-th-sh": ["f", "v"]},
    "es": {"b-v": ["b", "v"], "h-muda": ["j", "g"], "ll-y": ["i", "j"],
           "tildes": ["a", "e"], "c-s-z": ["k", "q"]},
    "fr": {"finales-muettes": ["d", "x"], "accents": ["é", "è"], "eau-au-o": ["ô", "w"],
           "doubles": ["m", "p"], "ou-u": ["w", "y"]},
    "de": {"grossschreibung": ["x", "y"], "ie-ei": ["j", "y"], "umlaute": ["ä", "ö"],
           "sch-ch": ["z", "k"], "doppel": ["p", "b"]},
    "pt": {"nasais": ["ã", "õ"], "acentos": ["á", "é"], "cedilha": ["c", "s"],
           "lh-nh": ["y", "w"], "s-z": ["s", "z"]},
    "pl": {"rz-z": ["ż", "z"], "o-u": ["ó", "u"], "ch-h": ["g", "k"],
           "nosowe": ["ą", "ę"], "miekkie": ["ś", "ć"]},
    "vi": {"ch-tr": ["c", "t"], "s-x": ["s", "x"], "d-gi-r": ["d", "r"]},
    "ru": {"unstressed-vowel-reduction": ["о", "а"], "soft-hard-sign": ["ь", "ъ"],
           "yo": ["е", "ё"], "doubled-consonant": ["т", "п"], "hushing-confusion": ["ы", "э"]},
    "ar": {"hamza": ["أ", "إ"], "taa-marbuta": ["ه", "ت"], "alif-maqsura": ["ي", "ى"],
           "long-vowels": ["ا", "و"]},
    "hi": {"matra": ["ि", "ी"], "retroflex": ["त", "द"], "aspirate": ["क", "ग"],
           "anusvara": ["ं", "ँ"], "halant": ["्"]},
    "sw": {"ng-apostrophe": ["g", "j"], "double-vowel": ["a", "u"], "l-r": ["l", "r"]},
    "fil": {"o-u": ["o", "u"], "d-r": ["d", "r"], "ng-digraph": ["m", "n"]},
    "ja": {"ha-wa": ["は", "わ"], "chiisai-tsu": ["つ", "っ"], "youon": ["や", "ゆ"]},
    "ko": {},   # jamo inventory fallback (runtime) is the right decoy source
    "zh": {"zh-z": ["z", "c"], "n-ng": ["n", "g"], "u-v": ["u", "v"]},
}

# ---- D5: template assignments. TAP_SILENT_UNIT{unit} must appear in the
# trap's first word; HEAR_PICK{foil} = regex swap on the first word (skipped
# if the swap is a no-op); TAP_STACK{unit,pair} for ko doubled jamo. ----------
SILENT = {  # lang -> trap -> unit ("LAST" = last unit of the first word)
    "en": {"silent-e": "e"}, "es": {"h-muda": "h"}, "fr": {"finales-muettes": "LAST"},
    "ru": {"soft-hard-sign": "ь"}, "hi": {"halant": "्"}, "ko": {"ieung": "ㅇ"},
}
HEARPICK = {  # lang -> trap -> (pattern, replacement, reverse-replacement)
    "en": {"ie-ei": ("ie", "ei")}, "es": {"b-v": ("b", "v")}, "de": {"ie-ei": ("ie", "ei")},
    "pt": {"s-z": ("s", "z")}, "pl": {"o-u": ("ó", "u")}, "vi": {"ch-tr": ("ch", "tr")},
    "ja": {"ha-wa": ("は", "わ")}, "zh": {"n-ng": ("ng", "n")}, "fil": {"d-r": ("d", "r")},
    "sw": {"ng-apostrophe": ("ng'", "ng")}, "ru": {"yo": ("ё", "е")},
    "ar": {"taa-marbuta": ("ة", "ه")},
}
SSANG = {"ㄲ": "ㄱㄱ", "ㄸ": "ㄷㄷ", "ㅃ": "ㅂㅂ", "ㅆ": "ㅅㅅ", "ㅉ": "ㅈㅈ"}

# ---- D8: emoji pairs. en pairs are word-curated; other languages rotate a
# neutral curated set (distinct, kid-friendly) — flagged for the audit round.
EN_EMOJI = {}  # filled per-beat below from word knowledge where obvious
NEUTRAL_EMOJI = [["🌟", "🎈"], ["🐢", "🐇"], ["🌊", "🔦"], ["🍀", "🌙"]]
EN_WORD_EMOJI = {
    "horse": "🐴", "kite": "🪁", "mouse": "🐭", "nose": "👃", "tree": "🌳",
    "book": "📖", "door": "🚪", "hill": "⛰️", "died": "🥀", "view": "🏞️",
    "die": "🎲", "lies": "🤥", "cat": "🐱", "cup": "☕", "milk": "🥛",
    "corn": "🌽", "fish": "🐟", "ship": "🚢", "the": "✨", "that": "👉",
}

# ---- D4: coach line pools (agent-drafted; ≤90 chars/line; ≤40 lines total).
COACH = {
 "en": {
  "wordDone": ["You built that one yourself!", "That's exactly right.", "See? You've got this.",
               "Another one done — nice and steady.", "You heard it and you spelled it."],
  "wordSetup": ["Here comes the next one — listen close.", "This one has a sneaky letter. Ready?",
                "Take your time with this one.", "Listen first, then build it."],
  "phase": ["New step: the word peeks out, then hides.", "Big step — ears only now. You're ready.",
            "Here's the real keyboard — just like the big game."],
  "reveal": ["There it is — that's the tricky part.", "No worries — now you know its secret."],
  "ceremony": ["Twenty words! Your first twenty — forever.", "Ready for the hard ones? Five more!"],
 },
 "es": {
  "wordDone": ["¡Lo armaste tú solo!", "Exactamente así.", "¿Ves? Puedes hacerlo.",
               "Otra más — paso a paso.", "La oíste y la escribiste."],
  "wordSetup": ["Viene la siguiente — escucha bien.", "Esta tiene una letra traviesa. ¿Listo?",
                "Tómate tu tiempo con esta.", "Primero escucha, luego construye."],
  "phase": ["Nuevo paso: la palabra se asoma y se esconde.", "Gran paso — solo con el oído. Ya puedes.",
            "Aquí está el teclado de verdad — como en el juego."],
  "reveal": ["Ahí está — esa es la parte difícil.", "Tranquilo — ya conoces su secreto."],
  "ceremony": ["¡Veinte palabras! Tus primeras veinte.", "¿Listo para las difíciles? ¡Cinco más!"],
 },
 "fr": {
  "wordDone": ["Tu l'as construit tout seul !", "Exactement !", "Tu vois ? Tu y arrives.",
               "Encore un — tranquillement.", "Tu l'as entendu, tu l'as écrit."],
  "wordSetup": ["Voici le prochain — écoute bien.", "Celui-ci a une lettre cachée. Prêt ?",
                "Prends ton temps.", "Écoute d'abord, puis construis."],
  "phase": ["Nouvelle étape : le mot se montre puis se cache.", "Grande étape — que les oreilles !",
            "Voici le vrai clavier — comme dans le grand jeu."],
  "reveal": ["Le voilà — c'est le piège.", "Pas grave — tu connais son secret."],
  "ceremony": ["Vingt mots ! Tes vingt premiers.", "Prêt pour les difficiles ? Cinq de plus !"],
 },
 "de": {
  "wordDone": ["Das hast du selbst gebaut!", "Genau richtig.", "Siehst du? Du kannst das.",
               "Noch eins geschafft — schön ruhig.", "Gehört und geschrieben!"],
  "wordSetup": ["Das nächste kommt — hör gut zu.", "Hier versteckt sich ein Buchstabe. Bereit?",
                "Lass dir Zeit.", "Erst hören, dann bauen."],
  "phase": ["Neuer Schritt: das Wort zeigt sich kurz.", "Großer Schritt — nur die Ohren!",
            "Hier ist die echte Tastatur — wie im großen Spiel."],
  "reveal": ["Da ist sie — die knifflige Stelle.", "Kein Problem — jetzt kennst du das Geheimnis."],
  "ceremony": ["Zwanzig Wörter! Deine ersten zwanzig.", "Bereit für die schweren? Noch fünf!"],
 },
 "pt": {
  "wordDone": ["Você montou sozinho!", "Exatamente!", "Viu? Você consegue.",
               "Mais uma — com calma.", "Você ouviu e escreveu."],
  "wordSetup": ["Vem a próxima — escute bem.", "Esta tem uma letra escondida. Pronto?",
                "Vá com calma nesta.", "Primeiro escute, depois monte."],
  "phase": ["Novo passo: a palavra aparece e se esconde.", "Passo grande — só de ouvido!",
            "Aqui está o teclado de verdade — como no jogo."],
  "reveal": ["Aí está — essa é a parte difícil.", "Tudo bem — agora você sabe o segredo."],
  "ceremony": ["Vinte palavras! Suas primeiras vinte.", "Pronto para as difíceis? Mais cinco!"],
 },
 "pl": {
  "wordDone": ["Ułożyłeś to sam!", "Dokładnie tak.", "Widzisz? Potrafisz.",
               "Kolejne gotowe — spokojnie.", "Usłyszałeś i napisałeś."],
  "wordSetup": ["Następne słowo — słuchaj uważnie.", "Tu kryje się podstępna litera. Gotowy?",
                "Nie spiesz się.", "Najpierw posłuchaj, potem układaj."],
  "phase": ["Nowy krok: słowo pokaże się i schowa.", "Wielki krok — tylko uszy!",
            "Oto prawdziwa klawiatura — jak w dużej grze."],
  "reveal": ["To tutaj — to jest pułapka.", "Spokojnie — znasz już jej sekret."],
  "ceremony": ["Dwadzieścia słów! Twoje pierwsze.", "Gotowy na trudne? Jeszcze pięć!"],
 },
 "vi": {
  "wordDone": ["Bạn tự ghép được rồi!", "Chính xác!", "Thấy chưa? Bạn làm được.",
               "Xong thêm một từ — từ từ thôi.", "Nghe và viết đúng rồi!"],
  "wordSetup": ["Từ tiếp theo — nghe kỹ nhé.", "Từ này có chữ khó đấy. Sẵn sàng chưa?",
                "Cứ từ từ với từ này.", "Nghe trước, ghép sau."],
  "phase": ["Bước mới: từ hiện ra rồi ẩn đi.", "Bước lớn — chỉ nghe thôi!",
            "Đây là bàn phím thật — như trò chơi chính."],
  "reveal": ["Đây rồi — chỗ khó là đây.", "Không sao — giờ bạn biết bí mật rồi."],
  "ceremony": ["Hai mươi từ! Hai mươi từ đầu tiên.", "Sẵn sàng cho từ khó? Thêm năm từ!"],
 },
 "ko": {
  "wordDone": ["혼자서 만들었어요!", "정확해요.", "봤죠? 할 수 있어요.",
               "하나 더 완성 — 차근차근.", "듣고 그대로 썼어요!"],
  "wordSetup": ["다음 단어예요 — 잘 들어 보세요.", "이 단어엔 까다로운 글자가 있어요. 준비됐나요?",
                "천천히 해도 괜찮아요.", "먼저 듣고, 그다음에 만들어요."],
  "phase": ["새 단계: 단어가 잠깐 보였다 숨어요.", "큰 단계 — 이제 귀로만!",
            "진짜 키보드예요 — 본 게임처럼."],
  "reveal": ["여기예요 — 이게 함정이에요.", "괜찮아요 — 이제 비밀을 알았어요."],
  "ceremony": ["스무 단어! 첫 스무 단어예요.", "어려운 단어에 도전할까요? 다섯 개 더!"],
 },
 "ja": {
  "wordDone": ["じぶんで作れたね！", "そのとおり！", "ほらね、できるよ。",
               "もうひとつクリア — ゆっくりでいいよ。", "聞いて、書けたね！"],
  "wordSetup": ["つぎの言葉だよ — よく聞いてね。", "この言葉にはいじわるな字があるよ。準備はいい？",
                "ゆっくりでいいよ。", "まず聞いて、それから作ろう。"],
  "phase": ["新しいステップ：言葉がちょっと見えて、隠れるよ。", "大きな一歩 — 耳だけで！",
            "本物のキーボードだよ — 本番と同じ。"],
  "reveal": ["ここだよ — ここがひっかけ。", "だいじょうぶ — もう秘密を知ってるね。"],
  "ceremony": ["20個できた！はじめての20個だよ。", "難しいのに挑戦する？あと5個！"],
 },
 "zh": {
  "wordDone": ["你自己拼出来了！", "完全正确。", "看到了吧？你可以的。",
               "又完成一个 — 稳稳的。", "听到了，也拼对了！"],
  "wordSetup": ["下一个词来了 — 仔细听。", "这个词里有个调皮的地方,准备好了吗?",
                "慢慢来,不着急。", "先听,再拼。"],
  "phase": ["新步骤:词语先出现,再藏起来。", "大步骤 — 只用耳朵!",
            "真正的键盘来了 — 和大游戏一样。"],
  "reveal": ["就是这里 — 这就是陷阱。", "没关系 — 现在你知道它的秘密了。"],
  "ceremony": ["二十个词!你的第一批二十个。", "准备好挑战难词了吗?再来五个!"],
 },
 "fil": {
  "wordDone": ["Ikaw mismo ang bumuo!", "Tamang-tama.", "Kita mo? Kaya mo.",
               "Isa na naman — dahan-dahan lang.", "Narinig mo at naisulat mo!"],
  "wordSetup": ["Heto ang susunod — makinig mabuti.", "May pasaway na letra rito. Handa ka na?",
                "Dahan-dahan lang dito.", "Makinig muna, saka buuin."],
  "phase": ["Bagong hakbang: sisilip ang salita, tapos magtatago.", "Malaking hakbang — tenga lang!",
            "Heto ang totoong keyboard — gaya ng malaking laro."],
  "reveal": ["Ayan — iyan ang patibong.", "Ayos lang — alam mo na ang sikreto."],
  "ceremony": ["Dalawampung salita! Ang una mong dalawampu.", "Handa sa mahihirap? Lima pa!"],
 },
 "ru": {
  "wordDone": ["Ты собрал это сам!", "Именно так.", "Видишь? У тебя получается.",
               "Ещё одно готово — спокойно и уверенно.", "Услышал — и написал!"],
  "wordSetup": ["Следующее слово — слушай внимательно.", "Тут прячется хитрая буква. Готов?",
                "Не торопись с этим словом.", "Сначала послушай, потом собирай."],
  "phase": ["Новый шаг: слово покажется и спрячется.", "Большой шаг — только на слух!",
            "Вот настоящая клавиатура — как в большой игре."],
  "reveal": ["Вот она — самая хитрая часть.", "Ничего — теперь ты знаешь секрет."],
  "ceremony": ["Двадцать слов! Твои первые двадцать.", "Готов к трудным? Ещё пять!"],
 },
 "ar": {
  "wordDone": ["ركّبتها بنفسك!", "صحيح تمامًا.", "أرأيت؟ تستطيع ذلك.",
               "واحدة أخرى — بهدوء وثبات.", "سمعتها وكتبتها!"],
  "wordSetup": ["الكلمة التالية — أنصت جيدًا.", "في هذه الكلمة حرف مخادع. مستعد؟",
                "خذ وقتك مع هذه الكلمة.", "استمع أولًا ثم ركّب."],
  "phase": ["خطوة جديدة: تظهر الكلمة ثم تختبئ.", "خطوة كبيرة — بالأذن فقط!",
            "ها هي لوحة المفاتيح الحقيقية — كما في اللعبة الكبيرة."],
  "reveal": ["ها هو — هذا هو الجزء الصعب.", "لا بأس — عرفت سرّه الآن."],
  "ceremony": ["عشرون كلمة! أول عشرين كلمة لك.", "مستعد للكلمات الصعبة؟ خمس أخرى!"],
 },
 "hi": {
  "wordDone": ["तुमने खुद बनाया!", "बिलकुल सही।", "देखा? तुम कर सकते हो।",
               "एक और पूरा — धीरे-धीरे।", "सुना और लिख दिया!"],
  "wordSetup": ["अगला शब्द — ध्यान से सुनो।", "इसमें एक शरारती अक्षर है। तैयार?",
                "इसमें जल्दी मत करो।", "पहले सुनो, फिर बनाओ।"],
  "phase": ["नया कदम: शब्द झाँकेगा, फिर छिप जाएगा।", "बड़ा कदम — अब सिर्फ़ कानों से!",
            "यह रहा असली कीबोर्ड — बड़े खेल जैसा।"],
  "reveal": ["यह रहा — यही है पेच।", "कोई बात नहीं — अब राज़ पता है।"],
  "ceremony": ["बीस शब्द! तुम्हारे पहले बीस।", "मुश्किल शब्दों के लिए तैयार? पाँच और!"],
 },
 "sw": {
  "wordDone": ["Umeliunda mwenyewe!", "Sawa kabisa.", "Umeona? Unaweza.",
               "Jingine limekamilika — taratibu.", "Ulisikia na ukaandika!"],
  "wordSetup": ["Neno linalofuata — sikiliza vizuri.", "Hili lina herufi ya ujanja. Uko tayari?",
                "Chukua muda wako.", "Sikiliza kwanza, kisha unda."],
  "phase": ["Hatua mpya: neno litajitokeza kisha kujificha.", "Hatua kubwa — masikio tu!",
            "Hii ndiyo kibodi halisi — kama mchezo mkubwa."],
  "reveal": ["Hapo ndipo — hiyo ndiyo sehemu ngumu.", "Usijali — sasa unajua siri yake."],
  "ceremony": ["Maneno ishirini! Ya kwanza kabisa.", "Tayari kwa magumu? Matano zaidi!"],
 },
}


def first_swap(word, a, b):
    """Swap a→b at first occurrence, else b→a; None if neither applies."""
    if a in word:
        return word.replace(a, b, 1)
    if b in word:
        return word.replace(b, a, 1)
    return None


def units_of(lang, word):
    # Mirrors practice::units — NFC chars (jamo handled runtime-side; decoy
    # and unit checks here only need char-level fidelity for non-ko).
    return list(NFC(word))


def template_for(lang, trap_id, first_word):
    s = SILENT.get(lang, {}).get(trap_id)
    if s is not None:
        unit = first_word[-1] if s == "LAST" else s
        if lang == "ko":
            # ieung: ㅇ exists as a jamo unit of the word (runtime-expanded);
            # cheap static check: any block whose initial is ㅇ.
            if not any(0xAC00 <= ord(c) <= 0xD7A3 and (ord(c) - 0xAC00) // (21 * 28) == 11
                       for c in NFC(first_word)):
                return None
        elif unit not in NFC(first_word):
            return None
        return {"id": "TAP_SILENT_UNIT", "params": {"unit": unit}}
    hp = HEARPICK.get(lang, {}).get(trap_id)
    if hp:
        foil = first_swap(NFC(first_word), hp[0], hp[1])
        if foil and foil != NFC(first_word):
            return {"id": "HEAR_PICK", "params": {"foil": foil}}
    if lang == "ko" and trap_id == "ssang":
        for c in NFC(first_word):
            if 0xAC00 <= ord(c) <= 0xD7A3:
                ini = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ"[(ord(c) - 0xAC00) // (21 * 28)]
                if ini in SSANG:
                    return {"id": "TAP_STACK", "params": {"unit": ini, "pair": SSANG[ini]}}
    return None


def main():
    langs = list(v1.CURRICULA.keys())
    for lang in langs:
        path = os.path.join(OUT, f"{lang}.json")
        c = json.load(open(path, encoding="utf-8"))
        b = v1.bank(lang)
        matchers = {t: mm for t, mm, _ in v1.CURRICULA[lang]}
        used = {NFC(w) for w in c["words"] + c["difficult"]}

        for t in c["traps"]:
            d = DECOYS.get(lang, {}).get(t["id"], [])
            if d:
                t["decoys"] = d
            first = c["words"][t["firstWord"]]
            tpl = template_for(lang, t["id"], first)
            if tpl:
                t["template"] = tpl
            else:
                t.pop("template", None)

        beats = []
        pool = b["easy"] + b["medium"]
        for i, t in enumerate(c["traps"]):
            at = t["firstWord"]
            if at == 0:
                continue  # never a beat on word 1 (the run must start instantly)
            alt = next((NFC(w) for w in pool
                        if NFC(w) not in used and matchers[t["id"]](
                            NFC(w).lower() if NFC(w).isascii() else NFC(w))), None)
            if not alt:
                continue
            used.add(alt)
            main_word = c["words"][at]
            if lang == "en":
                e1 = EN_WORD_EMOJI.get(main_word, NEUTRAL_EMOJI[i % 4][0])
                e2 = NEUTRAL_EMOJI[i % 4][1]
            else:
                e1, e2 = NEUTRAL_EMOJI[i % 4]
            beats.append({"at": at, "alt": alt, "emoji": [e1, e2]})
        c["choiceBeats"] = beats
        c["coach"] = COACH[lang]
        c["$draft"] = ("Agent-drafted per D10 v2 (2026-07-27) — words from the language's own "
                       "bank; intros, coach lines, template prompts and emoji pending the "
                       "standing audit round.")

        total = sum(len(v) for v in c["coach"].values())
        assert total <= 40, (lang, total)
        for pool_lines in c["coach"].values():
            for line in pool_lines:
                assert len(line) <= 90, (lang, line)

        with open(path, "w", encoding="utf-8") as f:
            json.dump(c, f, ensure_ascii=False, indent=2)
        tpls = [t["template"]["id"] for t in c["traps"] if "template" in t]
        print(f"  {lang}: +{len(beats)} beats, templates {tpls or ['(intro-card fallback)']}, coach {total}")


if __name__ == "__main__":
    main()
