# Native review: privacy and telemetry strings (2026-09-18)

Strings added or rewritten for the R7 privacy fix and CC-TELEMETRY-FOUNDATION.
Drafted by Claude (not a speaker) and shipped in build 228 so the app would stop
making false promises. Every one needs a native speaker's pass.

**For reviewers:** read the English line and the note, then fix the translation
below it. Keep the meaning exact: these are privacy statements, so a softer or
stronger promise than the English is a bug. The register is friendly and informal
(tu/du/ты), matching the rest of the app. Reply with corrections inline.

Tone reference and source: `src/i18n/locales/<lang>.json`. After review, apply
the fixes there and remove the language's section from this file.

## Spanish (`es`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- es: Este idioma necesita internet para la voz. Fragmentos cortos de lo que dices van al servidor de Spell, que usa el servicio de voz de Google para convertirlos en texto. Spell nunca guarda tu audio.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- es: Usar internet

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- es: No, gracias

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- es: Vota por este idioma

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- es: Voto contado ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- es: Ayuda a mejorar SpellGame

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- es: Envía informes de fallos y de velocidad. Nunca tus palabras, respuestas ni nada sobre ti.

**`NSMicrophoneUsageDescription`** (iOS permission prompt, `ios/App/App/es.lproj/InfoPlist.strings`) — Apple reviews these; must match what the app does.
- en: Spell listens so you can say or spell a word out loud. Usually your voice is recognized on this device. For some languages, only if you agree first, short clips are sent to Spell's server and Google's speech service to be turned into text. Spell never saves them.
- es: Spell escucha para que puedas decir o deletrear una palabra en voz alta. Normalmente tu voz se reconoce en este dispositivo. En algunos idiomas, solo si aceptas primero, se envían fragmentos cortos al servidor de Spell y al servicio de voz de Google para convertirlos en texto. Spell nunca los guarda.

**`NSSpeechRecognitionUsageDescription`** (iOS permission prompt, `ios/App/App/es.lproj/InfoPlist.strings`) — Apple reviews these; must match what the app does.
- en: Spell uses speech recognition on this device to understand what you say. Your voice isn't recorded or saved.
- es: Spell usa el reconocimiento de voz de este dispositivo para entender lo que dices. Tu voz no se graba ni se guarda.

## French (`fr`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- fr: Cette langue a besoin d'internet pour la voix. De courts extraits de ce que tu dis vont au serveur de Spell, qui utilise le service vocal de Google pour les transformer en texte. Spell ne conserve jamais ton audio.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- fr: Utiliser internet

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- fr: Non merci

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- fr: Voter pour cette langue

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- fr: Vote compté ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- fr: Aider à améliorer SpellGame

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- fr: Envoie des rapports de plantage et de vitesse. Jamais vos mots, vos réponses ni rien sur vous.

## German (`de`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- de: Diese Sprache braucht Internet für die Stimme. Kurze Aufnahmen von dem, was du sagst, gehen an den Spell-Server, der sie mit dem Sprachdienst von Google in Text umwandelt. Spell speichert dein Audio nie.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- de: Internet nutzen

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- de: Nein danke

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- de: Für diese Sprache stimmen

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- de: Stimme gezählt ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- de: SpellGame verbessern helfen

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- de: Sendet Absturz- und Geschwindigkeitsberichte. Niemals deine Wörter, Antworten oder etwas über dich.

## Portuguese (Brazil) (`pt`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- pt: Este idioma precisa de internet para a voz. Trechos curtos do que você diz vão ao servidor do Spell, que usa o serviço de voz do Google para transformá-los em texto. O Spell nunca guarda seu áudio.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- pt: Usar a internet

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- pt: Não, obrigado

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- pt: Votar neste idioma

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- pt: Voto contado ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- pt: Ajude a melhorar o SpellGame

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- pt: Envia relatórios de falhas e de velocidade. Nunca as suas palavras, respostas ou nada sobre você.

## Polish (`pl`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- pl: Ten język potrzebuje internetu do głosu. Krótkie nagrania tego, co mówisz, trafiają na serwer Spell, który zamienia je na tekst za pomocą usługi mowy Google. Spell nigdy nie zapisuje twojego audio.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- pl: Użyj internetu

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- pl: Nie, dziękuję

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- pl: Głosuj na ten język

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- pl: Głos policzony ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- pl: Pomóż ulepszyć SpellGame

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- pl: Wysyła raporty o awariach i szybkości. Nigdy twoich słów, odpowiedzi ani niczego o tobie.

## Vietnamese (`vi`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- vi: Ngôn ngữ này cần internet cho giọng nói. Những đoạn âm thanh ngắn bạn nói được gửi đến máy chủ Spell, nơi dùng dịch vụ giọng nói của Google để chuyển thành chữ. Spell không bao giờ lưu âm thanh của bạn.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- vi: Dùng internet

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- vi: Không, cảm ơn

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- vi: Bình chọn ngôn ngữ này

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- vi: Đã ghi nhận bình chọn ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- vi: Giúp cải thiện SpellGame

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- vi: Gửi báo cáo lỗi và tốc độ. Không bao giờ gửi từ, câu trả lời hay bất cứ điều gì về bạn.

## Korean (`ko`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- ko: 이 언어의 음성 인식에는 인터넷이 필요해요. 말한 내용의 짧은 오디오가 Spell 서버로 전송되고, 서버는 Google 음성 서비스를 이용해 텍스트로 바꿔요. Spell은 오디오를 절대 저장하지 않아요.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- ko: 인터넷 사용

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- ko: 괜찮아요

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- ko: 이 언어에 투표하기

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- ko: 투표했어요 ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- ko: SpellGame 개선에 참여

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- ko: 오류 및 속도 보고서를 보냅니다. 단어, 답변, 개인 정보는 절대 보내지 않습니다.

