import UIKit
import Capacitor

/// The UIScene life cycle. Apps built with the iOS 27 SDK (Xcode 27) must adopt
/// it: without a scene manifest UIKit refuses to launch the app ("UIScene life
/// cycle is required for apps built with this SDK"), so every build from the
/// Xcode 27 toolchain crashed at launch until this existed.
///
/// One window scene. Main.storyboard still builds the window and its
/// MainViewController (Info.plist UISceneStoryboardFile), so the Capacitor
/// bridge and the plugin registration are unchanged. Deep links that UIKit
/// used to hand AppDelegate now arrive here and go through the same router.
class SceneDelegate: UIResponder, UIWindowSceneDelegate {

    var window: UIWindow?

    func scene(_ scene: UIScene, willConnectTo session: UISceneSession, options connectionOptions: UIScene.ConnectionOptions) {
        // A cold launch from a widget or a link carries its URL here rather than
        // in openURLContexts.
        for context in connectionOptions.urlContexts {
            AppDelegate.route(url: context.url, options: [:])
        }
        for activity in connectionOptions.userActivities {
            _ = ApplicationDelegateProxy.shared.application(UIApplication.shared, continue: activity, restorationHandler: { _ in })
        }
    }

    func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) {
        for context in URLContexts {
            AppDelegate.route(url: context.url, options: [:])
        }
    }

    func scene(_ scene: UIScene, continue userActivity: NSUserActivity) {
        _ = ApplicationDelegateProxy.shared.application(UIApplication.shared, continue: userActivity, restorationHandler: { _ in })
    }
}
