//! BD-5 CC-YEARBOOK — surfaces. Parent side lives INSIDE gdashBody
//! (parent-gated by construction, the family-voices precedent). Export is
//! additionally entitlement-gated: free tier previews real data, the
//! artifact unlocks with Complete (D2 — proxied by `progress_reports`
//! until CC-ENTITLEMENTS lands its own flag; mapping recorded in the
//! ledger). All bytes stay on device (I4); the share door is the same
//! Filesystem+Share pair the finale card uses.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::yearbook::{compile, Book, Period};
use crate::yearbook_pdf::{assemble, PageJpeg, Paper};

thread_local! {
    static PERIOD: std::cell::Cell<Period> = const { std::cell::Cell::new(Period::CalendarYear) };
}

fn t(k: &str) -> String {
    crate::i18n::t(k)
}
fn tp1(k: &str, key: &str, v: &str) -> String {
    crate::i18n::tp(k, &[(key, v)])
}

pub fn dash_section_into(html: &mut String) {
    html.push_str("<div class=\"gd-sec\" id=\"ybSec\"></div>");
}

fn today() -> u32 {
    (js_sys::Date::now() / 86_400_000.0) as u32
}

pub fn fill_dash_section(app: &crate::App) {
    if !crate::dom::exists("ybSec") {
        return;
    }
    let period = PERIOD.with(std::cell::Cell::get);
    let book = compile(&app.borrow(), period, today());
    let mut html = format!("<div class=\"gd-h\">{}</div>", t("yb.title"));

    // Period row (three honest options; Climb seasons carry no dates yet).
    html.push_str("<div class=\"gd-row\">");
    for p in [Period::CalendarYear, Period::SchoolYear, Period::AllTime] {
        html.push_str(&format!(
            "<button class=\"ghost{}\" data-yb-period=\"{}\">{}</button>",
            if p == period { " on" } else { "" },
            p.key(),
            t(p.key())
        ));
    }
    html.push_str("</div>");

    if !book.enough() {
        // Feature 4: an honest floor, never filler, never guilt.
        html.push_str(&format!("<div class=\"gd-row\">{}</div>", t("yb.notYet")));
        crate::dom::set_html("ybSec", &html);
        return;
    }

    // The numbers the book will print — the preview IS real data (D2).
    html.push_str(&format!(
        "<div class=\"gd-row\">{} — {}</div>",
        t("yb.gallery"),
        tp1("yb.words", "n", &book.gallery.len().to_string())
    ));
    html.push_str(&format!(
        "<div class=\"gd-row\">{} · {} · {}</div>",
        tp1("yb.mastered", "n", &book.mastered_in_period.to_string()),
        tp1("yb.days", "n", &book.days_played.to_string()),
        tp1("yb.streak", "n", &book.best_streak.to_string()),
    ));

    // Cover choice (the one sanctioned new persistence, with the pic list
    // drawn from completed runs only).
    if !book.gallery.is_empty() {
        html.push_str(&format!("<div class=\"gd-row\">{}: ", t("yb.cover")));
        let cover = book.cover_pic.clone();
        for g in book.gallery.iter().take(8) {
            html.push_str(&format!(
                "<button class=\"ghost{}\" data-yb-cover=\"{}\">{}</button>",
                if cover.as_deref() == Some(g.pic.as_str()) { " on" } else { "" },
                g.pic,
                crate::dom::escape_html(&g.pic)
            ));
        }
        html.push_str("</div>");
    }

    // Export row: entitlement-gated; preview is always available.
    html.push_str(&format!(
        "<div class=\"gd-row\"><button class=\"ghost\" id=\"ybPreview\">{}</button>",
        t("yb.preview")
    ));
    if crate::play_hub::live_entitlements().progress_reports {
        html.push_str(&format!(
            "<button class=\"ghost\" data-yb-export=\"a4\">{}</button>\
             <button class=\"ghost\" data-yb-export=\"letter\">{}</button>",
            t("yb.exportA4"),
            t("yb.exportLetter")
        ));
    } else {
        // S2: this chip renders inside the parent-gated dash only — never
        // on a kid surface, absent in Little Speller with the whole dash.
        html.push_str(&format!("<span class=\"gd-chip\">{}</span>", t("yb.locked")));
    }
    html.push_str("</div>");

    // D5 (signed): the one time-based prompt in the batch, opt-in.
    let ping_on = crate::storage::get_raw("spell_yb_ping_v1").as_deref() == Some("1");
    html.push_str(&format!(
        "<div class=\"gd-row\"><button class=\"ghost{}\" id=\"ybPing\">{}</button></div>",
        if ping_on { " on" } else { "" },
        t("yb.ping")
    ));

    crate::dom::set_html("ybSec", &html);
}

// ---------------- page composition ----------------

