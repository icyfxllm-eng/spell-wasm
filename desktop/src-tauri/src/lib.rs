//! SpellGame desktop shell. This wraps the exact same web build (`../../dist`)
//! that ships to iOS (Capacitor) and the PWA — there is no forked frontend.
//! Locale detection, persistence, and switching are all handled by the web
//! layer via localStorage, which works unchanged inside Tauri. We add only:
//!  - window-state (remember size/position across launches),
//!  - os plugin (first-run locale default via the web layer's navigator.language
//!    fallback; the OS plugin is available if we ever want native detection).
//! The service worker never registers inside Tauri — index.html guards it with
//! `window.isWrappedPlatform()` (checks `__TAURI_INTERNALS__`).

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_os::init())
        .run(tauri::generate_context!())
        .expect("error while running SpellGame desktop");
}
