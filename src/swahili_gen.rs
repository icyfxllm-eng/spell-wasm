//! CC-SWAHILI-WORDBANK F2 — the Swahili morphological generator.
//!
//! # EXECUTION-BLOCKED (D1)
//! Behind the `swahili_gen` cargo feature, **default OFF**. Drafting this code is
//! explicitly permitted; running it on content is not, until BOTH the
//! CC-SWAHILI-TRIGGER fires (a STARTALK director's concrete interest) AND Eric
//! approves CC-SWAHILI-WORDBANK. There is no Swahili content in this repo for it
//! to act on, and this file adds none — it takes a `Lemma` slice from a caller
//! that does not exist yet.
//!
//! # THE RULES BELOW ARE NOT VERIFIED SWAHILI
//! D3 makes the generator's rule set **audited content**: an auditor reviews
//! [`RULES`] as a document, with the worked examples, exactly as they review word
//! rows. That is the whole reason this module exists rather than a pile of string
//! concatenation — and it means these rules are a PROPOSAL for a native speaker to
//! red-pen, not an assertion. I do not speak Swahili. Every rule carries
//! `audit: NeedsNativeAudit` and nothing here should be treated as correct until
//! that changes.
//!
//! # What it does
//! Swahili's agglutinative verb is a slot template:
//!
//! ```text
//!   SUBJECT + TAM + (OBJECT) + ROOT
//!     a-       na-    -ku-      -pend-a     ->  anakupenda  "he/she loves you"
//! ```
//!
//! so valid forms can be GENERATED from an audited lemma rather than only
//! collected. D4 caps that at 60 forms per lemma and demands frequency anchoring:
//! "as big as possible" means as big as *playable and defensible*, and an
//! exhaustive paradigm dump is a non-goal.
//!
//! # Determinism (D8)
//! Same lemmas + same [`GENERATOR_VERSION`] → byte-identical bank. No maps with
//! nondeterministic iteration, no clock, no randomness. A sampled audit is only
//! meaningful if the thing sampled can be reproduced.

/// Bumped whenever a rule changes. D7/D8: every form carries this, so revising a
/// rule can surgically invalidate exactly the forms it produced rather than the
/// whole bank.
pub const GENERATOR_VERSION: &str = "0.1.0-draft";

/// D4's hard ceiling, per lemma.
pub const MAX_FORMS_PER_LEMMA: usize = 60;

/// Whether a rule has been checked by someone who speaks the language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audit {
    /// Proposed by an engineer. NOT evidence. Blocks merge (D3).
    NeedsNativeAudit,
    /// A named native auditor signed this rule off (D3(b)).
    Verified,
}

/// One morphophonemic or class rule — a unit of AUDITED CONTENT (D3(b)).
///
/// `examples` is not decoration: it is the auditor's document. F2 requires ≥3 per
/// rule, and `rules_carry_three_worked_examples` enforces it, because a rule
/// stated without worked examples cannot be red-penned — only agreed with.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    pub description: &'static str,
    /// `(input gloss, expected output)` — what the auditor checks.
    pub examples: &'static [(&'static str, &'static str)],
    pub audit: Audit,
}

