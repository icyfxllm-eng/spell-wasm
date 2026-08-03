import UIKit
import AVFoundation
import Capacitor

@UIApplicationMain
class AppDelegate: UIResponder, UIApplicationDelegate {

    var window: UIWindow?

    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        // SPELL_AUDIO_SESSION: play word audio even with the ring/silent switch on (audio IS the game).
        do {
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
            try AVAudioSession.sharedInstance().setActive(true)
        } catch { print("AVAudioSession error: \(error)") }
        // Override point for customization after application launch.
        return true
    }

    func applicationWillResignActive(_ application: UIApplication) {
        // Sent when the application is about to move from active to inactive state. This can occur for certain types of temporary interruptions (such as an incoming phone call or SMS message) or when the user quits the application and it begins the transition to the background state.
        // Use this method to pause ongoing tasks, disable timers, and invalidate graphics rendering callbacks. Games should use this method to pause the game.
    }

    func applicationDidEnterBackground(_ application: UIApplication) {
        // Use this method to release shared resources, save user data, invalidate timers, and store enough application state information to restore your application to its current state in case it is terminated later.
        // If your application supports background execution, this method is called instead of applicationWillTerminate: when the user quits.
    }

    func applicationWillEnterForeground(_ application: UIApplication) {
        // Called as part of the transition from the background to the active state; here you can undo many of the changes made on entering the background.
    }

    func applicationDidBecomeActive(_ application: UIApplication) {
        // Restart any tasks that were paused (or not yet started) while the application was inactive. If the application was previously in the background, optionally refresh the user interface.
    }

    func applicationWillTerminate(_ application: UIApplication) {
        // Called when the application is about to terminate. Save data if appropriate. See also applicationDidEnterBackground:.
    }

    func application(_ app: UIApplication, open url: URL, options: [UIApplication.OpenURLOptionsKey: Any] = [:]) -> Bool {
        // CC-IOS-SURFACES (BD-1): widget/intent deep links. spellgame://
        // routes by setting the webview hash; the wasm hash router turns
        // "#daily" / "#practice" into the mode. Everything else falls
        // through to Capacitor's own handler.
        if url.scheme == "spellgame" {
            let route = url.host ?? ""
            DispatchQueue.main.async {
                if let bridge = (self.window?.rootViewController as? CAPBridgeViewController)?.bridge {
                    bridge.webView?.evaluateJavaScript("window.location.hash='" + route + "'; window.dispatchEvent(new HashChangeEvent('hashchange'));", completionHandler: nil)
                }
            }
            return true
        }
        // Called when the app was launched with a url. Feel free to add additional processing here,
        // but if you want the App API to support tracking app url opens, make sure to keep this call
        return ApplicationDelegateProxy.shared.application(app, open: url, options: options)
    }

    func application(_ application: UIApplication, continue userActivity: NSUserActivity, restorationHandler: @escaping ([UIUserActivityRestoring]?) -> Void) -> Bool {
        // Called when the app was launched with an activity, including Universal Links.
        // Feel free to add additional processing here, but if you want the App API to support
        // tracking app url opens, make sure to keep this call
        return ApplicationDelegateProxy.shared.application(application, continue: userActivity, restorationHandler: restorationHandler)
    }

}

/// The app's Capacitor bridge view controller. Registers plugins that live in the APP
/// target (not an SPM package), which Capacitor's package scan does NOT auto-discover —
/// without this, `NativeLanguageKit` has a JS proxy but no native handler, so every
/// native call (mic/speech, TTS, photo) silently no-ops and no permission prompt ever
/// appears. Wired as the root VC in Main.storyboard (customClass = MainViewController).
class MainViewController: CAPBridgeViewController {
    override open func capacitorDidLoad() {
        bridge?.registerPluginInstance(NativeLanguageKitPlugin())
    }
}
