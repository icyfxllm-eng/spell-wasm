//! CC-WORD-PICTURE v5 screen — picker + calligram canvas.
//!
//! Rendering rules (D1/D4, Eric's curve ruling):
//!  * flow paths → SVG <textPath> for scripts that shape safely along curves;
//!    complex-shaping scripts (ar, hi) render the word STRAIGHT, rotated to
//!    the path's chord angle (joins survive; the spike showed textPath breaks
//!    them). CJK/hangul ride textPath too — no joining, glyphs place cleanly.
//!  * stacks → one <text> per NFC unit, top-down (script-universal).
//! Grow-only: placed words only ever accumulate (engine enforces).

use std::cell::{Cell, RefCell};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::{api, dom, haptics, i18n, wordpic, App};

thread_local! {
    static LANG: RefCell<String> = const { RefCell::new(String::new()) };
    static PIC: RefCell<String> = const { RefCell::new(String::new()) };
    static FEED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// F3 — the open picture's layer ladder, (rung name, word count) in
    /// climb order; empty for non-scan pictures.
    static LADDER: RefCell<Vec<(String, u32)>> = const { RefCell::new(Vec::new()) };
    static PLACED: Cell<u32> = const { Cell::new(0) };
    static OPEN: Cell<bool> = const { Cell::new(false) };
    static CARD_UP: Cell<bool> = const { Cell::new(false) };
    static HOW_STEP: Cell<u8> = const { Cell::new(0) };
    static MILESTONE: Cell<u32> = const { Cell::new(0) };
    /// L7: expert auto-center zoom (toggle 🔍; docked default ON for expert).
    static ZOOMED: Cell<bool> = const { Cell::new(true) };
    /// v7 F1 (D1): runtime legality ladder position — 0 = first solve,
    /// 1..=5 = deterministic re-solves, >5 = substitution rung applied.
    static SEED_BUMP: Cell<u32> = const { Cell::new(0) };
    /// D3: the manual camera (x, y, w viewBox) once the player has taken
    /// it by dragging or pinching; None = automatic (L7 auto-center or
    /// the full frame). Reset on every picture open.
    static PANZOOM: Cell<Option<(f64, f64, f64)>> = const { Cell::new(None) };
    /// Live pointers on the stage (pointer id, client x, y) — one is a
    /// drag, two are a pinch.
    static POINTERS: RefCell<Vec<(i32, f64, f64)>> = const { RefCell::new(Vec::new()) };
    /// Double-tap detector for the camera-goes-home gesture.
    static LAST_TAP_MS: Cell<f64> = const { Cell::new(0.0) };
}

/// D3 camera floor: the tightest legal zoom, in picture units. Tight
/// enough to read one corridor comfortably, wide enough that a player
/// can never lose the picture inside a featureless crop.
const VB_MIN: f64 = 64.0;

/// Scripts whose shaping must not ride a curved textPath (D4 ruling).
fn complex_script(lang: &str) -> bool {
    matches!(lang, "ar" | "hi")
}

pub fn wire(app: &App) {
    let a = app.clone();
    dom::on_click("wordPicOpen", move || open_picker(&a));
    let a = app.clone();
    dom::on_click("wpPickerExit", move || {
        dom::remove_class("wpPicker", "show");
        let _ = &a;
    });
    let a = app.clone();
    dom::on_click("wpExit", move || close_play(&a));
    let a = app.clone();
    dom::on_click("wpReplay", move || {
        // Hidden at expert, but the handler guards too: a gate that only
        // exists in CSS is a gate that a stale class list can open.
        if audio_gate(&current_tier()).0 {
            replay(&a);
        }
    });
    let a = app.clone();
    dom::on_click("wpRestart", move || {
        if !CARD_UP.with(Cell::get) {
            dom::add_class("wpConfirm", "show");
            CARD_UP.with(|c| c.set(true));
        }
        let _ = &a;
    });
    let a = app.clone();
    dom::on_click("wpConfirmYes", move || {
        let (pic, lang) = (PIC.with(|p| p.borrow().clone()), LANG.with(|l| l.borrow().clone()));
        let mut s = wordpic::load();
        s.restart(&pic, &lang);
        wordpic::save(&s);
        dom::remove_class("wpConfirm", "show");
        CARD_UP.with(|c| c.set(false));
        open_play(&a, &pic);
    });
    let a2 = app.clone();
    dom::on_click("wpConfirmNo", move || {
        dom::remove_class("wpConfirm", "show");
        CARD_UP.with(|c| c.set(false));
        crate::keyboard::rebuild(&a2);
    });
    let a = app.clone();
    dom::on_click("wpHowNext", move || how_next(&a));
    // Gallery: tapping a trophy opens it fullscreen, at rest.
    let a = app.clone();
    dom::on::<web_sys::Event, _>("wpGallery", "click", move |e| {
        let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        if let Some(el) = t.closest("[data-gallery]").ok().flatten() {
            if let Some(id) = el.get_attribute("data-gallery") {
                open_gallery_piece(&a, &id);
            }
        }
    });
    dom::on_click("wpGalleryBtn", || {
        dom::toggle_class("wpGallery", "btn-hide",
            !dom::el("wpGallery").class_list().contains("btn-hide"));
    });
    let a = app.clone();
    dom::on_click("wpSurprise", move || {
        let lang = a.borrow().lang.clone();
        let mut s = wordpic::load();
        let startable: Vec<String> = startable_ids(&a, &s, &lang);
        if let Some(id) = wordpic::surprise(&mut s, &startable) {
            wordpic::save(&s);
            open_play(&a, &id);
        }
    });
    let a = app.clone();
    dom::on_click("wpShare", move || {
        let lang = LANG.with(|l| l.borrow().clone());
        let pic = PIC.with(|p| p.borrow().clone());
        let n = PLACED.with(Cell::get);
        share_wordpic(&lang, &pic, n);
        let _ = &a;
    });
    // CC-FINALE: Continue is the one advancing affordance, and it is always
    // one tap away. Nothing else leaves the rest state on its own.
    let a = app.clone();
    dom::on_click("wpContinue", move || {
        dom::remove_class("wpReveal", "show");
        dom::remove_class("wpReveal", "rest");
        CARD_UP.with(|c| c.set(false));
        close_play(&a);
    });
    // "It cost seconds to make; let them watch it again."
    // Share: the card image IS the payload -- no link, no identifier, no
    // tracking (feature 4). Falls back to text only if the device has no
    // file-share path at all.
    dom::on_click("wpShare", || {
        export_and(crate::spellpic_export::Product::ShareCard, |data_url, _| {
            let lang = LANG.with(|l| l.borrow().clone());
            let pic = PIC.with(|p| p.borrow().clone());
            let n = PLACED.with(Cell::get);
            let text = share_text_for(&lang, &pic, n);
            match (crate::share::cap_plugin("Filesystem"), crate::share::cap_share()) {
                (Some(fs), Some(sh)) => match data_url.split(',').nth(1) {
                    Some(b64) => crate::share::share_image(fs, sh, text, b64.to_string()),
                    None => share_wordpic(&lang, &pic, n),
                },
                _ => share_wordpic(&lang, &pic, n),
            }
        });
    });
    // Save: the keepsake, full resolution, no wordmark (D2 -- their art,
    // not an ad).
    dom::on_click("wpSave", || {
        export_and(crate::spellpic_export::Product::Keepsake, |data_url, _| {
            if let Err(e) = crate::spellpic_export::save_to_photos(&data_url) {
                note(e.i18n_key());
            }
        });
    });
    dom::on_click("wpReplayBuild", || {
        let pic = PIC.with(|p| p.borrow().clone());
        start_build(&pic);
    });
    // A tap anywhere on the reveal skips the build. Registered on the stage
    // and the backdrop, NOT on the action row -- otherwise the first tap on
    // Continue would be eaten by the skip.
    dom::on::<web_sys::Event, _>("wpRevealStage", "click", |_| skip_build());
    dom::on::<web_sys::Event, _>("wpReveal", "click", |e| {
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            if t.closest(".wp-reveal-acts").ok().flatten().is_some() {
                return;
            }
        }
        skip_build();
    });
    let a = app.clone();
    dom::on::<web_sys::Event, _>("wpInput", "input", move |_| on_typed(&a));
    // v7 F7 / D8: record how each character arrived. Dictated chunks are
    // refused at the submit path below — a spoken whole word is never a
    // spelling answer, in any mode.
    dom::on_before_input("wpInput", |ty, len| {
        crate::input_provenance::note_insert("wpInput", &ty, len)
    });
    let a = app.clone();
    dom::on_click("wpZoom", move || {
        ZOOMED.with(|z| z.set(!z.get()));
        // D3: the toggle is also a home button — a manual camera lets go.
        PANZOOM.with(|c| c.set(None));
        rerender_open();
        let _ = &a;
    });
    // CC-PICTURE-BANK D3 — the pan/zoom camera (drag pans, pinch zooms,
    // double-tap goes home). Listeners live on #wpStage, which survives
    // re-renders: set_html replaces only its children.
    wire_camera();
}

/// Campaign length: scan-locked subjects count their planned words;
/// legacy subjects count solver slots.
fn subject_total(p: &wordpic::Picture, lang: &str) -> u32 {
    if crate::spellpic::has(&p.id) {
        let seed = wordpic::load().run(&p.id, lang).map(|r| r.seed).unwrap_or(1);
        return crate::spellpic::plan(&p.id, lang, seed)
            .map(|pl| pl.words.len() as u32)
            .unwrap_or(0);
    }
    crate::wordpic_layout::slots_for_lang(p, lang).len() as u32
}