/// Compose the book's spreads as standalone SVGs (2:2.83 portrait). Text
/// comes from the audited pool only (I2); pictures re-render through the
/// FINALE export path — never screenshots.
fn pages_svg(book: &Book, lang: &str) -> Vec<String> {
    const W: u32 = 1240;
    const H: u32 = 1754;
    let head = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{W}\" height=\"{H}\" \
         viewBox=\"0 0 {W} {H}\"><rect width=\"{W}\" height=\"{H}\" fill=\"#ffffff\"/>"
    );
    let esc = crate::dom::escape_html;
    let mut pages = Vec::new();

    // Cover.
    let mut cover = head.clone();
    cover.push_str(&format!(
        "<text x=\"620\" y=\"300\" text-anchor=\"middle\" font-size=\"64\" \
         font-family=\"sans-serif\" font-weight=\"700\">{}</text>\
         <text x=\"620\" y=\"390\" text-anchor=\"middle\" font-size=\"36\" \
         font-family=\"sans-serif\">{}</text>",
        esc(&t("yb.title")),
        esc(&t(book.period.key()))
    ));
    if let Some(pic) = &book.cover_pic {
        if let Some(svg) = crate::yearbook::render_piece(pic, lang) {
            cover.push_str(&nest(&svg, 220, 480, 800, 800));
        }
    }
    cover.push_str("</svg>");
    pages.push(cover);

    // Gallery spread(s): 2x3 grid per page.
    for chunk in book.gallery.chunks(6) {
        let mut page = head.clone();
        page.push_str(&format!(
            "<text x=\"620\" y=\"110\" text-anchor=\"middle\" font-size=\"40\" \
             font-family=\"sans-serif\" font-weight=\"700\">{}</text>",
            esc(&t("yb.gallery"))
        ));
        for (i, g) in chunk.iter().enumerate() {
            let (col, row) = (i % 2, i / 2);
            let (x, y) = (90 + col as u32 * 560, 170 + row as u32 * 520);
            if let Some(svg) = crate::yearbook::render_piece(&g.pic, &g.lang) {
                page.push_str(&nest(&svg, x, y, 500, 420));
            }
            page.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"26\" \
                 font-family=\"sans-serif\">{} — {}</text>",
                x + 250,
                y + 470,
                esc(&g.pic),
                esc(&tp1("yb.words", "n", &g.words.to_string()))
            ));
        }
        page.push_str("</svg>");
        pages.push(page);
    }

    // Milestones + journey, numbers-first (I2-safe in every locale).
    let mut mj = head.clone();
    let mut y = 160u32;
    fn text_line(mj: &mut String, y: &mut u32, s: &str, size: u32) {
        mj.push_str(&format!(
            "<text x=\"120\" y=\"{}\" font-size=\"{size}\" font-family=\"sans-serif\">{}</text>",
            *y,
            crate::dom::escape_html(s)
        ));
        *y += size + 34;
    }
    let line = |mj: &mut String, y: &mut u32, s: &str, size: u32| text_line(mj, y, s, size);
    line(&mut mj, &mut y, &t("yb.milestones"), 44);
    line(&mut mj, &mut y, &tp1("yb.mastered", "n", &book.mastered_in_period.to_string()), 32);
    line(&mut mj, &mut y, &tp1("yb.redeemed", "n", &book.redemptions_in_period.to_string()), 32);
    if let Some(w) = &book.hardest_word {
        line(&mut mj, &mut y, &tp1("yb.hardest", "w", w), 32);
    }
    y += 40;
    line(&mut mj, &mut y, &t("yb.journey"), 44);
    line(&mut mj, &mut y, &tp1("yb.days", "n", &book.days_played.to_string()), 32);
    line(&mut mj, &mut y, &tp1("yb.streak", "n", &book.best_streak.to_string()), 32);
    line(&mut mj, &mut y, &tp1("yb.shields", "n", &book.shields.to_string()), 32);
    y += 40;
    line(&mut mj, &mut y, &t("yb.firstfav"), 44);
    for (_, w) in book.first_words.iter().take(3) {
        line(&mut mj, &mut y, &tp1("yb.first", "w", w), 32);
    }
    if let Some(p) = &book.favorite_pic {
        line(&mut mj, &mut y, &format!("{}: {}", t("yb.favorite"), p), 32);
    }
    // D3: wordmark on the BACK (this last page's foot), never interior.
    mj.push_str(&format!(
        "<text x=\"620\" y=\"1700\" text-anchor=\"middle\" font-size=\"22\" \
         font-family=\"sans-serif\" fill=\"#999999\">spellgame.net</text>"
    ));
    mj.push_str("</svg>");
    pages.push(mj);
    pages
}

/// Nest a piece SVG into a page at (x,y,w,h) via an <image> data URI —
/// resolution-independent and namespace-safe.
fn nest(piece_svg: &str, x: u32, y: u32, w: u32, h: u32) -> String {
    format!(
        "<image x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" \
         href=\"data:image/svg+xml;base64,{}\"/>",
        b64(piece_svg.as_bytes())
    )
}

