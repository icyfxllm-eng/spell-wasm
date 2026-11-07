//! CC-GUARDIAN-REPORT v1.1 — where Report Days keeps its schedule on the
//! device. The one write door for the schedule, kept out of the report screen
//! on purpose: that screen is read-only over learner state (R-I1) and a test
//! holds it to that. The schedule is not learner state.

use crate::report_days::DaysState;

/// One per profile; today the profile is the device.
const KEY: &str = "spell_report_days_v1";

/// What is stored, or `None` when nothing is stored or it is unreadable (an
/// empty weekday set included): the caller then uses the default.
pub fn load() -> Option<DaysState> {
    crate::storage::get_raw(KEY).and_then(|s| DaysState::decode(&s))
}

pub fn save(state: &DaysState) {
    crate::storage::set_raw(KEY, &state.encode());
}