fn startable_ids(app: &App, state: &wordpic::State, lang: &str) -> Vec<String> {
    wordpic::picker_order(state, lang)
        .into_iter()
        .filter(|p| allowed(app, p, lang))
        .map(|p| p.id.clone())
        .collect()
}

/// D16 gating: free trio at PREVIEW, everything with Complete; Kid Mode uses
/// the computed eligibility.
fn allowed(app: &App, p: &wordpic::Picture, lang: &str) -> bool {
    if !wordpic::playable(p, lang) {
        return false;
    }
    if crate::wordpic_layout::READINESS_EXCEPTIONS.contains(&(p.id.as_str(), lang)) {
        return false; // I1 readiness-listed pair — hidden, never jumbled
    }
    if crate::spellpic::has(&p.id) && subject_total(p, lang) == 0 {
        return false; // v8.2: no legal plan for this language — not offered
    }
    let kid = app.borrow().kid;
    if kid && !wordpic::kid_ok(p, lang, 1) {
        return false;
    }
    let ent = crate::play_hub::live_entitlements();
    let full = matches!(ent.lang_level(lang), crate::entitlements::AccessLevel::Full);
    full || wordpic::manifest().free.contains(&p.id)
}

pub fn open_picker(app: &App) {
    let lang = app.borrow().lang.clone();
    LANG.with(|l| *l.borrow_mut() = lang.clone());
    let state = wordpic::load();
    let mut html = String::new();
    for p in wordpic::picker_order(&state, &lang) {
        let total = subject_total(p, &lang);
        let (cls, prog) = match state.run(&p.id, &lang) {
            Some(r) if r.done => ("wp-tile done", i18n::t("wordpic.done")),
            Some(r) if !r.words.is_empty() => ("wp-tile", format!("{}/{}", r.words.len(), total)),
            _ => ("wp-tile", format!("{total}")),
        };
        let locked = !allowed(app, p, &lang);
        html.push_str(&format!(
            "<button type=\"button\" class=\"{}{}\" data-pic=\"{}\" {}><span class=\"ico\">{}</span><span class=\"prog\">{}</span></button>",
            cls,
            if locked { " locked" } else { "" },
            p.id,
            if locked { "disabled" } else { "" },
            p.icon,
            prog
        ));
    }
    dom::set_html("wpGrid", &html);
    render_gallery(&state, &lang);
    // Delegate tile taps once per open (idempotent listener via fresh nodes).
    let a = app.clone();
    dom::on::<web_sys::MouseEvent, _>("wpGrid", "click", move |e| {
        let Some(el) = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|t| t.closest("[data-pic]").ok().flatten())
        else {
            return;
        };
        if let Some(id) = el.get_attribute("data-pic") {
            open_play(&a, &id);
        }
    });
    dom::add_class("wpPicker", "show");
}

fn open_play(app: &App, pic_id: &str) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(p) = wordpic::picture(pic_id) else { return };
    let mut s = wordpic::load();
    let run = s.open(pic_id, &lang);
    let first_time = !s.how_shown;
    wordpic::save(&s);
    if PIC.with(|x| x.borrow().clone()) != pic_id {
        SEED_BUMP.with(|b| b.set(0));
    }
    PIC.with(|x| *x.borrow_mut() = pic_id.to_string());
    PLACED.with(|c| c.set(run.words.len() as u32));
    MILESTONE.with(|c| c.set(0));
    // v6: the layout engine solves words AND placements together. Placed
    // words (run.words) stay canonical; the solver's feed supplies the rest,
    // skipping anything already on the canvas.
    let recent = wordpic::load();
    let recent = recent.recent_words.get(&lang).map(|v| v.as_slice()).unwrap_or(&[]);
    let bump = SEED_BUMP.with(|b| b.get());
    // v8.2: for scan-locked subjects the FEED is the plan's word order —
    // the word you spell is the word that lands on the next baseline.
    let mut feed = if crate::spellpic::has(pic_id) {
        let pl = crate::spellpic::plan(pic_id, &lang, run.seed);
        LADDER.with(|l| {
            *l.borrow_mut() = pl.as_ref().map(|p| p.ladder.clone()).unwrap_or_default()
        });
        pl.map(|pl| pl.words).unwrap_or_default()
    } else {
        LADDER.with(|l| l.borrow_mut().clear());
        crate::wordpic_layout::layout_feed_opt(
            p, &lang, run.seed + bump.min(5) as u64, recent, bump >= 5).0
    };
    if !run.words.is_empty() {
        let mut rest: Vec<String> =
            feed.iter().filter(|w| !run.words.contains(w)).cloned().collect();
        feed = run.words.clone();
        feed.append(&mut rest);
        feed.truncate(subject_total(p, &lang) as usize);
    }
    FEED.with(|f| *f.borrow_mut() = feed);
    crate::spell_aloud::on_new_word();
    crate::spell_aloud::set_surface(crate::spell_aloud::Surface::SpellPicture);
    borrow_keyboard(true);
    reflect_voice(app);
    dom::set_text("wpPlayTitle", &format!("{} {}", p.icon, i18n::t("tools.wordpic.name")));
    crate::dom::toggle_class("wpZoom", "btn-hide", p.tier != "expert");
    ZOOMED.with(|z| z.set(true));
    // D3: every picture opens with the automatic camera, and on the
    // pannable tiers the browser must not scroll the page out from
    // under the gestures.
    PANZOOM.with(|c| c.set(None));
    POINTERS.with(|q| q.borrow_mut().clear());
    let _ = dom::el("wpStage")
        .set_attribute("style", if p.tier == "expert" { "touch-action:none" } else { "" });
    render_canvas(p, &lang, &run.words);
    reflect_count(p);
    OPEN.with(|c| c.set(true));
    // The keyboard syncs against the ACTIVE surface, so it must be told
    // after the picture is open — otherwise it locks itself on a closed
    // screen and the mode has no keys at all.
    crate::keyboard::rebuild(app);
    dom::remove_class("wpPicker", "show");
    dom::add_class("wpPlay", "show");
    // Feature 5: the Replay button HIDES at expert -- the word plays once.
    dom::toggle_class("wpReplay", "btn-hide", !audio_gate(&current_tier()).0);
    if let Ok(inp) = dom::el("wpInput").dyn_into::<web_sys::HtmlInputElement>() {
        inp.set_value("");
        let _ = inp.focus();
    }
    if first_time {
        HOW_STEP.with(|c| c.set(0));
        how_next(app); // shows card 1; advances on taps
    } else if run.words.len() < FEED.with(|f| f.borrow().len()) {
        replay(app);
    } else {
        show_done(app);
    }
}

fn how_next(app: &App) {
    // the card gated input; once it closes the shared keyboard goes live
    let _ = app;
    let step = HOW_STEP.with(Cell::get);
    if step < 3 {
        dom::set_text("wpHowText", &i18n::t(&format!("wordpic.how{}", step + 1)));
        dom::add_class("wpHow", "show");
        CARD_UP.with(|c| c.set(true));
        HOW_STEP.with(|c| c.set(step + 1));
    } else {
        dom::remove_class("wpHow", "show");
        CARD_UP.with(|c| c.set(false));
        crate::keyboard::rebuild(app); // card gated the keys; they go live now
        let mut s = wordpic::load();
        s.how_shown = true;
        wordpic::save(&s);
        replay(app);
    }
}

fn close_play(app: &App) {
    OPEN.with(|c| c.set(false));
    dom::remove_class("wpPlay", "show");
    for id in ["wpHow", "wpConfirm", "wpDone"] {
        dom::remove_class(id, "show");
    }
    CARD_UP.with(|c| c.set(false));
    api::stop();
    borrow_keyboard(false);
    crate::spell_aloud::set_surface(crate::spell_aloud::Surface::Game);
    crate::spell_aloud::on_new_word();
    crate::keyboard::rebuild(app);
    open_picker(app);
}

