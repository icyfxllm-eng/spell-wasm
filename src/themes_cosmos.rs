//! CC-THEMES-COSMOS v1: the twelve earned themes beyond the Classic five
//! (Metals, Sky, Moon, Space, Wonders), as registry data and pure look
//! resolution. Phase A: every look is built and reviewable in the Theme Lab;
//! none is earnable or listed in Appearance yet (no unlock wiring, T-D6).
//!
//! Names and facts are English-only and approval-pinned (T-D4, F6): changing
//! one needs Eric's approval, and `the_fact_cards_are_pinned` fails until the
//! pin is moved on purpose.
//!
//! The Sky band, the moon phase and the planet are a pure function of a local
//! date and time plus the Lab's pins. The Lab clock override reaches only this
//! resolver (I-T11): nothing here is read by Daily, streaks, the Climb or timers.

// Names, facts, waves and the planet table are read by the Theme Lab today and
// by each wave's unlock moment later; a store build has neither yet.
#![cfg_attr(not(feature = "theme_lab"), allow(dead_code))]

use crate::astro::{self, Band};
use crate::themes::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Classic,
    Metals,
    Sky,
    Moon,
    Space,
    Wonders,
}

impl Family {
    pub fn name(self) -> &'static str {
        match self {
            Family::Classic => "Classic",
            Family::Metals => "Metals",
            Family::Sky => "Sky",
            Family::Moon => "Moon",
            Family::Space => "Space",
            Family::Wonders => "Wonders",
        }
    }
}

/// One registry row (F2).
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub theme: Theme,
    pub family: Family,
    /// The release wave (T-D6); 0 for the Classic five.
    pub wave: u8,
    /// English-only (T-D4).
    pub name: &'static str,
    /// How it is earned, as Lab copy (the player-facing strings arrive with each wave).
    pub unlock: &'static str,
    /// F6, pinned. None for the Classic five.
    pub fact: Option<&'static str>,
}

pub const REGISTRY: [Entry; 17] = [
    Entry { theme: Theme::Eclipse, family: Family::Classic, wave: 0, name: "Eclipse", unlock: "Free", fact: None },
    Entry { theme: Theme::Daybreak, family: Family::Classic, wave: 0, name: "Daybreak", unlock: "Free", fact: None },
    Entry { theme: Theme::Chalkboard, family: Family::Classic, wave: 0, name: "Chalkboard", unlock: "7-day Daily streak", fact: None },
    Entry { theme: Theme::Typewriter, family: Family::Classic, wave: 0, name: "Typewriter", unlock: "500 correct words", fact: None },
    Entry { theme: Theme::BeeStage, family: Family::Classic, wave: 0, name: "Spelling Bee Stage", unlock: "100 correct at Hard or above", fact: None },
    Entry { theme: Theme::Bronze, family: Family::Metals, wave: 1, name: "Bronze", unlock: "250 lifetime correct words", fact: Some("Bronze is a mix of copper and tin.") },
    Entry { theme: Theme::Silver, family: Family::Metals, wave: 1, name: "Silver", unlock: "1,500 lifetime correct words", fact: Some("Silver's symbol is Ag, from the Latin word argentum.") },
    Entry { theme: Theme::Gold, family: Family::Metals, wave: 1, name: "Gold", unlock: "5,000 lifetime correct words", fact: Some("Gold's symbol is Au, from the Latin word aurum.") },
    Entry { theme: Theme::Diamond, family: Family::Metals, wave: 1, name: "Diamond", unlock: "50 Impossible words correct", fact: Some("A diamond is carbon atoms locked in a crystal, the hardest natural material.") },
    Entry { theme: Theme::Sky, family: Family::Sky, wave: 1, name: "Sky", unlock: "3-day streak", fact: Some("Sunsets look red because sunlight crosses more air, which scatters the blue away.") },
    Entry { theme: Theme::Lunar, family: Family::Moon, wave: 1, name: "Lunar", unlock: "Play on all 4 principal moon phases", fact: Some("The Moon takes about 29.5 days to cycle through its phases.") },
    Entry { theme: Theme::Solar, family: Family::Space, wave: 2, name: "Solar System", unlock: "Every 5 Daily Challenges unlocks the next planet", fact: Some("Eight planets circle the Sun. Pluto was renamed a dwarf planet in 2006.") },
    Entry { theme: Theme::Aurora, family: Family::Sky, wave: 2, name: "Aurora", unlock: "30 days played", fact: Some("Particles from the Sun strike gases high in our air and make them glow.") },
    Entry { theme: Theme::Beach, family: Family::Sky, wave: 2, name: "Beach", unlock: "First 3 sessions completed", fact: Some("Most beach sand is tiny grains of rock, often quartz.") },
    Entry { theme: Theme::PinkMoon, family: Family::Moon, wave: 3, name: "Pink Moon", unlock: "Play within a day of April's full moon (or on 3 full moons)", fact: Some("April's full moon is named for pink spring wildflowers. The Moon itself isn't pink.") },
    Entry { theme: Theme::BloodMoon, family: Family::Moon, wave: 3, name: "Blood Moon", unlock: "Play within a day of a total lunar eclipse (or on 13 full moons)", fact: Some("In a total lunar eclipse, Earth's shadow covers the Moon, and light bent through our air turns it red.") },
    Entry { theme: Theme::GreatWall, family: Family::Wonders, wave: 3, name: "Great Wall", unlock: "Climb milestone (threshold not signed, O1/O2)", fact: Some("Built and rebuilt over more than 2,000 years. Despite the myth, you can't see it from the Moon.") },
];

