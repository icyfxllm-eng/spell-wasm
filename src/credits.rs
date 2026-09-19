//! CC-HUMAN-AUDIO F7 — the Credits screen, generated rather than hand-kept.
//!
//! `tools/human-audio/bundle.py` writes `assets/human-audio/credits.json` with
//! every SHIPPED clip whose license requires attribution (CC BY, CC BY-SA),
//! and `scripts/human-audio-check.mjs` fails the build if one is missing. This
//! screen only renders that list. CC0 clips and Gig C recordings (work for
//! hire) carry no attribution and are not listed. In Spell Jr the screen sits
//! behind the parent gate: it links out to Wikimedia Commons.

use crate::dom;

const CREDITS: &str = include_str!("../assets/human-audio/credits.json");

#[derive(serde::Deserialize, Default)]
struct Credits {
    #[serde(default)]
    clips: Vec<Credit>,
}

#[derive(serde::Deserialize)]
struct Credit {
    entry: String,
    speaker: String,
    license: String,
    source: String,
}

fn credits() -> Credits {
    serde_json::from_str(CREDITS).unwrap_or_default()
}

fn render() {
    let list = dom::el("creditsList");
    list.set_inner_html("");
    let c = credits();
    let doc = dom::doc();
    for cr in &c.clips {
        let Ok(li) = doc.create_element("li") else { continue };
        let Ok(a) = doc.create_element("a") else { continue };
        // Text nodes only: speaker names come from a public wiki.
        a.set_text_content(Some(&format!("“{}”", cr.entry)));
        let _ = a.set_attribute("href", &cr.source);
        let _ = a.set_attribute("target", "_blank");
        let _ = a.set_attribute("rel", "noopener noreferrer");
        let _ = li.append_child(&a);
        let Ok(who) = doc.create_element("span") else { continue };
        who.set_text_content(Some(&format!(" — {} · {}", cr.speaker, cr.license)));
        let _ = li.append_child(&who);
        let _ = list.append_child(&li);
    }
    dom::toggle_class("creditsNone", "btn-hide", !c.clips.is_empty());
}

fn open() {
    render();
    dom::add_class("creditsScrim", "show");
}

pub fn wire() {
    dom::on_click("creditsBtn", || {
        let kid = dom::doc().body().map(|b| b.class_list().contains("kid")).unwrap_or(false);
        if kid {
            crate::parent_gate_then(Box::new(open));
        } else {
            open();
        }
    });
    dom::on_click("creditsClose", || dom::remove_class("creditsScrim", "show"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_credits_parse() {
        let _: Credits = serde_json::from_str(CREDITS).expect("credits.json must parse");
    }
}