/// Build the SVG canvas from the SOLVER's placements (v6 L2 path-lock: the
/// renderer accepts no free positions — only slots and placements).
fn render_canvas(p: &wordpic::Picture, lang: &str, words: &[String]) {
    // v8.2 SCANLOCK: when the subject ships a pinned scan, the device
    // plans through the SAME pure crate against the SAME data as CI, so
    // the layout proven legal offline is the layout drawn here. A
    // subject whose plan is illegal displays NOTHING (F5/I10).
    if crate::spellpic::has(&p.id) {
        let seed = wordpic::load().run(&p.id, lang).map(|r| r.seed).unwrap_or(1);
        match crate::spellpic::plan(&p.id, lang, seed) {
            Some(plan) => {
                render_scanlock(&plan, lang, words);
                return;
            }
            None => {
                dom::set_html("wpStage", "");
                web_sys::console::warn_1(
                    &format!("spellpic: {} blocked — no legal plan, nothing displayed", p.id).into(),
                );
                return;
            }
        }
    }
    let slots = crate::wordpic_layout::slots_for_lang(p, lang);
    let recent = wordpic::load();
    let recent = recent.recent_words.get(lang).map(|v| v.as_slice()).unwrap_or(&[]);
    let run_seed = wordpic::load()
        .run(&p.id, lang)
        .map(|r| r.seed)
        .unwrap_or(1);
    let bump = SEED_BUMP.with(|b| b.get());
    let (_, placements, _) = crate::wordpic_layout::layout_feed_opt(
        p, lang, run_seed + bump.min(5) as u64, recent, bump >= 5);
    let complex = matches!(lang, "ar" | "hi");
    let expert = p.tier == "expert";
    let next_slot = words.len();
    // L7: expert auto-centers the active path; others show the full frame.
    // D3: a camera the player has taken outranks the L7 auto-center —
    // a re-render (new word placed) must not yank the view away.
    let manual = if expert { PANZOOM.with(Cell::get) } else { None };
    let viewbox = if let Some((mx, my, mw)) = manual {
        format!("{mx:.1} {my:.1} {mw:.1} {mw:.1}")
    } else if expert && next_slot < slots.len() {
        let focus = placements
            .iter()
            .find(|pl| pl.slot == next_slot)
            .map(|pl| pl.bounds);
        match focus {
            Some((x0, y0, x1, y1)) if ZOOMED.with(Cell::get) => {
                let cx = (x0 + x1) / 2.0;
                let cy = (y0 + y1) / 2.0;
                let w = ((x1 - x0).max(y1 - y0) * 3.2).clamp(160.0, 512.0);
                format!(
                    "{:.0} {:.0} {:.0} {:.0}",
                    (cx - w / 2.0).clamp(0.0, 512.0 - w),
                    (cy - w / 2.0).clamp(0.0, 512.0 - w),
                    w,
                    w
                )
            }
            _ => "0 0 512 512".to_string(),
        }
    } else {
        "0 0 512 512".to_string()
    };
    let mut svg = format!(
        "<svg viewBox=\"{viewbox}\" xmlns=\"http://www.w3.org/2000/svg\">"
    );
    svg.push_str("<defs>");
    for (si, sl) in slots.iter().enumerate() {
        if let Some(poly) = &sl.poly {
            let d: String = poly
                .pts
                .iter()
                .enumerate()
                .map(|(k, (x, y))| format!("{}{x:.1} {y:.1} ", if k == 0 { "M" } else { "L" }))
                .collect();
            svg.push_str(&format!("<path id=\"wps{si}\" d=\"{d}\"/>"));
        }
    }
    svg.push_str("</defs>");
    // v7.5 Option 2: the guide layer — Eric's graded ink, always visible,
    // never a word host. Same normalized space as the slots.
    for g in crate::wordpic_layout::guide_polys(p) {
        let d: String = g
            .pts
            .iter()
            .enumerate()
            .map(|(k, (x, y))| format!("{}{x:.1} {y:.1} ", if k == 0 { "M" } else { "L" }))
            .collect();
        svg.push_str(&format!("<path class=\"wp-guide\" d=\"{d}\"/>"));
    }
    for (si, sl) in slots.iter().enumerate() {
        let placed = si < words.len();
        let is_next = si == next_slot;
        if !placed {
            match (&sl.poly, &sl.stack) {
                (Some(_), _) => {
                    svg.push_str(&format!(
                        "<use href=\"#wps{si}\" class=\"wp-outline{}\"/>",
                        if is_next { " next" } else { "" }
                    ));
                }
                (_, Some((x, y, cell, height))) => {
                    let n = (height / cell).round() as i32;
                    for k in 0..n {
                        svg.push_str(&format!(
                            "<rect class=\"wp-outline{}\" x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" rx=\"5\"/>",
                            if is_next { " next" } else { "" },
                            x - cell * 0.42,
                            y + k as f32 * cell - cell * 0.72,
                            cell * 0.84,
                            cell * 0.84
                        ));
                    }
                }
                _ => {}
            }
            continue;
        }
        let w = &words[si];
        let newest = si + 1 == words.len();
        let cls = if newest { "wp-word new" } else { "wp-word" };
        let Some(pl) = placements.iter().find(|pl| pl.slot == si) else { continue };
        match (&sl.poly, &sl.stack) {
            (_, Some((x, y, _cell, _height))) => {
                for (k, ch) in w.chars().enumerate() {
                    svg.push_str(&format!(
                        "<text class=\"{cls}\" data-s=\"{si}\" x=\"{x:.0}\" y=\"{:.0}\" font-size=\"{:.0}\" text-anchor=\"middle\">{}</text>",
                        y + k as f32 * pl.size,
                        pl.size,
                        dom::escape_html(&ch.to_string())
                    ));
                }
            }
            (Some(poly), _) if complex => {
                // D4 ruling: straight word rotated to the chord angle.
                let (x0, y0) = poly.pts[0];
                let (x1, y1) = *poly.pts.last().unwrap_or(&(x0, y0));
                let (px, py) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                let ang = (y1 - y0).atan2(x1 - x0).to_degrees();
                // v7 F1: force the solved span onto the device. The solver's
                // legality proof assumed this width; without textLength the
                // real font's natural advance can spill past the slot (the
                // round-2 star escape). spacingAndGlyphs keeps joins intact
                // for complex scripts — one continuous run, uniformly scaled.
                let chord = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
                svg.push_str(&format!(
                    "<text class=\"{cls}\" data-s=\"{si}\" x=\"{px:.0}\" y=\"{py:.0}\" font-size=\"{:.0}\" text-anchor=\"middle\" textLength=\"{:.0}\" lengthAdjust=\"spacingAndGlyphs\" transform=\"rotate({ang:.1} {px:.0} {py:.0})\">{}</text>",
                    pl.size,
                    chord * pl.fill,
                    dom::escape_html(w)
                ));
            }
            (Some(poly), _) => {
                // v7 F1: textLength forces the device to the solved span —
                // the geometry CI proved legal is the geometry that renders.
                // letter-spacing alone (round 2) let real font metrics spill
                // past the slot: the star overlap escape.
                svg.push_str(&format!(
                    "<text class=\"{cls}\" data-s=\"{si}\" font-size=\"{:.0}\" text-anchor=\"middle\"><textPath href=\"#wps{si}\" startOffset=\"50%\" textLength=\"{:.0}\" lengthAdjust=\"spacingAndGlyphs\">{}</textPath></text>",
                    pl.size,
                    poly.len() * pl.fill,
                    dom::escape_html(w)
                ));
            }
            _ => {}
        }
    }
    svg.push_str("</svg>");
    dom::set_html("wpStage", &svg);
    reflect_slots_indicator(lang);
    enforce_legality(p, lang, words);
}

/// v7 F1 (D1) — the layout law enforced on DEVICE, not just in CI: measure
/// the real rendered glyph boxes; on any intersection or out-of-frame
/// glyph, deterministically re-solve with the next seed (cap 5), then fall
/// back to shorter-word substitution. An illegal frame is never displayed.
fn enforce_legality(p: &wordpic::Picture, lang: &str, words: &[String]) {
    let bump = SEED_BUMP.with(|b| b.get());
    if bump > 5 {
        return; // substitution rung already applied — best legal effort
    }
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let Some(stage) = doc.get_element_by_id("wpStage") else { return };
    let texts = stage.get_elements_by_tag_name("text");
    let mut glyphs: Vec<(u32, f32, f32, f32, f32)> = Vec::new();
    for i in 0..texts.length() {
        let Some(el) = texts.item(i) else { continue };
        let sid: u32 = el
            .get_attribute("data-s")
            .and_then(|v| v.parse().ok())
            .unwrap_or(i);
        let Ok(tc) = el.dyn_into::<web_sys::SvgTextContentElement>() else { continue };
        let n = tc.get_number_of_chars();
        for c in 0..n {
            if let Ok(r) = tc.get_extent_of_char(c as u32) {
                glyphs.push((sid, r.x(), r.y(), r.width(), r.height()));
            }
        }
    }
    if glyphs.is_empty() {
        return; // headless/test render — CI covers geometry there
    }
    let v = crate::wordpic_layout::glyph_violations(&glyphs);
    if v > 0 {
        web_sys::console::warn_1(
            &format!("wordpic: {v} glyph violation(s) on device — re-solve #{}", bump + 1).into(),
        );
        SEED_BUMP.with(|b| b.set(bump + 1));
        render_canvas(p, lang, words);
    }
}

/// v8.2 — draw a scan-locked plan: spelled words ride their pinned
/// baselines; everything not yet spelled shows as the pinned stroke, so
/// the picture starts as its own outline and fills in as you spell.
/// Micro features (eyes, pupils) are always drawn, filled.
fn render_scanlock(plan: &crate::spellpic::Plan, lang: &str, words: &[String]) {
    let svg = scanlock_svg(plan, lang, words, RenderMode::Play);
    dom::set_html("wpStage", &svg);
    reflect_slots_indicator(lang);
}

/// Where the SVG is going. The GEOMETRY is identical in both modes -- that
/// is the whole point, and it is what makes CC-FINALE Done #1 ("export
/// matches the in-play final frame exactly") true by construction rather
/// than by a diff that someone has to keep passing.
///
/// Play draws into #wpStage and leans on the page's stylesheet and fonts.
/// Export has to survive being rasterized through an <img>, which loads no
/// stylesheet and -- critically -- no webfonts, so it carries its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderMode<'a> {
    Play,
    /// Self-contained: inline styles, own background, and the font embedded
    /// as a data URI. The app's faces are ALREADY split by unicode-range --
    /// instrument-latin.woff2 is 30KB -- so the right pre-subsetted file is
    /// simply picked by language; no runtime subsetter is needed.
    Export { font_data_uri: &'a str },
}