pub fn entry(t: Theme) -> &'static Entry {
    REGISTRY.iter().find(|e| e.theme == t).expect("every theme has a registry row")
}

/// The twelve themes that draw art (`data-cosmos` on the root).
pub fn is_cosmos(t: Theme) -> bool {
    entry(t).family != Family::Classic
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Planet {
    Mercury,
    Venus,
    Earth,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
}

pub const PLANETS: [Planet; 8] =
    [Planet::Mercury, Planet::Venus, Planet::Earth, Planet::Mars, Planet::Jupiter, Planet::Saturn, Planet::Uranus, Planet::Neptune];

impl Planet {
    pub fn id(self) -> &'static str {
        match self {
            Planet::Mercury => "mercury",
            Planet::Venus => "venus",
            Planet::Earth => "earth",
            Planet::Mars => "mars",
            Planet::Jupiter => "jupiter",
            Planet::Saturn => "saturn",
            Planet::Uranus => "uranus",
            Planet::Neptune => "neptune",
        }
    }
    pub fn from_id(s: &str) -> Option<Planet> {
        PLANETS.into_iter().find(|p| p.id() == s)
    }
    pub fn name(self) -> &'static str {
        match self {
            Planet::Mercury => "Mercury",
            Planet::Venus => "Venus",
            Planet::Earth => "Earth",
            Planet::Mars => "Mars",
            Planet::Jupiter => "Jupiter",
            Planet::Saturn => "Saturn",
            Planet::Uranus => "Uranus",
            Planet::Neptune => "Neptune",
        }
    }
    /// F6 per-planet facts, pinned.
    pub fn fact(self) -> &'static str {
        match self {
            Planet::Mercury => "Closest to the Sun.",
            Planet::Venus => "The hottest planet.",
            Planet::Earth => "The only planet known to have life.",
            Planet::Mars => "Red because of iron oxide, a kind of rust.",
            Planet::Jupiter => "The largest planet.",
            Planet::Saturn => "Its rings are mostly ice.",
            Planet::Uranus => "Spins tipped on its side.",
            Planet::Neptune => "Has the fastest winds in the solar system.",
        }
    }
}

pub fn band_id(b: Band) -> &'static str {
    match b {
        Band::Sunrise => "sunrise",
        Band::Day => "day",
        Band::Sunset => "sunset",
        Band::Night => "night",
    }
}

pub fn band_from_id(s: &str) -> Option<Band> {
    [Band::Sunrise, Band::Day, Band::Sunset, Band::Night].into_iter().find(|b| band_id(*b) == s)
}

pub const PHASE_NAMES: [&str; 8] =
    ["New", "Waxing crescent", "First quarter", "Waxing gibbous", "Full", "Waning gibbous", "Last quarter", "Waning crescent"];

/// A local calendar date and time (the device clock, or the Lab's override).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalNow {
    pub y: i32,
    pub m: u32,
    pub d: u32,
    pub h: u32,
    pub min: u32,
}

/// The Lab's pins for the applied theme (F1.4). Default: follow the clock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pins {
    pub band: Option<Band>,
    pub phase: Option<u8>,
    pub planet: Option<Planet>,
}

/// What exactly to draw: the theme plus its band, phase or planet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub theme: Theme,
    pub band: Option<Band>,
    /// (index 0..=7, moon age in days); the age draws the terminator.
    pub phase: Option<(u8, f64)>,
    pub planet: Option<Planet>,
}

const SYNODIC: f64 = 29.530_588_853;

