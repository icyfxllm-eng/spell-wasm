import AVFoundation

/// CC-SPELLIT-MIC-FIX F2 — the ONE owner of AVAudioSession (I-M5).
///
/// AVAudioSession is a single object per process, so word audio and the
/// microphone cannot hold different categories at once; they can only take
/// turns. Before this existed, five call sites across two files set the
/// category and activated the session with no coordination, and the
/// consequence was the bug this file was written for: `.playback` is an
/// OUTPUT-ONLY category with no input route, so a spoken word starting while
/// the mic was listening took the mic's input away. The tap then received
/// nothing for the rest of the session — diag `buf=0` on Eric's build 219 —
/// and the recognizer truthfully reported NO_SPEECH for the silence it was
/// handed.
///
/// Two rules, and they are the whole design:
///
///   1. **Mutual exclusion.** While a capture session is live, a playback
///      request is HELD, not applied (F2.4). It lands when capture ends.
///   2. **Both playback categories survive.** `.playback` ignores the ring
///      switch, `.ambient` respects it; that distinction IS CC-FEEDBACK D4,
///      which is why one owner must not mean one category.
///
/// Capture also RESTORES what was in effect before it started (F2.3). The old
/// code did restore, but only in `finish()` and only ever to `.playback`, so a
/// session that had been on `.ambient` for feedback sounds came back on
/// `.playback` — the ring switch silently stopped applying to them. Restoring
/// what was actually in effect is the difference, and a request that arrived
/// while the mic was live lands here rather than being lost.
final class AudioSessionOwner {
    static let shared = AudioSessionOwner()
    private init() {}

    private let session = AVAudioSession.sharedInstance()

    /// True from a successful `beginRecording()` until `endRecording()`.
    private(set) var capturing = false

    /// The category to return to when capture ends. `.playback` is the app's
    /// resting state (AppDelegate asserts it at launch) because word audio must
    /// be audible with the ring switch on — audio IS the game.
    private var playbackCategory: AVAudioSession.Category = .playback

    /// A playback request that arrived while capturing. Applied at endRecording.
    private var held: AVAudioSession.Category?

    /// What the session actually gave us. Reported in the diag line so a future
    /// failure is readable instead of guessed at (census C2).
    struct Route {
        let sampleRate: Double
        let inputPort: String
        let category: String
        let mode: String
        var diag: String {
            "cat=\(category) mode=\(mode) route=\(inputPort.isEmpty ? "(none)" : inputPort) sr=\(Int(sampleRate))"
        }
    }

    /// Word audio and feedback sounds. Honoured immediately when idle; held
    /// while capturing, because applying it is precisely what breaks the mic.
    func requestPlayback(_ category: AVAudioSession.Category) {
        guard category == .playback || category == .ambient else { return }
        if capturing {
            held = category
            return
        }
        playbackCategory = category
        try? session.setCategory(category, mode: .default)
        try? session.setActive(true)
    }

    /// Switch to a record-capable session for one listening session.
    ///
    /// Returns nil when the session cannot actually supply input — no usable
    /// sample rate, or no input port in the current route. F2.2 requires that
    /// check BEFORE a tap is installed, so a caller that gets nil must go to
    /// the mic's error state rather than install a tap that can only ever
    /// receive silence. That silent tap was the shipped behaviour.
    func beginRecording() -> Route? {
        // Deactivate first: switching category on an already-active session
        // does not reliably re-route the microphone.
        try? session.setActive(false, options: .notifyOthersOnDeactivation)
        do {
            try session.setCategory(.playAndRecord, mode: .measurement,
                                    options: [.duckOthers, .defaultToSpeaker, .allowBluetooth])
            try session.setActive(true, options: .notifyOthersOnDeactivation)
        } catch {
            // Put the player back where they were rather than leaving a
            // half-configured session behind.
            try? session.setCategory(playbackCategory, mode: .default)
            try? session.setActive(true)
            return nil
        }
        let route = Route(sampleRate: session.sampleRate,
                          inputPort: session.currentRoute.inputs.first?.portName ?? "",
                          category: session.category.rawValue,
                          mode: session.mode.rawValue)
        guard route.sampleRate > 0, !route.inputPort.isEmpty else {
            try? session.setCategory(playbackCategory, mode: .default)
            try? session.setActive(true)
            return nil
        }
        capturing = true
        held = nil
        return route
    }

    /// Restore the category that was in effect before the mic started, or the
    /// one requested while it was listening. Safe to call when not capturing.
    func endRecording() {
        guard capturing || held != nil else { return }
        capturing = false
        let next = held ?? playbackCategory
        held = nil
        playbackCategory = next
        try? session.setActive(false, options: .notifyOthersOnDeactivation)
        try? session.setCategory(next, mode: .default)
        try? session.setActive(true)
    }
}