/// The one place picture geometry is turned into SVG.
pub fn scanlock_svg(
    plan: &crate::spellpic::Plan,
    lang: &str,
    words: &[String],
    mode: RenderMode,
) -> String {
    let complex = complex_script(lang);
    let placed = words.len().min(plan.placements.len());
    let mut svg = String::from("<svg viewBox=\"0 0 512 512\" xmlns=\"http://www.w3.org/2000/svg\">");
    if let RenderMode::Export { font_data_uri } = mode {
        // Rasterizing an SVG through an <img> loads no stylesheet and no
        // webfont, so everything the picture needs travels with it. Values
        // are copied from index.html; wordpic-export-parity.mjs fails the
        // build if the two ever drift.
        svg.push_str(&format!(
            "<style>@font-face{{font-family:'SpellExport';src:url({font_data_uri});}}\
             .wp-pinned{{stroke:rgba(232,236,245,.92);stroke-width:1.8;fill:none;stroke-linecap:round;stroke-linejoin:round}}\
             .wp-feature{{fill:rgba(232,236,245,.95);stroke:rgba(232,236,245,.95);stroke-width:1.5;stroke-linejoin:round}}\
             .wp-outline{{stroke:rgba(255,255,255,.22);stroke-width:3;fill:none;stroke-linecap:round}}\
             .wp-word{{fill:#e8ecf5;font-weight:700;font-family:'SpellExport',system-ui,sans-serif}}</style>\
             <rect width=\"512\" height=\"512\" fill=\"#0e1420\"/>"
        ));
    }
    // defs: one path per placement baseline
    svg.push_str("<defs>");
    for (i, pl) in plan.placements.iter().enumerate() {
        let d: String = pl
            .baseline
            .iter()
            .enumerate()
            .map(|(k, (x, y))| format!("{}{x:.1} {y:.1} ", if k == 0 { "M" } else { "L" }))
            .collect();
        svg.push_str(&format!("<path id=\"sl{i}\" d=\"{d}\"/>"));
    }
    svg.push_str("</defs>");
    // unspelled placements draw as the pinned stroke (F5)
    for (i, pl) in plan.placements.iter().enumerate().skip(placed) {
        let d: String = pl
            .baseline
            .iter()
            .enumerate()
            .map(|(k, (x, y))| format!("{}{x:.1} {y:.1} ", if k == 0 { "M" } else { "L" }))
            .collect();
        let next = if i == placed && mode == RenderMode::Play { " next" } else { "" };
        svg.push_str(&format!("<path class=\"wp-outline{next}\" d=\"{d}\"/>"));
    }
    // F5 pinned ink: scan the words cannot host is still the picture.
    for m in &plan.pinned {
        if m.points.len() < 2 {
            continue;
        }
        let d: String = m
            .points
            .iter()
            .enumerate()
            .map(|(k, (x, y))| format!("{}{x:.1} {y:.1} ", if k == 0 { "M" } else { "L" }))
            .collect();
        svg.push_str(&format!("<path class=\"wp-pinned\" d=\"{d}\"/>"));
    }
    // micro features: always present, filled (D-C)
    for m in &plan.micro {
        if m.points.len() < 3 {
            continue;
        }
        let d: String = m
            .points
            .iter()
            .enumerate()
            .map(|(k, (x, y))| format!("{}{x:.1} {y:.1} ", if k == 0 { "M" } else { "L" }))
            .collect();
        svg.push_str(&format!("<path class=\"wp-feature\" d=\"{d}Z\"/>"));
    }
    // spelled words on their baselines, justified to the exact span
    for (i, pl) in plan.placements.iter().take(placed).enumerate() {
        let len = crate::spellpic::poly_len(&pl.baseline);
        let cls = if i + 1 == placed && mode == RenderMode::Play { "wp-word new" } else { "wp-word" };
        if complex {
            // D4 ruling: complex-shaping scripts render straight at the
            // chord angle rather than along the curve.
            let (x0, y0) = pl.baseline[0];
            let (x1, y1) = *pl.baseline.last().unwrap();
            let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            let ang = (y1 - y0).atan2(x1 - x0).to_degrees();
            svg.push_str(&format!(
                "<text class=\"{cls}\" x=\"{cx:.0}\" y=\"{cy:.0}\" font-size=\"{:.0}\" text-anchor=\"middle\" textLength=\"{len:.0}\" lengthAdjust=\"spacingAndGlyphs\" transform=\"rotate({ang:.1} {cx:.0} {cy:.0})\">{}</text>",
                pl.glyph_size,
                dom::escape_html(&pl.word)
            ));
        } else {
            svg.push_str(&format!(
                "<text class=\"{cls}\" font-size=\"{:.0}\"><textPath href=\"#sl{i}\" textLength=\"{len:.0}\" lengthAdjust=\"spacing\">{}</textPath></text>",
                pl.glyph_size,
                dom::escape_html(&pl.word)
            ));
        }
    }
    svg.push_str("</svg>");
    svg
}

// ---- v7 F7: Spell Picture uses the APP's per-language keyboard ----
// The base game has no DOM input precisely so the iOS keyboard (and its
// dictation key) cannot open. Spell Picture now borrows that same
// keyboard — Korean jamo composition, Vietnamese tones, pinyin — instead
// of a system field, which is what "per-language keyboards everywhere"
// actually requires.

fn wp_input() -> Option<web_sys::HtmlInputElement> {
    dom::el("wpInput").dyn_into::<web_sys::HtmlInputElement>().ok()
}

/// A key tap from the shared keyboard, routed here while the picture is
/// open. Korean composition is handled by the keyboard layer above us.
/// CC-FINALE feature 5 — the trophy shelf.
///
/// D5: what is stored is the piece data plus its seed; the thumbnail is
/// RE-RENDERED here, never a cached bitmap. That keeps storage in kilobytes
/// and lets a finished piece inherit every later renderer improvement --
/// and it is only exact because the renderer is deterministic, which makes
/// D5 a standing constraint on the renderer, not just a storage choice.
fn render_gallery(state: &wordpic::State, lang: &str) {
    let mut html = String::new();
    let mut n = 0;
    for p in wordpic::picker_order(state, lang) {
        let Some(run) = state.run(&p.id, lang) else { continue };
        if !run.done {
            continue;
        }
        let Some(plan) = crate::spellpic::plan(&p.id, lang, run.seed) else { continue };
        let svg = scanlock_svg(&plan, lang, &run.words, RenderMode::Play);
        html.push_str(&format!(
            "<button type=\"button\" data-gallery=\"{}\" aria-label=\"{}\">{svg}</button>",
            p.id, dom::escape_html(&p.id)
        ));
        n += 1;
    }
    dom::set_html("wpGallery", &html);
    dom::toggle_class("wpGallery", "btn-hide", n == 0);
}

/// Open a finished piece fullscreen, at rest. Same treatment as the reveal --
/// including Save and Share -- but no build animation: the player has
/// already watched this one being made.
fn open_gallery_piece(app: &App, pic: &str) {
    let lang = LANG.with(|l| l.borrow().clone());
    let _ = app;
    let st = wordpic::load();
    let Some(run) = st.run(pic, &lang).cloned() else { return };
    PIC.with(|p| *p.borrow_mut() = pic.to_string());
    PLACED.with(|c| c.set(run.words.len() as u32));
    match crate::spellpic::plan(pic, &lang, run.seed) {
        Some(plan) => dom::set_html(
            "wpRevealStage",
            &scanlock_svg(&plan, &lang, &run.words, RenderMode::Play),
        ),
        None => return,
    }
    dom::set_text(
        "wpRevealNote",
        &i18n::tp("wordpic.doneBody", &[("n", &run.words.len().to_string())]),
    );
    // The reveal is markup INSIDE #wpPlay. Opened from the picker, its
    // parent screen is display:none and the reveal would "show" invisibly --
    // the gallery spec caught exactly that. The reveal covers the screen
    // (inset:0), so the play HUD beneath never paints.
    dom::add_class("wpPlay", "show");
    dom::add_class("wpReveal", "show");
    dom::add_class("wpReveal", "rest");
    CARD_UP.with(|c| c.set(true));
}

/// CC-FINALE features 3+4. Export the finished piece and hand it to the
/// share sheet (ShareCard) or save it (Keepsake).
///
/// Both go through the ONE export renderer, so what leaves the device is
/// re-rendered from the piece's data at export resolution -- never the
/// screen. Every failure surfaces an audited string rather than a
/// degraded image: an export that silently fell back to the wrong font
/// would not be noticed until it was already in someone's camera roll.
fn export_and(product: crate::spellpic_export::Product, then: fn(String, String)) {
    use crate::spellpic_export as ex;
    let lang = LANG.with(|l| l.borrow().clone());
    let pic = PIC.with(|p| p.borrow().clone());
    let st = wordpic::load();
    let words = st.run(&pic, &lang).map(|r| r.words.clone()).unwrap_or_default();
    let seed = st.run(&pic, &lang).map(|r| r.seed).unwrap_or(1);
    let Some(plan) = crate::spellpic::plan(&pic, &lang, seed) else {
        note(ex::ExportError::NoPlan.i18n_key());
        return;
    };
    let (icon, tier) = wordpic::picture(&pic)
        .map(|p| (p.icon.clone(), p.tier.clone()))
        .unwrap_or_default();
    let endonym = crate::consts::BUILTIN_LANGS
        .iter()
        .find(|(c, _, _, _)| *c == lang)
        .map(|(_, n, _, _)| n.to_string())
        .unwrap_or_default();
    let attribution = crate::spellpic::attribution(&pic).to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let meta = ex::CardMeta {
            icon: &icon,
            lang_endonym: &endonym,
            attribution: &attribution,
            tier_dots: match tier.as_str() {
                "easy" => 1,
                "medium" => 2,
                "hard" => 3,
                _ => 4,
            },
        };
        // The keepsake carries NO meta by design (D2: their art, not an ad).
        let m = if product == ex::Product::ShareCard { Some(meta) } else { None };
        match ex::export_png(&plan, &lang, &words, product, m).await {
            Ok(data_url) => then(data_url, String::new()),
            Err(e) => note(e.i18n_key()),
        }
    });
}

