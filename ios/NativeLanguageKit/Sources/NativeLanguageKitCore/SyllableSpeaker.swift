import Foundation
import AVFoundation

/// Speaks a word as a sequence of syllables in one utterance and reports each
/// syllable boundary as AVSpeech reaches it (`willSpeakRangeOfSpeechString`), so
/// the web core can highlight the revealed spelling in sync with the audio —
/// timing comes from the OS, not a guess (Feature F7, native path). One at a
/// time: a new `speak`/`stop` cancels the previous. `voiceId` is REQUIRED — like
/// `Speaker`, this never picks a voice on the caller's behalf (Decision D3).
public final class SyllableSpeaker: NSObject, AVSpeechSynthesizerDelegate {
    private let synth = AVSpeechSynthesizer()
    private var plan = SyllablePlan(syllables: [])
    private var lastReported = -1
    private var onSyllable: ((Int) -> Void)?
    private var onDone: ((Bool) -> Void)?

    public override init() {
        super.init()
        synth.delegate = self
    }

    /// Speak `syllables` in order with `voiceId` at the game's rate. `onSyllable`
    /// fires with the 0-based index each time a NEW syllable begins;
    /// `onComplete(true)` on natural finish, `onComplete(false)` if the voice is
    /// unknown, the list is empty, or the utterance is cancelled/superseded.
    public func speak(syllables: [String], voiceId: String, gameRate: Float,
                      onSyllable: @escaping (Int) -> Void,
                      onComplete: @escaping (Bool) -> Void) {
        guard !syllables.isEmpty, let voice = AVSpeechSynthesisVoice(identifier: voiceId) else {
            onComplete(false)
            return
        }
        if synth.isSpeaking {
            synth.stopSpeaking(at: .immediate)
        }
        // Normalize to NFC first, then build the plan from the SAME normalized
        // tokens so the plan's UTF-16 offsets match the offsets the synthesizer
        // reports for the utterance string.
        let normalized = syllables.map { $0.precomposedStringWithCanonicalMapping }
        plan = SyllablePlan(syllables: normalized)
        lastReported = -1
        self.onSyllable = onSyllable
        onDone = onComplete
        let utterance = AVSpeechUtterance(string: plan.text)
        utterance.voice = voice
        utterance.rate = SpeechRate.avRate(fromGameRate: gameRate)
        synth.speak(utterance)
    }

    public func stop() {
        synth.stopSpeaking(at: .immediate)
    }

    public func speechSynthesizer(_ s: AVSpeechSynthesizer,
                                  willSpeakRangeOfSpeechString characterRange: NSRange,
                                  utterance: AVSpeechUtterance) {
        let idx = plan.syllableIndex(forUTF16Offset: characterRange.location)
        if idx != lastReported {
            lastReported = idx
            onSyllable?(idx)
        }
    }

    public func speechSynthesizer(_ s: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
        let cb = onDone; onDone = nil; cb?(true)
    }

    public func speechSynthesizer(_ s: AVSpeechSynthesizer, didCancel utterance: AVSpeechUtterance) {
        let cb = onDone; onDone = nil; cb?(false)
    }
}
