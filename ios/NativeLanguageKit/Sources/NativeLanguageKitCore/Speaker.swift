import Foundation
import AVFoundation

/// On-demand offline TTS via AVSpeechSynthesizer — the zero-network audio path.
/// One utterance at a time: a new `speak` cancels the previous (matching the
/// game replaying a word). `voiceId` is REQUIRED — the speaker never picks a
/// voice; the web core selects one from the capability catalog (Decision D3).
///
/// No caching layer: AVSpeech is synthesis-on-demand and effectively free.
public final class Speaker: NSObject, AVSpeechSynthesizerDelegate {
    private let synth = AVSpeechSynthesizer()
    private var onDone: ((Bool) -> Void)?

    public override init() {
        super.init()
        synth.delegate = self
    }

    /// Speaks `text` with the given voice and the game's rate. `onComplete(true)`
    /// on natural finish, `onComplete(false)` if the voice is unknown or the
    /// utterance is cancelled (by `stop()` or a subsequent `speak`).
    public func speak(text: String, voiceId: String, gameRate: Float, onComplete: @escaping (Bool) -> Void) {
        guard let voice = AVSpeechSynthesisVoice(identifier: voiceId) else {
            onComplete(false)
            return
        }
        // Cancel any in-flight utterance first; its delegate fires didCancel and
        // resolves the previous completion as cancelled.
        if synth.isSpeaking {
            synth.stopSpeaking(at: .immediate)
        }
        let utterance = AVSpeechUtterance(string: text.precomposedStringWithCanonicalMapping)
        utterance.voice = voice
        utterance.rate = SpeechRate.avRate(fromGameRate: gameRate)
        onDone = onComplete
        synth.speak(utterance)
    }

    public func stop() {
        synth.stopSpeaking(at: .immediate)
    }

    public func speechSynthesizer(_ s: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
        let cb = onDone; onDone = nil; cb?(true)
    }

    public func speechSynthesizer(_ s: AVSpeechSynthesizer, didCancel utterance: AVSpeechUtterance) {
        let cb = onDone; onDone = nil; cb?(false)
    }
}