/// Say what happened, in the player's language, through the audit gate.
fn note(key: &str) {
    dom::set_text("wpRevealNote", &i18n::t(key));
}

/// The picture's own share text. It lived in share.rs, which made a shared
/// module import the picture subtree -- the exact direction I3 forbids. The
/// generic share_text() stays shared; knowing what a picture is does not.
fn share_text_for(lang: &str, pic: &str, n: u32) -> String {
    let name = crate::consts::BUILTIN_LANGS
        .iter()
        .find(|(c, _, _, _)| *c == lang)
        .map(|(_, n, _, _)| *n)
        .unwrap_or(lang);
    let icon = wordpic::picture(pic).map(|p| p.icon.as_str()).unwrap_or("\u{1f5bc}\u{fe0f}");
    i18n::tp("wordpic.shareText", &[("pic", icon), ("lang", name), ("n", &n.to_string())])
}

pub fn share_wordpic(lang: &str, pic: &str, n: u32) {
    let name = crate::consts::BUILTIN_LANGS
        .iter()
        .find(|(c, _, _, _)| *c == lang)
        .map(|(_, n, _, _)| *n)
        .unwrap_or(lang);
    let icon = wordpic::picture(pic).map(|p| p.icon.as_str()).unwrap_or("\u{1f5bc}\u{fe0f}");
    let text = i18n::tp("wordpic.shareText", &[("pic", icon), ("lang", name), ("n", &n.to_string())]);
    crate::share::share_text(&text);
}

/// I3: register with shared code rather than being reached into. Called
/// once from wire(); with the picture compiled out this never runs and every
/// hook stays None, so the base game takes the path it always took.
pub fn install_surface_hooks() {
    crate::surface_hooks::install(crate::surface_hooks::Hooks {
        type_char: Some(kb_type),
        type_jamo: Some(kb_jamo),
        backspace: Some(kb_backspace),
        voice_state: Some(voice_state),
        voice_set: Some(voice_set),
    });
}

pub fn kb_type(ch: char) {
    let Some(inp) = wp_input() else { return };
    let mut v = inp.value();
    v.push(ch);
    crate::input_provenance::note_insert("wpInput", "insertText", 1);
    inp.set_value(&v);
    dom::el("wpInput").dispatch_event(&web_sys::Event::new("input").unwrap()).ok();
}

/// Korean: the same Hangul composition automaton the base game uses,
/// applied to the picture's buffer (ko types jamo, renders blocks).
pub fn kb_jamo(jamo: char) {
    let Some(inp) = wp_input() else { return };
    let composed = crate::hangul::feed(&inp.value(), jamo);
    crate::input_provenance::note_insert("wpInput", "insertText", 1);
    inp.set_value(&composed);
    dom::el("wpInput").dispatch_event(&web_sys::Event::new("input").unwrap()).ok();
}

pub fn kb_backspace() {
    let Some(inp) = wp_input() else { return };
    let mut v = inp.value();
    v.pop();
    inp.set_value(&v);
    dom::el("wpInput").dispatch_event(&web_sys::Event::new("input").unwrap()).ok();
}

/// Lend the shared keyboard to the picture screen (and give it back).
fn borrow_keyboard(take: bool) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let (Some(kb), Some(home)) = (
        doc.get_element_by_id("gameKeyboard"),
        doc.get_element_by_id("kbHome"),
    ) else {
        return;
    };
    if take {
        if let Some(row) = doc.get_element_by_id("wpKbSlot") {
            let _ = row.append_child(&kb);
            let _ = kb.class_list().remove_1("locked");
        }
    } else {
        let _ = home.append_child(&kb);
        let _ = kb.class_list().add_1("locked");
    }
}

// ---- v7 F7 / D9: the Spell It mic, serving Spell Picture ----

/// State the CC-SPELL-ALOUD capture component reads: (target word,
/// current buffer, input live). Same component as the base game — this
/// is only the surface adapter, not a second implementation.
pub fn voice_state() -> (String, String, bool) {
    let target = current_word().unwrap_or_default();
    let buf = dom::el("wpInput")
        .dyn_into::<web_sys::HtmlInputElement>()
        .map(|i| i.value())
        .unwrap_or_default();
    let live = OPEN.with(Cell::get) && !CARD_UP.with(Cell::get) && !target.is_empty();
    (target, buf, live)
}

/// Letters the component assembled, written back to the field. Spoken
/// letters are keystroke-equivalent provenance (D8 rejects spoken WHOLE
/// WORDS, which the component itself refuses first — D3-strict).
pub fn voice_set(text: &str) {
    let Ok(inp) = dom::el("wpInput").dyn_into::<web_sys::HtmlInputElement>() else { return };
    crate::input_provenance::reset("wpInput");
    for _ in 0..text.chars().count() {
        crate::input_provenance::note_insert("wpInput", "insertText", 1);
    }
    inp.set_value(text);
    let lang = LANG.with(|l| l.borrow().clone());
    reflect_slots_indicator(&lang);
}

/// D9: the mic appears in Spell Picture only where `voiceSpell` is on
/// (en/es v1) and the component itself is available — never a dead
/// button. Moves the shared mic control into the play row.
fn reflect_voice(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    if !crate::consts::voice_spell(&lang) {
        dom::add_class("voiceSpellMic", "btn-hide");
        return;
    }
    crate::spell_aloud::set_surface(crate::spell_aloud::Surface::SpellPicture);
    crate::spell_aloud::reflect(app);
}

/// L6 — the DOCKED letter-slot indicator: one fixed home above the input bar
/// (chosen once; it cannot wander). One dash per unit of the current word.
fn reflect_slots_indicator(lang: &str) {
    let Some(w) = current_word() else {
        dom::set_html("wpSlots", "");
        return;
    };
    let typed = dom::el("wpInput")
        .dyn_into::<web_sys::HtmlInputElement>()
        .map(|i| i.value())
        .unwrap_or_default();
    let (ok, _) = crate::practice::check_prefix(lang, &w, &typed);
    let units = crate::practice::units(lang, &w).len();
    let mut html = String::new();
    for k in 0..units {
        html.push_str(if k < ok { "<span class=\"on\">●</span>" } else { "<span>–</span>" });
    }
    dom::set_html("wpSlots", &html);
}

fn reflect_count(p: &wordpic::Picture) {
    let lang = LANG.with(|l| l.borrow().clone());
    let total = subject_total(p, &lang);
    dom::set_text("wpCount", &format!("{}/{}", PLACED.with(Cell::get), total));
}

fn current_word() -> Option<String> {
    FEED.with(|f| f.borrow().get(PLACED.with(Cell::get) as usize).cloned())
}

/// Observation-only, for the E2E seam: the word the picture is waiting on.
/// Same contract as the rest of the seam -- it reads, it never types, and it
/// never bypasses the answer check.
#[cfg(feature = "testseam")]
pub fn seam_current_word() -> String {
    current_word().unwrap_or_default()
}

/// Observation-only, for the Done #6 relaunch spec: the open picture's
/// ladder and how far up it the run has climbed. Same contract as
/// `seam_current_word` — reads, never writes.
#[cfg(feature = "testseam")]
pub fn seam_ladder() -> String {
    let ladder = LADDER.with(|l| l.borrow().clone());
    let placed = PLACED.with(Cell::get);
    serde_json::json!({ "ladder": ladder, "placed": placed }).to_string()
}

// ------------------------------------------------------------- D3 camera

/// Clamp a square viewBox into the 512 frame. Pure, unit-tested.
fn vb_clamp(x: f64, y: f64, w: f64) -> (f64, f64, f64) {
    let w = w.clamp(VB_MIN, 512.0);
    (x.clamp(0.0, 512.0 - w), y.clamp(0.0, 512.0 - w), w)
}

/// Pan by a screen-pixel delta, converted through the stage's rendered
/// width so a finger-width of drag moves a finger-width of picture at
/// any zoom. Dragging content right moves the window left.
fn vb_pan(vb: (f64, f64, f64), dx_px: f64, dy_px: f64, stage_px: f64) -> (f64, f64, f64) {
    let (x, y, w) = vb;
    let scale = w / stage_px.max(1.0);
    vb_clamp(x - dx_px * scale, y - dy_px * scale, w)
}