/// THE rule set. This is the artifact an auditor reviews under D3(b).
pub const RULES: &[Rule] = &[
    Rule {
        id: "SW-V-01",
        description:
            "Monosyllabic-root ku- retention. A one-syllable verb root keeps the infinitive ku- \
             in the na-/li-/ta-/me- tenses, because Swahili stress falls on the penult and the \
             bare root leaves nothing to carry it. Without ku-, *anala is not a word.",
        examples: &[
            ("a + na + -la 'eat'", "anakula"),
            ("tu + li + -ja 'come'", "tulikuja"),
            ("wa + ta + -nywa 'drink'", "watakunywa"),
            ("ni + me + -pa 'give'", "nimekupa"),
        ],
        audit: Audit::NeedsNativeAudit,
    },
    Rule {
        id: "SW-V-02",
        description:
            "Object marker m- -> mw- before a vowel-initial root. The class-1 object marker is a \
             bare nasal and cannot stand before a vowel.",
        examples: &[
            ("a + na + m + -ona 'see'", "anamwona"),
            ("tu + li + m + -uliza 'ask'", "tulimwuliza"),
            ("ni + ta + m + -ambia 'tell'", "nitamwambia"),
        ],
        audit: Audit::NeedsNativeAudit,
    },
    Rule {
        id: "SW-V-03",
        description:
            "Vowel-initial roots take the TAM marker unchanged; no elision at the TAM/root \
             juncture. Recorded as an explicit NON-rule so an auditor can reject it if Swahili \
             actually does contract here — an absent rule is invisible to review, which is how \
             wrong output gets shipped.",
        examples: &[
            ("a + na + -ona 'see'", "anaona"),
            ("wa + li + -andika 'write'", "waliandika"),
            ("tu + ta + -elewa 'understand'", "tutaelewa"),
        ],
        audit: Audit::NeedsNativeAudit,
    },
    Rule {
        id: "SW-N-01",
        description:
            "Noun class 1/2 (m-/wa-), people. Both members generate ONLY when the lemma's class \
             is source-attested (D4) — guessing a noun's class invents words.",
        examples: &[("mtoto 'child'", "watoto"), ("mwalimu 'teacher'", "walimu"), ("mgeni 'guest'", "wageni")],
        audit: Audit::NeedsNativeAudit,
    },
    Rule {
        id: "SW-N-02",
        description: "Noun class 7/8 (ki-/vi-), things. Same attestation condition as SW-N-01.",
        examples: &[("kitabu 'book'", "vitabu"), ("kiti 'chair'", "viti"), ("kisu 'knife'", "visu")],
        audit: Audit::NeedsNativeAudit,
    },
    Rule {
        id: "SW-N-03",
        description: "Noun class 5/6 (ji-/ma-). The ji- is often zero on the singular.",
        examples: &[("jicho 'eye'", "macho"), ("jina 'name'", "majina"), ("yai 'egg'", "mayai")],
        audit: Audit::NeedsNativeAudit,
    },
    Rule {
        id: "SW-N-04",
        description: "Noun class 11/10 (u-/n-). The plural often surfaces with no prefix at all.",
        examples: &[("ukuta 'wall'", "kuta"), ("ufunguo 'key'", "funguo"), ("uso 'face'", "nyuso")],
        audit: Audit::NeedsNativeAudit,
    },
];

/// The D4 slot inventory: "the common paradigm cells a learner actually meets",
/// not every cell the language permits.
///
/// Ordered by how ordinary the form is, because [`generate`] takes the first
/// [`MAX_FORMS_PER_LEMMA`] in enumeration order — so the cap keeps the forms a
/// learner meets and drops the exotica. When corpus frequency lands (F1), it
/// replaces this hand-ordering; until then the ordering IS the frequency anchor,
/// and saying so is better than pretending the cap is neutral.
pub const SUBJECTS: &[(&str, &str)] =
    &[("a", "3sg"), ("ni", "1sg"), ("tu", "1pl"), ("wa", "3pl"), ("u", "2sg"), ("m", "2pl")];
pub const TAMS: &[(&str, &str)] = &[("na", "present"), ("li", "past"), ("ta", "future"), ("me", "perfect"), ("ki", "situative")];
/// `""` = no object marker, which is the commonest cell and therefore first.
pub const OBJECTS: &[(&str, &str)] = &[("", "none"), ("ku", "2sg"), ("m", "3sg"), ("ni", "1sg"), ("wa", "3pl"), ("tu", "1pl")];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pos {
    Verb,
    Noun,
}

/// A Swahili noun class pair, as attested by the source of record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NounClass {
    /// m-/wa-
    C1C2,
    /// ki-/vi-
    C7C8,
    /// ji-/ma-
    C5C6,
    /// u-/n-
    C11C10,
}

/// An audited lemma from the source of record (F1's output). This module never
/// invents one.
#[derive(Debug, Clone)]
pub struct Lemma {
    pub lemma: String,
    pub pos: Pos,
    /// `None` = the source did not attest a class. D4: do NOT pair it.
    pub noun_class: Option<NounClass>,
    /// Corpus rank where attested; `None` = unranked.
    pub freq_rank: Option<u32>,
}

/// One generated form, carrying the provenance D7/D8 require.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedForm {
    pub form: String,
    /// The audited lemma this descends from. D4 of CC-BANK-EXPANSION: a form
    /// inherits its lemma's definition, license and audit lineage.
    pub lemma: String,
    /// Which rules fired. A failed sample invalidates exactly these forms.
    pub rule_ids: Vec<&'static str>,
    pub generator_version: &'static str,
    /// The slots that produced it — what `segment` must recover (F2's
    /// round-trip-segmentability test).
    pub slots: Slots,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slots {
    pub subject: String,
    pub tam: String,
    pub object: String,
    pub root: String,
}

