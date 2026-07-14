//! Audit-review build only (Feature 7). Compiled ONLY under the build.rs
//! `audit` cargo feature (i.e. `--features audit`); the module is not even
//! declared in a production build, so none of this — the banner text, its DOM,
//! its storage key — exists in a shipped bundle. Everything user-visible routes
//! through the i18n `t()` system; there is no per-language conditional here (the
//! preselect + picker pin read `consts::audit_langs`, the single build-time seam).

use crate::dom;

/// LocalStorage flag: the one-time first-launch banner has been dismissed.
const BANNER_DISMISSED: &str = "spellgame.audit.bannerDismissed";

/// Show the one-time, dismissible first-launch banner naming the audit build and
/// the preselected language. Renders once per device; a dismiss (or any prior
/// dismiss recorded in storage) keeps it gone across reloads. The banner DOM +
/// styles are injected here (not in index.html) so the shared static shell stays
/// byte-identical between production and audit builds.
pub fn show_first_launch_banner() {
    if crate::storage::get_raw(BANNER_DISMISSED).is_some() {
        return;
    }
    let doc = dom::doc();
    let Some(body) = doc.body() else { return };
    if doc.get_element_by_id("auditBanner").is_some() {
        return;
    }
    let Ok(banner) = doc.create_element("div") else { return };
    banner.set_id("auditBanner");
    let _ = banner.set_attribute(
        "style",
        "position:fixed;left:0;right:0;bottom:0;z-index:9999;\
         display:flex;align-items:center;gap:12px;\
         padding:12px 16px;\
         background:#1b1030;color:#fff;\
         border-top:2px solid #a855f7;\
         font:500 15px/1.4 system-ui,-apple-system,sans-serif;\
         box-shadow:0 -6px 24px rgba(0,0,0,.35)",
    );
    banner.set_inner_html(&format!(
        "<span style=\"flex:1\">{}</span>\
         <button id=\"auditBannerX\" aria-label=\"Dismiss\" \
         style=\"flex:none;border:0;border-radius:8px;cursor:pointer;\
         padding:6px 12px;font:600 14px system-ui;\
         background:#a855f7;color:#fff\">\u{2715}</button>",
        dom::escape_html(&crate::i18n::t("audit.banner"))
    ));
    let _ = body.append_child(&banner);

    dom::on_click("auditBannerX", || {
        crate::storage::set_raw(BANNER_DISMISSED, "1");
        let doc = dom::doc();
        if let Some(el) = doc.get_element_by_id("auditBanner") {
            el.remove();
        }
    });
}