/// Pinch about a fixed anchor: the picture point under the pinch
/// midpoint stays under it. `ratio` > 1 = fingers spreading = zoom in.
fn vb_pinch(vb: (f64, f64, f64), mid_px: (f64, f64), ratio: f64, stage_px: f64) -> (f64, f64, f64) {
    let (x, y, w) = vb;
    let s = stage_px.max(1.0);
    let (ax, ay) = (x + mid_px.0 / s * w, y + mid_px.1 / s * w);
    let nw = (w / ratio).clamp(VB_MIN, 512.0);
    vb_clamp(ax - mid_px.0 / s * nw, ay - mid_px.1 / s * nw, nw)
}

/// The D3 camera exists at expert only (masterpiece rides this same arm
/// when the tier arrives) and only while a picture is up.
fn pan_zoom_active() -> bool {
    OPEN.with(Cell::get) && matches!(current_tier().as_str(), "expert" | "masterpiece")
}

/// The camera as currently rendered: the manual viewBox if the player
/// has taken it, else parsed off the live svg — so the first drag picks
/// up seamlessly from wherever L7's auto-center happens to be looking.
fn current_vb() -> (f64, f64, f64) {
    if let Some(vb) = PANZOOM.with(Cell::get) {
        return vb;
    }
    let attr = dom::doc()
        .query_selector("#wpStage svg")
        .ok()
        .flatten()
        .and_then(|svg| svg.get_attribute("viewBox"))
        .unwrap_or_default();
    let n: Vec<f64> = attr.split_whitespace().filter_map(|t| t.parse().ok()).collect();
    if n.len() == 4 { (n[0], n[1], n[2]) } else { (0.0, 0.0, 512.0) }
}

/// Take the camera: remember it and write the attribute directly — a
/// gesture must never pay for a full SVG rebuild per pointermove.
fn set_vb(vb: (f64, f64, f64)) {
    PANZOOM.with(|c| c.set(Some(vb)));
    if let Ok(Some(svg)) = dom::doc().query_selector("#wpStage svg") {
        let _ = svg.set_attribute(
            "viewBox",
            &format!("{:.1} {:.1} {:.1} {:.1}", vb.0, vb.1, vb.2, vb.2),
        );
    }
}

/// Re-render the open picture (the wpZoom recipe, shared with the
/// double-tap camera reset).
fn rerender_open() {
    let (pic, lang) = (PIC.with(|x| x.borrow().clone()), LANG.with(|l| l.borrow().clone()));
    if let Some(p) = wordpic::picture(&pic) {
        let words = wordpic::load().run(&pic, &lang).map(|r| r.words.clone()).unwrap_or_default();
        render_canvas(p, &lang, &words);
    }
}

fn wire_camera() {
    dom::on::<web_sys::PointerEvent, _>("wpStage", "pointerdown", |e| {
        if !pan_zoom_active() {
            return;
        }
        e.prevent_default();
        let now = e.time_stamp();
        let first = POINTERS.with(|p| p.borrow().is_empty());
        if first && now - LAST_TAP_MS.with(Cell::get) < 300.0 {
            // Double-tap: the camera goes home — back to L7 auto-center
            // (or the full frame, per the wpZoom toggle).
            PANZOOM.with(|c| c.set(None));
            LAST_TAP_MS.with(|c| c.set(0.0));
            rerender_open();
            return;
        }
        LAST_TAP_MS.with(|c| c.set(now));
        POINTERS.with(|p| {
            p.borrow_mut().push((e.pointer_id(), e.client_x() as f64, e.client_y() as f64))
        });
    });
    dom::on::<web_sys::PointerEvent, _>("wpStage", "pointermove", |e| {
        if !pan_zoom_active() {
            return;
        }
        let id = e.pointer_id();
        let (cx, cy) = (e.client_x() as f64, e.client_y() as f64);
        POINTERS.with(|ps| {
            let mut ps = ps.borrow_mut();
            let n = ps.len();
            let Some(i) = ps.iter().position(|p| p.0 == id) else { return };
            if n == 1 {
                let (dx, dy) = (cx - ps[i].1, cy - ps[i].2);
                ps[i] = (id, cx, cy);
                if dx != 0.0 || dy != 0.0 {
                    let r = dom::el("wpStage").get_bounding_client_rect();
                    set_vb(vb_pan(current_vb(), dx, dy, r.width()));
                }
            } else if n >= 2 {
                let j = if i == 0 { 1 } else { 0 };
                let d0 = ((ps[i].1 - ps[j].1).powi(2) + (ps[i].2 - ps[j].2).powi(2)).sqrt();
                ps[i] = (id, cx, cy);
                let d1 = ((ps[i].1 - ps[j].1).powi(2) + (ps[i].2 - ps[j].2).powi(2)).sqrt();
                if d0 > 8.0 && d1 > 8.0 {
                    let r = dom::el("wpStage").get_bounding_client_rect();
                    let mid = (
                        (ps[i].1 + ps[j].1) / 2.0 - r.left(),
                        (ps[i].2 + ps[j].2) / 2.0 - r.top(),
                    );
                    set_vb(vb_pinch(current_vb(), mid, d1 / d0, r.width()));
                }
            }
        });
    });
    for kind in ["pointerup", "pointercancel", "pointerleave"] {
        dom::on::<web_sys::PointerEvent, _>("wpStage", kind, |e| {
            let id = e.pointer_id();
            POINTERS.with(|p| p.borrow_mut().retain(|q| q.0 != id));
        });
    }
}

/// CC-PICTURE-BANK feature 5 — audio modifiers as tier gates. Listening
/// skill climbs with spelling skill: starter/intermediate keep Replay and
/// the slow voice; advanced loses Slow; expert (and masterpiece when the
/// tier arrives) hears the word ONCE with the Replay button hidden -- hide,
/// never disable, per the file.
pub fn audio_gate(tier: &str) -> (bool, bool) {
    // (replay_allowed, slow_allowed)
    match tier {
        "easy" | "medium" => (true, true),
        "hard" => (true, false),
        _ => (false, false),
    }
}

fn current_tier() -> String {
    let pic = PIC.with(|p| p.borrow().clone());
    wordpic::picture(&pic).map(|p| p.tier.clone()).unwrap_or_default()
}

fn replay(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(w) = current_word() else { return };
    let code = format!("{}-{}", lang, lang.to_uppercase());
    let _ = app;
    let (_, slow) = audio_gate(&current_tier());
    let variant = if slow { "slow" } else { "normal" };
    let rate = if slow { 0.55 } else { 0.9 };
    api::play_word(&w.clone(), variant, 1.0, &lang, move || {
        crate::speech_out::speak(&w, rate, &code)
    });
}

fn on_typed(app: &App) {
    if CARD_UP.with(Cell::get) {
        return;
    }
    let lang = LANG.with(|l| l.borrow().clone());
    let Some(target) = current_word() else { return };
    let Ok(inp) = dom::el("wpInput").dyn_into::<web_sys::HtmlInputElement>() else { return };
    let value = inp.value();
    let (ok, complete) = crate::practice::check_prefix(&lang, &target, &value);
    if !crate::practice::prefix_viable(&lang, &target, &value) && !complete {
        // CC-PICTURE-BANK F6 — error economics. The canvas is untouched at
        // every tier (grow-only; no mechanic ever deletes a different
        // correct stroke — both miss classes land only on `.next`).
        //
        // CC-LEARNING-ENGINE: a FRESH miss — exactly one wrong unit past
        // the good prefix — records as a failed typed attempt. The guard
        // keeps an expert's standing garbage from recording once per
        // keystroke: only the transition counts, not the aftermath.
        if value.chars().count() == ok + 1 {
            crate::learner::note_attempt(&lang, &target, false, crate::learner::Channel::Typed);
        }
        let tier = current_tier();
        let (repair, dims, charges) = miss_economics(&tier);
        if dims {
            next_stroke_class("miss-dim", true);
        }
        if charges {
            next_stroke_class("miss-charged", true);
        }
        if repair {
            // The bottom tiers' repair loop: the game erases the damage.
            let good: String = target.chars().take(ok).collect();
            inp.set_value(&good);
        }
        // At expert the wrong letters STAY — erase-to-recover means the
        // player backspaces them out; the viable-again branch below lifts
        // the charge the moment they have.
        dom::add_class("wpStatus", "pulse");
        after(400, || dom::remove_class("wpStatus", "pulse"));
        haptics::key_tap();
        // A miss never replays the word at tiers where Replay is gated
        // away (feature 5: at expert the word plays once, full stop).
        if audio_gate(&tier).0 {
            replay(app);
        }
        return;
    }
    // Erase-to-recover: the prefix is viable again, so the charge lifts.
    // The advanced dim stays until the word actually lands ("until
    // corrected") — the placement redraw clears it.
    next_stroke_class("miss-charged", false);
    if complete {
        // D8 gate: a value that could not have been typed is refused, and
        // the field is cleared — the word is not placed and not scored.
        if crate::input_provenance::is_dictated("wpInput", value.chars().count() as u32) {
            inp.set_value("");
            crate::input_provenance::reset("wpInput");
            dom::add_class("wpStatus", "pulse");
            after(400, || dom::remove_class("wpStatus", "pulse"));
            replay(app);
            return;
        }
        crate::input_provenance::reset("wpInput");
        inp.set_value("");
        place(app, &target);
    } else {
        reflect_slots_indicator(&lang);
    }
}