## Japanese (`ja`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- ja: この言語の音声認識にはインターネットが必要です。話した声の短い音声がSpellサーバーへ送られ、Googleの音声認識サービスで文字に変換されます。Spellが音声を保存することはありません。

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- ja: インターネットを使う

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- ja: やめておく

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- ja: この言語に投票する

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- ja: 投票しました ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- ja: SpellGame の改善に協力

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- ja: クラッシュと速度のレポートを送信します。単語や回答、あなたに関する情報は送信しません。

## Filipino (`fil`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- fil: Kailangan ng internet ang boses para sa wikang ito. Ipinapadala ang maiikling clip ng sinasabi mo sa Spell server, na gumagamit ng speech service ng Google para gawin itong text. Hindi kailanman iniimbak ng Spell ang audio mo.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- fil: Gamitin ang internet

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- fil: Huwag na lang

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- fil: Iboto ang wikang ito

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- fil: Nabilang ang boto mo ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- fil: Tumulong pagbutihin ang SpellGame

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- fil: Nagpapadala ng ulat tungkol sa crash at bilis. Hindi kailanman ang iyong mga salita, sagot, o anumang tungkol sa iyo.

## Chinese (Simplified) (`zh`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- zh: 这个语言的语音需要联网。你说话的短音频会发送到 Spell 服务器，服务器用 Google 的语音服务把它转成文字。Spell 绝不保存你的音频。

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- zh: 使用网络

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- zh: 不用了

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- zh: 为这个语言投票

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- zh: 已计入投票 ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- zh: 帮助改进 SpellGame

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- zh: 发送崩溃和速度报告。绝不发送你的单词、答案或任何关于你的信息。

## Russian (`ru`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- ru: Для голоса на этом языке нужен интернет. Короткие записи того, что ты говоришь, отправляются на сервер Spell, который превращает их в текст с помощью речевого сервиса Google. Spell никогда не сохраняет твоё аудио.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- ru: Использовать интернет

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- ru: Нет, спасибо

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- ru: Голосовать за этот язык

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- ru: Голос учтён ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- ru: Помочь улучшить SpellGame

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- ru: Отправляет отчёты о сбоях и скорости. Никогда — ваши слова, ответы или что-либо о вас.

## Arabic (`ar`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- ar: تحتاج هذه اللغة إلى الإنترنت للصوت. تُرسَل مقاطع قصيرة مما تقوله إلى خادم Spell، الذي يستخدم خدمة الكلام من Google لتحويلها إلى نص. لا يحفظ Spell صوتك أبدًا.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- ar: استخدام الإنترنت

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- ar: لا، شكرًا

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- ar: صوّت لهذه اللغة

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- ar: تم احتساب صوتك ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- ar: ساعد في تحسين SpellGame

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- ar: يرسل تقارير الأعطال والسرعة. لا يرسل أبدًا كلماتك أو إجاباتك أو أي شيء عنك.

## Swahili (`sw`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- sw: Lugha hii inahitaji intaneti kwa sauti. Vipande vifupi vya unachosema huenda kwenye seva ya Spell, inayotumia huduma ya sauti ya Google kuvigeuza kuwa maandishi. Spell haihifadhi sauti yako kamwe.

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- sw: Tumia intaneti

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- sw: Hapana asante

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- sw: Ipigie kura lugha hii

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- sw: Kura yako imehesabiwa ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- sw: Saidia kuboresha SpellGame

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- sw: Hutuma ripoti za hitilafu na kasi. Kamwe si maneno yako, majibu, au chochote kukuhusu.

## Hindi (`hi`)

**`voiceSpell.netAsk`** — One-time consent card before a language uses server speech recognition. Must say clearly that clips of the player's voice go to Spell's server and to Google, and that Spell doesn't keep them. Legal accuracy matters more than brevity.
- en: This language needs the internet for voice. Short clips of what you say go to the Spell server, which uses Google's speech service to turn them into text. Spell never saves your audio.
- hi: इस भाषा की आवाज़ के लिए इंटरनेट चाहिए। तुम जो बोलते हो उसकी छोटी क्लिप Spell सर्वर पर जाती हैं, जो उन्हें टेक्स्ट में बदलने के लिए Google की स्पीच सेवा का इस्तेमाल करता है। Spell तुम्हारी आवाज़ कभी सेव नहीं करता।

**`voiceSpell.netOk`** — Button on that card: agree to use the internet for voice. (Unchanged; review only.)
- en: Use the internet
- hi: इंटरनेट इस्तेमाल करो

**`voiceSpell.netNo`** — Button on that card: decline. (Unchanged; review only.)
- en: No thanks
- hi: नहीं, धन्यवाद

**`coming.notify`** — Button on a coming-soon language. It records a vote; the app can NOT notify anyone, so it must not promise a notification. Short: fits one button.
- en: Vote for this language
- hi: इस भाषा के लिए वोट करो

**`coming.confirmed`** — The same button after tapping, disabled. Short, ends with ✓.
- en: Vote counted ✓
- hi: वोट गिन लिया गया ✓

**`settings.telemetry`** — Settings switch label. "SpellGame" is the product name — keep it in Latin letters.
- en: Help improve SpellGame
- hi: SpellGame को बेहतर बनाने में मदद करें

**`settings.telemetrySmall`** — Small line under that switch. Must not overpromise: reports never include the player's words, answers or anything about them.
- en: Sends crash and speed reports. Never your words, answers, or anything about you.
- hi: क्रैश और गति की रिपोर्ट भेजता है। आपके शब्द, उत्तर या आपके बारे में कुछ भी कभी नहीं।
