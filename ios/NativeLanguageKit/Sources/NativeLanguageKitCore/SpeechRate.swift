import Foundation
import AVFoundation

/// Maps the game's playback-rate scale onto AVSpeech's own rate scale so the
/// native TTS path presents identical slow/normal controls to the cached-audio
/// path. The game uses ~0.9 = "normal" and 0.7 = "slow" (src/settings.rs
/// `state.rate`); 0.9 maps to AVSpeech's default rate and slower values scale
/// below it, clamped to AVSpeech's supported range.
public enum SpeechRate {
    public static let gameNormal: Float = 0.9

    public static func avRate(fromGameRate gameRate: Float) -> Float {
        let scaled = AVSpeechUtteranceDefaultSpeechRate * (gameRate / gameNormal)
        return min(max(scaled, AVSpeechUtteranceMinimumSpeechRate), AVSpeechUtteranceMaximumSpeechRate)
    }
}