/// CC-PICTURE-BANK feature 6 as a table — (auto_repair, miss_dims,
/// miss_charges) per tier. Pressure without punishment at the bottom,
/// stakes at the top; the wildcard treats any future tier (masterpiece)
/// as the strictest, the same safe direction the audio gate takes.
fn miss_economics(tier: &str) -> (bool, bool, bool) {
    match tier {
        "easy" | "medium" => (true, false, false),
        "hard" => (true, true, false),
        _ => (false, false, true),
    }
}

/// Toggles a miss class on the stroke being earned. Selector-pinned to
/// `.next`: the one stroke the current word is earning, never any other.
fn next_stroke_class(cls: &str, on: bool) {
    if let Ok(Some(el)) = dom::doc().query_selector("#wpStage .wp-outline.next") {
        let list = el.class_list();
        let _ = if on { list.add_1(cls) } else { list.remove_1(cls) };
    }
}

fn place(app: &App, word: &str) {
    crate::spell_aloud::on_new_word();
    let lang = LANG.with(|l| l.borrow().clone());
    let pic = PIC.with(|p| p.borrow().clone());
    let Some(p) = wordpic::picture(&pic) else { return };
    let total = subject_total(p, &lang);
    let mut s = wordpic::load();
    let placed = s.place(&pic, &lang, word, total);
    wordpic::save(&s);
    PLACED.with(|c| c.set(placed));
    // CC-LEARNING-ENGINE: a landed word is a validated typed success.
    crate::learner::note_attempt(&lang, word, true, crate::learner::Channel::Typed);
    haptics::key_tap();
    let words = s.run(&pic, &lang).map(|r| r.words.clone()).unwrap_or_default();
    render_canvas(p, &lang, &words);
    reflect_count(p);
    // D6 milestones at 25/50/75%.
    let pct = placed * 100 / total.max(1);
    let stage = MILESTONE.with(Cell::get);
    for (i, gate) in [25u32, 50, 75].iter().enumerate() {
        if pct >= *gate && stage <= i as u32 {
            MILESTONE.with(|c| c.set(i as u32 + 1));
            dom::set_text("wpStatus", &format!("{} {gate}%", i18n::t("wordpic.milestone")));
            after(1800, || dom::set_text("wpStatus", ""));
            break;
        }
    }
    // CC-PICTURE-BANK F3: crossing a rung is the moment worth naming — the
    // status line takes the rung over the percentage when both land on the
    // same word. The final rung's completion IS the picture's completion,
    // and the reveal already owns that moment, so no flash there.
    if placed < total && placed >= 1 {
        let ladder = LADDER.with(|l| l.borrow().clone());
        let before = crate::spellpic::ladder_progress(&ladder, placed - 1).0;
        let (now, current) = crate::spellpic::ladder_progress(&ladder, placed);
        if now > before {
            if let Some(name) = current {
                dom::set_text(
                    "wpStatus",
                    &i18n::tp("wordpic.layerUp", &[("name", &layer_label(name))]),
                );
                after(1800, || dom::set_text("wpStatus", ""));
            }
        }
    }
    if placed >= total {
        // CC-FINALE D6: the chime marks the moment itself — this branch and
        // only this branch. Re-entering a finished run or replaying the
        // build from the gallery shows the same reveal but stays silent.
        crate::audio_boost::chime();
        show_done(app);
    } else {
        let a = app.clone();
        after(650, move || replay(&a));
    }
}

/// CC-FINALE D1 (Eric, 2026-07-31): the build runs 3s for a starter piece
/// up to 6s for a masterpiece. Scaled to the piece, not to the word count --
/// a 60-word picture builds faster per word, but the moment is the same
/// length, because the moment is the point.
fn build_ms(tier: &str) -> u32 {
    match tier {
        "easy" => 3000,
        "medium" => 4000,
        "hard" => 5000,
        // expert, and masterpiece once CC-PICTURE-BANK adds the tier
        _ => 6000,
    }
}

/// CC-FINALE feature 1. The last correct word does not roll into the next
/// picture: the HUD goes, the piece rebuilds itself in spelling order, and
/// then it rests for as long as the player wants.
fn show_done(app: &App) {
    let lang = LANG.with(|l| l.borrow().clone());
    let pic = PIC.with(|p| p.borrow().clone());
    let n = PLACED.with(Cell::get);
    dom::set_text("wpRevealNote", &i18n::tp("wordpic.doneBody", &[("n", &n.to_string())]));
    // Same geometry the player has been watching fill in, now on its own.
    let st = wordpic::load();
    let run = st.run(&pic, &lang);
    let words = run.map(|r| r.words.clone()).unwrap_or_default();
    let seed = st.run(&pic, &lang).map(|r| r.seed).unwrap_or(1);
    // Same seed, same plan, same geometry the player watched fill in.
    match crate::spellpic::plan(&pic, &lang, seed) {
        Some(plan) => dom::set_html("wpRevealStage", &scanlock_svg(&plan, &lang, &words, RenderMode::Play)),
        // I10: nothing legal to draw means draw nothing, even here.
        None => dom::set_html("wpRevealStage", ""),
    }
    dom::add_class("wpReveal", "show");
    CARD_UP.with(|c| c.set(true));
    start_build(&pic);
    let _ = app;
}

/// Test seam (OBSERVE-only, `--features testseam` builds): the export
/// renderer's 1× piece SVG for the currently open picture, through the
/// REAL export path — plan from the run's own seed, fonts fetched with
/// fetch-and-refuse, mode Export. Done #1's pixel diff compares this
/// against the reveal frame the player is looking at.
#[cfg(feature = "testseam")]
pub async fn seam_export_svg() -> Result<String, String> {
    let lang = LANG.with(|l| l.borrow().clone());
    let pic = PIC.with(|p| p.borrow().clone());
    let st = wordpic::load();
    let run = st.run(&pic, &lang).ok_or("no run open")?;
    let plan = crate::spellpic::plan(&pic, &lang, run.seed).ok_or("no legal plan")?;
    let css = crate::spellpic_export::fetch_face_css()
        .await
        .map_err(|e| e.i18n_key().to_string())?;
    Ok(crate::spellpic_export::export_svg(&plan, &lang, &run.words, &css))
}


/// Test seam (OBSERVE-only): the REAL export bytes — a PNG data URL through
/// the same plan → svg → rasterize path Save and Share use, keepsake or
/// card. Done #4's metadata audit walks the chunks of what this returns.
#[cfg(feature = "testseam")]
pub async fn seam_export_png(product: String) -> Result<String, String> {
    use crate::spellpic_export as ex;
    let lang = LANG.with(|l| l.borrow().clone());
    let pic = PIC.with(|p| p.borrow().clone());
    let st = wordpic::load();
    let run = st.run(&pic, &lang).ok_or("no run open")?;
    let plan = crate::spellpic::plan(&pic, &lang, run.seed).ok_or("no legal plan")?;
    let (icon, tier) = wordpic::picture(&pic)
        .map(|p| (p.icon.clone(), p.tier.clone()))
        .unwrap_or_default();
    let endonym = crate::consts::BUILTIN_LANGS
        .iter()
        .find(|(c, _, _, _)| *c == lang)
        .map(|(_, n, _, _)| n.to_string())
        .unwrap_or_default();
    let attribution = crate::spellpic::attribution(&pic).to_string();
    let prod = if product == "card" { ex::Product::ShareCard } else { ex::Product::Keepsake };
    let meta = ex::CardMeta {
        icon: &icon,
        lang_endonym: &endonym,
        attribution: &attribution,
        tier_dots: match tier.as_str() {
            "easy" => 1,
            "medium" => 2,
            "hard" => 3,
            _ => 4,
        },
    };
    // The keepsake carries NO meta by design (D2: their art, not an ad).
    let m = if prod == ex::Product::ShareCard { Some(meta) } else { None };
    ex::export_png(&plan, &lang, &run.words, prod, m).await.map_err(|e| e.i18n_key().to_string())
}

/// F3 — a rung's player-facing name. The four manifest rungs are i18n
/// keys; anything unrecognized (there is nothing unrecognized today)
/// shows its raw name rather than a warning-spewing missing key.
fn layer_label(raw: &str) -> String {
    match raw {
        "outline" => i18n::t("wordpic.layerOutline"),
        "features" => i18n::t("wordpic.layerFeatures"),
        "texture" => i18n::t("wordpic.layerTexture"),
        "shading" => i18n::t("wordpic.layerShading"),
        other => other.to_string(),
    }
}

/// Stagger the words across the build window. Done in code rather than CSS
/// because the step depends on how many words this particular piece has.
fn start_build(pic: &str) {
    let tier = wordpic::picture(pic).map(|p| p.tier.clone()).unwrap_or_default();
    dom::remove_class("wpReveal", "rest");
    let Ok(list) = dom::doc().query_selector_all("#wpRevealStage .wp-word") else { return };
    let total = list.length();
    if total == 0 {
        dom::add_class("wpReveal", "rest");
        return;
    }
    let window = build_ms(&tier);
    for i in 0..total {
        let Some(node) = list.get(i) else { continue };
        // Element, NOT HtmlElement: these are SVG <text> nodes, and casting
        // them to HtmlElement quietly yields None -- which skipped every word
        // and left the build un-staggered with no error anywhere.
        let Some(el) = node.dyn_ref::<web_sys::Element>() else { continue };
        let d = window * i / total;
        let _ = el.set_attribute("style", &format!("animation-delay:{d}ms"));
    }
    // The pinned ink and micro features are the picture's own lines, not
    // words -- they lead so the words land onto something.
    if let Ok(ink) = dom::doc().query_selector_all("#wpRevealStage .wp-pinned, #wpRevealStage .wp-feature") {
        for i in 0..ink.length() {
            let Some(node) = ink.get(i) else { continue };
            if let Some(el) = node.dyn_ref::<web_sys::Element>() {
                let _ = el.set_attribute("style", "animation-delay:0ms");
            }
        }
    }
    // Settle into rest when the last word has landed, so the animation is
    // not left running under a state that claims to be still.
    after((window + 500) as i32, || dom::add_class("wpReveal", "rest"));
}