/// Local base64 (the packs precedent — the wall owns the direction of
/// spellpic imports, and this module must not add one for 15 lines).
pub(crate) fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

// ---------------- rasterize + export ----------------

pub(crate) async fn rasterize(svg: &str, w: u32, h: u32) -> Option<PageJpeg> {
    let doc = web_sys::window()?.document()?;
    let img: web_sys::HtmlImageElement = doc.create_element("img").ok()?.dyn_into().ok()?;
    let url = format!("data:image/svg+xml;base64,{}", b64(svg.as_bytes()));
    let loaded = js_sys::Promise::new(&mut |res, rej| {
        img.set_onload(Some(res.unchecked_ref()));
        img.set_onerror(Some(rej.unchecked_ref()));
    });
    img.set_src(&url);
    JsFuture::from(loaded).await.ok()?;
    let canvas: web_sys::HtmlCanvasElement = doc.create_element("canvas").ok()?.dyn_into().ok()?;
    canvas.set_width(w);
    canvas.set_height(h);
    let ctx: web_sys::CanvasRenderingContext2d =
        canvas.get_context("2d").ok()??.dyn_into().ok()?;
    ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, w as f64, h as f64).ok()?;
    let data_url = canvas.to_data_url_with_type_and_encoder_options("image/jpeg", &0.92.into()).ok()?;
    let b64_part = data_url.split(',').nth(1)?;
    let bin = web_sys::window()?.atob(b64_part).ok()?;
    Some(PageJpeg { width: w, height: h, bytes: bin.bytes().collect() })
}

async fn export(app: crate::App, paper: Paper) {
    let period = PERIOD.with(std::cell::Cell::get);
    let (book, lang) = {
        let s = app.borrow();
        (compile(&s, period, today()), s.lang.clone())
    };
    let mut pages = Vec::new();
    for svg in pages_svg(&book, &lang) {
        if let Some(p) = rasterize(&svg, 1240, 1754).await {
            pages.push(p);
        }
    }
    if pages.is_empty() {
        return;
    }
    let pdf = assemble(&pages, paper);
    if let (Some(fs), Some(sh)) = (crate::share::cap_plugin("Filesystem"), crate::share::cap_share()) {
        crate::share::share_file(fs, sh, "share/spelling-yearbook.pdf", b64(&pdf));
    }
}

/// Preview: first two pages rendered into the dash as inline images.
async fn preview(app: crate::App) {
    let period = PERIOD.with(std::cell::Cell::get);
    let (book, lang) = {
        let s = app.borrow();
        (compile(&s, period, today()), s.lang.clone())
    };
    let mut html = String::new();
    for svg in pages_svg(&book, &lang).into_iter().take(2) {
        html.push_str(&format!(
            "<img style=\"width:48%;margin:1%\" src=\"data:image/svg+xml;base64,{}\"/>",
            b64(svg.as_bytes())
        ));
    }
    if crate::dom::exists("ybSec") {
        let cur = crate::dom::el("ybSec").inner_html();
        crate::dom::set_html("ybSec", &format!("{cur}<div class=\"gd-row\">{html}</div>"));
    }
}

pub fn wire(app: &crate::App) {
    if !crate::dom::exists("gdashBody") {
        return;
    }
    let a = app.clone();
    crate::dom::on::<web_sys::MouseEvent, _>("gdashBody", "click", move |e| {
        let Some(el) = e.target().and_then(|x| x.dyn_into::<web_sys::Element>().ok()) else { return };
        if let Some(pk) = el.get_attribute("data-yb-period") {
            let p = match pk.as_str() {
                "yb.period.school" => Period::SchoolYear,
                "yb.period.all" => Period::AllTime,
                _ => Period::CalendarYear,
            };
            PERIOD.with(|c| c.set(p));
            fill_dash_section(&a);
        } else if let Some(pic) = el.get_attribute("data-yb-cover") {
            crate::storage::set_raw("spell_yb_cover_v1", &pic);
            fill_dash_section(&a);
        } else if let Some(paper) = el.get_attribute("data-yb-export") {
            let paper = if paper == "letter" { Paper::Letter } else { Paper::A4 };
            let a2 = a.clone();
            wasm_bindgen_futures::spawn_local(export(a2, paper));
        } else if el.get_attribute("id").as_deref() == Some("ybPreview") {
            let a2 = a.clone();
            wasm_bindgen_futures::spawn_local(preview(a2));
        } else if el.get_attribute("id").as_deref() == Some("ybPing") {
            let on = crate::storage::get_raw("spell_yb_ping_v1").as_deref() != Some("1");
            crate::storage::set_raw("spell_yb_ping_v1", if on { "1" } else { "0" });
            crate::notifications::yearbook_ping(on);
            fill_dash_section(&a);
        }
    });
}