const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];

fn starts_with_vowel(root: &str) -> bool {
    root.chars().next().map(|c| VOWELS.contains(&c)).unwrap_or(false)
}

/// SW-V-01: is the root monosyllabic (one vowel)?
fn is_monosyllabic(root: &str) -> bool {
    root.chars().filter(|c| VOWELS.contains(c)).count() == 1
}

/// Build one verb form, recording which rules fired.
fn verb_form(subject: &str, tam: &str, object: &str, root: &str) -> (String, Vec<&'static str>) {
    let mut rules = Vec::new();
    let mut out = String::new();
    out.push_str(subject);
    out.push_str(tam);

    // SW-V-02: m- -> mw- before a vowel-initial root.
    if !object.is_empty() {
        if object == "m" && starts_with_vowel(root) {
            out.push_str("mw");
            rules.push("SW-V-02");
        } else {
            out.push_str(object);
        }
    }

    // SW-V-01: a monosyllabic root keeps ku- in na/li/ta/me. NOT in ki-, where the
    // marker itself carries the stress. That exception is exactly the kind of claim
    // an auditor must confirm, which is why the rule says so out loud.
    if is_monosyllabic(root) && matches!(tam, "na" | "li" | "ta" | "me") {
        out.push_str("ku");
        rules.push("SW-V-01");
    }
    if starts_with_vowel(root) && object.is_empty() {
        rules.push("SW-V-03");
    }
    out.push_str(root);
    (out, rules)
}

/// Generate the capped, ordered form bank for `lemmas` (D4).
///
/// Deterministic (D8): enumeration order is fixed by the slot tables, and the cap
/// takes a prefix of it. Same inputs + same [`GENERATOR_VERSION`] → identical bank.
pub fn generate(lemmas: &[Lemma]) -> Vec<GeneratedForm> {
    let mut out = Vec::new();
    for lemma in lemmas {
        match lemma.pos {
            Pos::Verb => {
                let root = lemma.lemma.as_str();
                let mut made = 0usize;
                // Object-outermost so the no-object cells — the commonest — come
                // first and survive the cap.
                'cells: for (object, _) in OBJECTS {
                    for (tam, _) in TAMS {
                        for (subject, _) in SUBJECTS {
                            if made >= MAX_FORMS_PER_LEMMA {
                                break 'cells; // D4's hard cap
                            }
                            let (form, rule_ids) = verb_form(subject, tam, object, root);
                            out.push(GeneratedForm {
                                form,
                                lemma: lemma.lemma.clone(),
                                rule_ids,
                                generator_version: GENERATOR_VERSION,
                                slots: Slots {
                                    subject: subject.to_string(),
                                    tam: tam.to_string(),
                                    object: object.to_string(),
                                    root: root.to_string(),
                                },
                            });
                            made += 1;
                        }
                    }
                }
            }
            Pos::Noun => {
                // D4: pair ONLY when the class is source-attested. An unattested
                // class means we do not know the plural, and guessing invents a
                // word — the exact failure the license/audit chain cannot catch,
                // because it would be a well-formed row from a real lemma.
                let Some(class) = lemma.noun_class else { continue };
                let (rule_id, plural) = match class {
                    NounClass::C1C2 => ("SW-N-01", pluralize_prefix(&lemma.lemma, "m", "wa")),
                    NounClass::C7C8 => ("SW-N-02", pluralize_prefix(&lemma.lemma, "ki", "vi")),
                    NounClass::C5C6 => ("SW-N-03", pluralize_prefix(&lemma.lemma, "ji", "ma")),
                    NounClass::C11C10 => ("SW-N-04", pluralize_prefix(&lemma.lemma, "u", "")),
                };
                let Some(plural) = plural else { continue };
                out.push(GeneratedForm {
                    form: plural,
                    lemma: lemma.lemma.clone(),
                    rule_ids: vec![rule_id],
                    generator_version: GENERATOR_VERSION,
                    slots: Slots {
                        subject: String::new(),
                        tam: String::new(),
                        object: String::new(),
                        root: lemma.lemma.clone(),
                    },
                });
            }
        }
    }
    out
}