/// Skip: a tap anywhere during the build jumps straight to the rest state.
/// Never the reverse -- rest does not decay back into anything.
fn skip_build() {
    dom::add_class("wpReveal", "rest");
}

fn after(ms: i32, f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    if let Some(win) = web_sys::window() {
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
    }
}

#[cfg(test)]
mod audio_gate_tests {
    use super::audio_gate;

    /// CC-PICTURE-BANK feature 5, as a table. If a tier is ever added
    /// (masterpiece), the wildcard already treats it as the strictest gate,
    /// which is the safe direction for a tier ABOVE expert.
    #[test]
    fn the_ladder_of_listening() {
        assert_eq!(audio_gate("easy"), (true, true), "starter keeps Replay + Slow");
        assert_eq!(audio_gate("medium"), (true, true), "intermediate too");
        assert_eq!(audio_gate("hard"), (true, false), "advanced loses Slow");
        assert_eq!(audio_gate("expert"), (false, false), "expert hears it once");
        assert_eq!(audio_gate("masterpiece"), (false, false), "future tiers inherit the strictest");
    }

    /// CC-PICTURE-BANK feature 6, as a table. Exactly one economy applies
    /// per tier, the bottom auto-repairs with zero stroke cost, and only
    /// expert-and-up ever withholds the repair (erase-to-recover).
    #[test]
    fn the_ladder_of_stakes() {
        use super::miss_economics;
        assert_eq!(miss_economics("easy"), (true, false, false), "starter: unlimited free retries");
        assert_eq!(miss_economics("medium"), (true, false, false), "intermediate too");
        assert_eq!(miss_economics("hard"), (true, true, false), "advanced: repaired, but the stroke dims");
        assert_eq!(miss_economics("expert"), (false, false, true), "expert: charged, erase to recover");
        assert_eq!(miss_economics("masterpiece"), (false, false, true), "future tiers inherit the stakes");
        for t in ["easy", "medium", "hard", "expert", "masterpiece"] {
            let (repair, dims, charges) = miss_economics(t);
            assert!(!(dims && charges), "{t}: a miss is dimmed or charged, never both");
            assert!(repair != charges, "{t}: withholding the repair is what a charge means");
        }
    }
}

#[cfg(test)]
mod export_tests {
    use super::*;
    use crate::spellpic::Plan;
    use scanlock::{MicroStroke, Placement};

    fn plan() -> Plan {
        Plan {
            placements: vec![
                Placement {
                    path_idx: 0,
                    t0: 0.0,
                    t1: 0.4,
                    word: "turtle".into(),
                    glyph_size: 13.0,
                    advance_px: 7.0,
                    baseline: vec![(10.0, 20.0), (90.0, 24.0), (170.0, 40.0)],
                },
                Placement {
                    path_idx: 0,
                    t0: 0.4,
                    t1: 0.7,
                    word: "shell".into(),
                    glyph_size: 13.0,
                    advance_px: 7.0,
                    baseline: vec![(170.0, 40.0), (240.0, 80.0)],
                },
            ],
            micro: vec![MicroStroke { path_idx: 1, points: vec![(5.0, 5.0), (9.0, 5.0), (9.0, 9.0)] }],
            pinned: vec![MicroStroke { path_idx: 2, points: vec![(300.0, 300.0), (340.0, 318.0)] }],
            words: vec![],
            ladder: vec![],
            size: 13.0,
        }
    }

    /// Strip what is legitimately mode-specific -- the export's own styles
    /// and background -- and what is left must be identical. This is
    /// CC-FINALE Done #1 held as an invariant instead of a pixel diff that
    /// has to be re-run and re-approved.
    fn geometry_only(svg: &str) -> String {
        let mut out = svg.to_string();
        if let (Some(a), Some(b)) = (out.find("<style>"), out.find("</style>")) {
            out.replace_range(a..b + "</style>".len(), "");
        }
        // `new` is the 1s entry animation on the word just spelled. By the
        // time the frame is FINAL it has settled to plain `wp-word`, so the
        // settled play frame and the export agree -- which is what Done #1
        // actually claims. Normalising it here keeps that honest rather than
        // letting the test pass on a technicality.
        out.replace("<rect width=\"512\" height=\"512\" fill=\"#0e1420\"/>", "")
            .replace("wp-word new", "wp-word")
    }

    #[test]
    fn export_geometry_matches_the_finished_play_frame() {
        let p = plan();
        let words: Vec<String> = p.placements.iter().map(|x| x.word.clone()).collect();
        let play = scanlock_svg(&p, "en", &words, RenderMode::Play);
        let exp = scanlock_svg(&p, "en", &words, RenderMode::Export { font_data_uri: "data:x" });
        assert_eq!(
            geometry_only(&play),
            geometry_only(&exp),
            "export drifted from the in-play frame -- same solver output must render the same"
        );
    }

    #[test]
    fn export_carries_its_own_styles_and_font() {
        let p = plan();
        let words: Vec<String> = p.placements.iter().map(|x| x.word.clone()).collect();
        let exp = scanlock_svg(&p, "en", &words, RenderMode::Export { font_data_uri: "data:font/woff2;base64,AAAA" });
        // An <img>-rasterized SVG loads no stylesheet and no webfont.
        assert!(exp.contains("@font-face"), "no embedded face: text would fall back and letterforms shift");
        assert!(exp.contains("data:font/woff2;base64,AAAA"));
        assert!(exp.contains(".wp-word{fill:#e8ecf5"), "word fill would be transparent off-page");
        assert!(exp.contains("fill=\"#0e1420\""), "no background: PNG would export transparent");
        assert!(!scanlock_svg(&p, "en", &words, RenderMode::Play).contains("@font-face"),
                "the play frame must keep using the page's own font, not a duplicate");
    }

    /// A finished piece has no "next stroke" marker and no entry animation:
    /// those are live-play state, and baking them into a keepsake would
    /// freeze a pulsing dashed stroke into the artwork.
    #[test]
    fn export_drops_live_play_affordances() {
        let p = plan();
        let half: Vec<String> = vec!["turtle".into()];
        let play = scanlock_svg(&p, "en", &half, RenderMode::Play);
        let exp = scanlock_svg(&p, "en", &half, RenderMode::Export { font_data_uri: "d" });
        assert!(play.contains("wp-outline next"), "play should mark the next stroke");
        assert!(!exp.contains("next"), "export must not bake in the next-stroke marker");
        assert!(play.contains("wp-word new"), "play should animate the word just spelled");
        assert!(!exp.contains("wp-word new"), "export must not bake in the entry animation");
    }
}

#[cfg(test)]
mod d3_camera_tests {
    use super::*;

    #[test]
    fn clamp_holds_the_frame() {
        assert_eq!(vb_clamp(-50.0, 600.0, 1000.0), (0.0, 0.0, 512.0));
        assert_eq!(vb_clamp(400.0, 400.0, 200.0), (312.0, 312.0, 200.0));
        assert_eq!(vb_clamp(10.0, 10.0, 10.0), (10.0, 10.0, VB_MIN));
    }

    #[test]
    fn pan_scales_with_zoom_and_clamps() {
        // w=256 on a 512px stage: 100px of finger = 50 picture units,
        // and dragging content right moves the window left.
        assert_eq!(vb_pan((100.0, 100.0, 256.0), 100.0, 0.0, 512.0), (50.0, 100.0, 256.0));
        assert_eq!(vb_pan((100.0, 100.0, 256.0), -100.0, 0.0, 512.0), (150.0, 100.0, 256.0));
        // A wild fling stays inside the frame.
        assert_eq!(vb_pan((100.0, 100.0, 256.0), 9999.0, 9999.0, 512.0), (0.0, 0.0, 256.0));
    }

    #[test]
    fn pinch_keeps_the_anchor_still() {
        let (x, y, w) = vb_pinch((0.0, 0.0, 512.0), (128.0, 128.0), 2.0, 512.0);
        assert_eq!(w, 256.0);
        assert!((x + 128.0 / 512.0 * w - 128.0).abs() < 1e-9, "anchor x moved");
        assert!((y + 128.0 / 512.0 * w - 128.0).abs() < 1e-9, "anchor y moved");
    }

    #[test]
    fn pinch_respects_floor_and_ceiling() {
        let (_, _, w) = vb_pinch((0.0, 0.0, 512.0), (256.0, 256.0), 100.0, 512.0);
        assert_eq!(w, VB_MIN, "zoom floor");
        let vb = vb_pinch((200.0, 200.0, 100.0), (256.0, 256.0), 0.01, 512.0);
        assert_eq!(vb, (0.0, 0.0, 512.0), "zoom out lands on the full frame");
    }
}