/// The look for `theme` at `now`. Solar System shows the Lab's planet, else
/// Mercury (the planet a player has reached arrives with the wave 2 unlocks).
pub fn look(theme: Theme, now: LocalNow, pins: Pins) -> Look {
    let mut l = Look { theme, band: None, phase: None, planet: None };
    match theme {
        Theme::Sky => l.band = Some(pins.band.unwrap_or_else(|| astro::sky_band(now.h, now.min))),
        Theme::Lunar => {
            l.phase = Some(match pins.phase {
                Some(i) => (i.min(7), f64::from(i.min(7)) * SYNODIC / 8.0),
                None => {
                    let p = astro::phase(now.y, now.m, now.d);
                    (p.index, p.age_days)
                }
            })
        }
        Theme::Solar => l.planet = Some(pins.planet.unwrap_or(Planet::Mercury)),
        _ => {}
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOON: LocalNow = LocalNow { y: 2026, m: 11, d: 2, h: 12, min: 0 };

    #[test]
    fn the_registry_holds_all_seventeen_once() {
        assert_eq!(REGISTRY.len(), 17);
        for (i, a) in REGISTRY.iter().enumerate() {
            for b in &REGISTRY[i + 1..] {
                assert_ne!(a.theme.id(), b.theme.id());
            }
        }
        assert_eq!(REGISTRY.iter().filter(|e| e.family == Family::Classic).count(), 5);
        assert!(REGISTRY.iter().filter(|e| e.family != Family::Classic).all(|e| e.fact.is_some() && (1..=3).contains(&e.wave)));
    }

    #[test]
    fn waves_are_t_d6() {
        let wave = |w: u8| REGISTRY.iter().filter(|e| e.wave == w).map(|e| e.theme.id()).collect::<Vec<_>>();
        assert_eq!(wave(1), ["bronze", "silver", "gold", "diamond", "sky", "lunar"]);
        assert_eq!(wave(2), ["solar", "aurora", "beach"]);
        assert_eq!(wave(3), ["pinkmoon", "bloodmoon", "greatwall"]);
    }

    /// F6: the strings are approval-pinned. Changing one changes this hash on
    /// purpose, with Eric's approval recorded in the commit.
    #[test]
    fn the_fact_cards_are_pinned() {
        let mut all = String::new();
        for e in REGISTRY {
            all.push_str(e.name);
            all.push('|');
            all.push_str(e.fact.unwrap_or(""));
            all.push('\n');
        }
        for p in PLANETS {
            all.push_str(p.name());
            all.push('|');
            all.push_str(p.fact());
            all.push('\n');
        }
        // FNV-1a, so the pin needs no dependency.
        let h = all.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
        assert_eq!(h, PINNED_FACTS, "a fact card or theme name changed: that needs Eric's approval (F6), then move the pin");
        assert!(!all.contains("Pluto is a planet"));
    }
    const PINNED_FACTS: u64 = 15_280_899_310_820_233_549;

    #[test]
    fn sky_follows_the_clock_unless_pinned() {
        assert_eq!(look(Theme::Sky, LocalNow { h: 6, ..NOON }, Pins::default()).band, Some(Band::Sunrise));
        assert_eq!(look(Theme::Sky, LocalNow { h: 21, ..NOON }, Pins::default()).band, Some(Band::Night));
        let pinned = Pins { band: Some(Band::Sunset), ..Pins::default() };
        assert_eq!(look(Theme::Sky, LocalNow { h: 6, ..NOON }, pinned).band, Some(Band::Sunset));
    }

    #[test]
    fn lunar_follows_the_date_unless_forced() {
        // 2026-03-03 is a full moon (and a total lunar eclipse).
        let full = look(Theme::Lunar, LocalNow { y: 2026, m: 3, d: 3, ..NOON }, Pins::default());
        assert_eq!(full.phase.map(|p| p.0), Some(4));
        let forced = look(Theme::Lunar, NOON, Pins { phase: Some(2), ..Pins::default() });
        assert_eq!(forced.phase.map(|p| p.0), Some(2));
        assert!((forced.phase.unwrap().1 - SYNODIC / 4.0).abs() < 1e-9);
    }

    #[test]
    fn only_their_own_themes_carry_a_variant() {
        let l = look(Theme::Gold, NOON, Pins { band: Some(Band::Day), phase: Some(3), planet: Some(Planet::Mars) });
        assert_eq!((l.band, l.phase, l.planet), (None, None, None));
        assert_eq!(look(Theme::Solar, NOON, Pins::default()).planet, Some(Planet::Mercury));
    }

    #[test]
    fn eight_planets_and_no_pluto() {
        assert_eq!(PLANETS.len(), 8);
        assert!(Planet::from_id("pluto").is_none());
        assert!(entry(Theme::Solar).fact.unwrap().contains("Pluto"));
    }
}