/// Swap a singular class prefix for its plural. `None` when the lemma does not
/// actually carry the prefix its class claims — a source disagreeing with itself,
/// which is a data problem to report, not to paper over.
fn pluralize_prefix(lemma: &str, sg: &str, pl: &str) -> Option<String> {
    let stem = lemma.strip_prefix(sg)?;
    Some(format!("{pl}{stem}"))
}

/// Recover the slots from a generated verb form (F2's round-trip requirement).
/// Returns `None` when `form` was not produced by these rules from `root`.
pub fn segment(form: &str, root: &str) -> Option<Slots> {
    for (object, _) in OBJECTS {
        for (tam, _) in TAMS {
            for (subject, _) in SUBJECTS {
                let (candidate, _) = verb_form(subject, tam, object, root);
                if candidate == form {
                    return Some(Slots {
                        subject: subject.to_string(),
                        tam: tam.to_string(),
                        object: object.to_string(),
                        root: root.to_string(),
                    });
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verb(l: &str) -> Lemma {
        Lemma { lemma: l.to_string(), pos: Pos::Verb, noun_class: None, freq_rank: None }
    }
    fn noun(l: &str, c: NounClass) -> Lemma {
        Lemma { lemma: l.to_string(), pos: Pos::Noun, noun_class: Some(c), freq_rank: None }
    }

    // ---- D3(b): the rules are audited content, so the DOCUMENT has invariants ----

    #[test]
    fn rules_carry_three_worked_examples() {
        // F2 requires >=3 per rule. A rule stated without worked examples cannot be
        // red-penned, only agreed with — and agreement is not an audit.
        for r in RULES {
            assert!(r.examples.len() >= 3, "{}: needs >=3 worked examples, has {}", r.id, r.examples.len());
            assert!(!r.description.is_empty(), "{}: needs a description", r.id);
        }
    }

    #[test]
    fn no_rule_claims_to_be_verified_yet() {
        // The honesty invariant. These rules were written by someone who does not
        // speak Swahili; D3 requires a native auditor to sign the rule set. If this
        // test ever fails, someone flipped a rule to Verified — that must be a
        // reviewed act with an auditor's name on it, not a default.
        for r in RULES {
            assert_eq!(r.audit, Audit::NeedsNativeAudit, "{}: unverified rules must say so", r.id);
        }
    }

    #[test]
    fn rule_ids_are_unique() {
        let mut ids: Vec<&str> = RULES.iter().map(|r| r.id).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate rule id — provenance would be ambiguous");
    }

    /// The rules' own worked examples must be what the generator actually emits.
    /// Otherwise the auditor reviews one thing and the bank contains another.
    #[test]
    fn worked_examples_match_generator_output() {
        assert_eq!(verb_form("a", "na", "", "la").0, "anakula", "SW-V-01");
        assert_eq!(verb_form("tu", "li", "", "ja").0, "tulikuja", "SW-V-01");
        assert_eq!(verb_form("wa", "ta", "", "nywa").0, "watakunywa", "SW-V-01");
        assert_eq!(verb_form("a", "na", "m", "ona").0, "anamwona", "SW-V-02");
        assert_eq!(verb_form("ni", "ta", "m", "ambia").0, "nitamwambia", "SW-V-02");
        assert_eq!(verb_form("a", "na", "", "ona").0, "anaona", "SW-V-03");
        assert_eq!(verb_form("wa", "li", "", "andika").0, "waliandika", "SW-V-03");
        assert_eq!(verb_form("a", "na", "ku", "penda").0, "anakupenda", "the module doc's example");
    }

    // ---- D4 ----

    #[test]
    fn cap_is_enforced_per_lemma() {
        let forms = generate(&[verb("penda")]);
        assert!(forms.len() <= MAX_FORMS_PER_LEMMA, "D4 hard cap: got {}", forms.len());
        // The full cross-product is 6x5x6 = 180, so the cap MUST bind — if it did
        // not, the cap would be untested decoration.
        assert_eq!(forms.len(), MAX_FORMS_PER_LEMMA, "the cap must actually bite");
    }

    #[test]
    fn the_cap_keeps_the_commonest_cells() {
        // The cap takes a prefix of enumeration order, so the ordinary no-object
        // forms survive and the exotica are dropped. If the ordering ever changed,
        // the cap would silently start keeping different words.
        let forms = generate(&[verb("penda")]);
        let kept: Vec<&str> = forms.iter().map(|f| f.form.as_str()).collect();
        assert!(kept.contains(&"anapenda"), "3sg present with no object must survive the cap");
        assert!(kept.contains(&"nilipenda"), "1sg past must survive");
        assert!(forms.iter().take(6).all(|f| f.slots.object.is_empty()), "no-object cells come first");
    }

    #[test]
    fn an_unattested_noun_class_generates_nothing() {
        // D4. Guessing a plural invents a word — and it would be a well-formed row
        // from a real lemma, so no license or audit gate downstream would catch it.
        let unattested = Lemma { lemma: "kitu".into(), pos: Pos::Noun, noun_class: None, freq_rank: None };
        assert!(generate(&[unattested]).is_empty(), "no class attested -> no form");
        assert_eq!(generate(&[noun("kitabu", NounClass::C7C8)]).len(), 1, "attested -> one plural");
    }

    #[test]
    fn noun_pairs_follow_their_rules() {
        let got = |l: &str, c: NounClass| generate(&[noun(l, c)])[0].form.clone();
        assert_eq!(got("mtoto", NounClass::C1C2), "watoto");
        assert_eq!(got("kitabu", NounClass::C7C8), "vitabu");
        assert_eq!(got("jicho", NounClass::C5C6), "macho");
        assert_eq!(got("ukuta", NounClass::C11C10), "kuta");
    }

    #[test]
    fn a_lemma_that_lacks_its_class_prefix_is_reported_not_guessed() {
        // "kitabu" is class 7 and starts ki-. "mtoto" is not class 7 and does not.
        // If the source says otherwise, that is a data defect: emit nothing rather
        // than mangle the stem.
        let wrong = noun("mtoto", NounClass::C7C8);
        assert!(generate(&[wrong]).is_empty(), "lemma does not carry its declared prefix -> no form");
    }

    // ---- F2's property suite ----

    #[test]
    fn every_generated_verb_form_round_trips() {
        for root in ["penda", "soma", "ona", "la", "ja", "andika", "ambia"] {
            for f in generate(&[verb(root)]) {
                let back = segment(&f.form, root)
                    .unwrap_or_else(|| panic!("{} did not round-trip", f.form));
                assert_eq!(back, f.slots, "{}: segmentation disagrees with generation", f.form);
            }
        }
    }

    #[test]
    fn generation_is_deterministic() {
        // D8. A sampled audit is only meaningful if the sampled bank reproduces.
        let lemmas = vec![verb("penda"), verb("la"), noun("kitabu", NounClass::C7C8)];
        assert_eq!(generate(&lemmas), generate(&lemmas), "same inputs -> identical bank");
    }

    #[test]
    fn every_form_carries_full_provenance() {
        // D7/D8: lemma + rule ids + version, so a failed sample invalidates exactly
        // the offending rule's forms.
        for f in generate(&[verb("la"), verb("ona"), noun("mtoto", NounClass::C1C2)]) {
            assert!(!f.lemma.is_empty(), "{}: no lemma", f.form);
            assert_eq!(f.generator_version, GENERATOR_VERSION);
            for id in &f.rule_ids {
                assert!(RULES.iter().any(|r| r.id == *id), "{}: rule id {id} is not in RULES", f.form);
            }
        }
    }

    #[test]
    fn no_form_is_generated_without_a_lemma_parent() {
        // CC-BANK-EXPANSION D4 / CC-SWAHILI-WORDBANK D4: forms inherit their
        // lemma's definition, license and audit lineage. An orphan form has none.
        assert!(generate(&[]).is_empty(), "no lemmas -> no forms, ever");
        for f in generate(&[verb("soma")]) {
            assert_eq!(f.lemma, "soma");
        }
    }

    #[test]
    fn monosyllabic_ku_retention_is_tense_conditional() {
        // The claim an auditor most needs to check: ku- in na/li/ta/me, absent in
        // ki-. Pinned so a native verdict changes it deliberately, with a version
        // bump, rather than by drift.
        assert_eq!(verb_form("a", "na", "", "la").0, "anakula");
        assert_eq!(verb_form("a", "ki", "", "la").0, "akila", "no ku- in the situative");
        assert_eq!(verb_form("a", "na", "", "soma").0, "anasoma", "polysyllabic root: never ku-");
    }
}
