//! CC-TELEMETRY-FOUNDATION v1.1 I9 — the ONE telemetry schema.
//!
//! Every byte that leaves the device for the telemetry endpoint is one of the
//! records below. The JS glue (`telemetry-schema.js`) and the Worker's
//! validator (`workers/telemetry/schema.json`) are GENERATED from `RECORDS` by
//! `bindings_are_current` (bless with `TELEMETRY_BLESS=1 cargo test --lib
//! telemetry`). Nothing else may define an event type (acceptance 8).
//!
//! I2 is structural: `FieldType` has no string variant. A field is an enum
//! drawn from a fixed list, a bounded number, a bool, a 64-bit hash, or the
//! build stamp — so free text cannot be expressed, let alone sent.
//! I3 (no learning data) is enforced by `schema_lint` on field names.

use serde::Serialize;

/// Bumped on any breaking change to `RECORDS`. The Worker rejects other values.
pub const SCHEMA_VERSION: u32 = 1;

/// A closed string enum: the Rust type, its wire values, and `ALL` for the
/// schema, from one declaration.
macro_rules! wire_enum {
    // A variant may carry its own attributes: `#[cfg(not(feature = "web"))]`
    // keeps an app-only mode's name out of the site build (CC-PICTURE-PLATFORM
    // I1), in the enum, its wire list and both matches.
    ($(#[$m:meta])* $name:ident { $($(#[$vm:meta])* $var:ident => $wire:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name { $($(#[$vm])* $var),+ }
        impl $name {
            pub const ALL: &'static [&'static str] = &[$($(#[$vm])* $wire),+];
            pub fn as_str(self) -> &'static str { match self { $($(#[$vm])* $name::$var => $wire),+ } }
            #[allow(dead_code)] // not every enum is parsed back
            pub fn from_wire(s: &str) -> Option<Self> { match s { $($(#[$vm])* $wire => Some($name::$var),)+ _ => None } }
        }
        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_str(self.as_str()) }
        }
    };
}

wire_enum!(Platform { Ios => "ios", Android => "android", Web => "web", Desktop => "desktop" });

wire_enum!(
    /// F1 — what broke. `native_*` come from MetricKit via the iOS bridge.
    ErrorCode {
        JsUncaught => "js_uncaught",
        JsUnhandledRejection => "js_unhandled_rejection",
        WasmPanic => "wasm_panic",
        WasmInitFailed => "wasm_init_failed",
        PackLoadFailed => "pack_load_failed",
        NativeCrash => "native_crash",
        NativeHang => "native_hang",
    }
);

wire_enum!(
    /// F5 — what was measured. The ms metrics take an ms bucket; the
    /// resolution metric takes `resolved` or `unavailable` (`Bucket::fits`).
    PerfMetric {
        WasmInitMs => "wasm_init_ms",
        TapToAudioMs => "tap_to_audio_ms",
        AudioResolution => "audio_resolution",
    }
);

wire_enum!(
    /// F5 — a histogram bucket. Never a raw number of milliseconds.
    Bucket {
        Lt100 => "lt100", Lt250 => "lt250", Lt500 => "lt500", Lt1000 => "lt1000",
        Lt2000 => "lt2000", Lt5000 => "lt5000", Ge5000 => "ge5000",
        Resolved => "resolved", Unavailable => "unavailable",
    }
);

impl Bucket {
    pub fn of_ms(ms: f64) -> Bucket {
        match ms {
            x if x < 100.0 => Bucket::Lt100,
            x if x < 250.0 => Bucket::Lt250,
            x if x < 500.0 => Bucket::Lt500,
            x if x < 1000.0 => Bucket::Lt1000,
            x if x < 2000.0 => Bucket::Lt2000,
            x if x < 5000.0 => Bucket::Lt5000,
            _ => Bucket::Ge5000,
        }
    }
    /// Whether this bucket belongs to `metric` (the pairing the schema can't express).
    pub fn fits(self, metric: PerfMetric) -> bool {
        let resolution = matches!(self, Bucket::Resolved | Bucket::Unavailable);
        resolution == (metric == PerfMetric::AudioResolution)
    }
}

wire_enum!(
    /// The study language when it broke. `Mine` is My Words (its contents are
    /// never sent, I4); `Other` is any code not listed here.
    Lang {
        En => "en", Es => "es", Fr => "fr", De => "de", Pt => "pt", Pl => "pl",
        Vi => "vi", Ko => "ko", Ja => "ja", Fil => "fil", Ru => "ru", Sw => "sw",
        Ar => "ar", Hi => "hi", Fa => "fa", Zh => "zh", Mine => "mine", Other => "other",
    }
);

wire_enum!(
    /// The last mode entered this launch (`telemetry::set_mode`). A coarse
    /// "where was the player" hint, never a lifecycle event (I3).
    Mode {
        Home => "home", Standard => "standard", Daily => "daily", Review => "review",
        Versus => "versus", Practice => "practice", GhostRacing => "ghost_racing",
        Racing => "racing", SayIt => "say_it", DefMatch => "def_match",
        LetterForge => "letter_forge", WordChains => "word_chains", Impostor => "impostor",
        BeeSim => "bee_sim",
        #[cfg(not(feature = "web"))]
        WordPicture => "word_picture",
        Translate => "translate",
        OnlineSpelloff => "online_spelloff", MyLists => "my_lists",
    }
);

/// A 64-bit FNV-1a hash, on the wire as 16 lowercase hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash64(pub u64);

impl Serialize for Hash64 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:016x}", self.0))
    }
}

impl Hash64 {
    pub fn of(bytes: &[u8]) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Hash64(h)
    }
    pub fn parse(s: &str) -> Option<Self> {
        (s.len() == 16 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
            .then(|| u64::from_str_radix(s, 16).ok().map(Hash64))
            .flatten()
    }
}

/// The dist content stamp `build-web.sh` writes into `window.SPELL_BUILD`
/// (12 lowercase hex), or `dev` for an unstamped build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildId(String);

impl BuildId {
    pub fn parse(s: &str) -> Self {
        let ok = s.len() == 12 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        BuildId(if ok { s.to_string() } else { "dev".to_string() })
    }
}

impl Serialize for BuildId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_str(&self.0) }
}

// ---- Records ---------------------------------------------------------------

/// F1 — one error, standard players only.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErrorEvent {
    pub error_code: ErrorCode,
    pub lang: Lang,
    pub mode: Mode,
    pub stack_hash: Hash64,
}

/// F5 — a standard player's histogram cell since the last send.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerfRow {
    pub metric: PerfMetric,
    pub bucket: Bucket,
    pub lang: Lang,
    pub count: u32,
}

/// F5 + F6 — an aggregate device's histogram cell: no language.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AggPerfRow {
    pub metric: PerfMetric,
    pub bucket: Bucket,
    pub count: u32,
}

/// POST /v1/events — a standard player's batch. `session_id` is random per
/// launch and never reused (D4).
#[derive(Debug, Clone, Serialize)]
pub struct EventBatch {
    pub v: u32,
    pub build: BuildId,
    pub platform: Platform,
    pub session_id: Hash64,
    pub events: Vec<ErrorEvent>,
    pub perf: Vec<PerfRow>,
}

/// F6 — one aggregate row: how many times `error_code` happened since the last send.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AggRow {
    pub error_code: ErrorCode,
    pub count: u32,
}

/// POST /v1/aggregate — Jr, unknown-age, and Education devices. No identifier
/// of any kind; at most one per day (F6).
#[derive(Debug, Clone, Serialize)]
pub struct AggBatch {
    pub v: u32,
    pub build: BuildId,
    pub platform: Platform,
    pub rows: Vec<AggRow>,
    pub perf: Vec<AggPerfRow>,
}

/// GET /v1/flags — the remote kill switch (I7, R4). The Worker's answer;
/// the client validates it against `RECORDS` rather than deserializing this.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct Flags {
    pub telemetry_enabled: bool,
}

// ---- The schema as data ----------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Enum(&'static [&'static str]),
    UInt { max: u64 },
    Bool,
    Hash64,
    BuildId,
    List { of: &'static str, max: u32 },
    /// Always exactly SCHEMA_VERSION.
    Version,
}

#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub name: &'static str,
    pub ty: FieldType,
}

#[derive(Debug, Clone, Copy)]
pub struct Record {
    pub name: &'static str,
    pub fields: &'static [Field],
}

/// Transport caps (Transport and storage).
pub const QUEUE_MAX_EVENTS: u32 = 200;
pub const QUEUE_MAX_BYTES: usize = 128 * 1024;
pub const AGG_MAX_COUNT: u64 = 1_000_000;
/// Every (metric, bucket, lang) cell, so a full histogram always fits.
pub const PERF_MAX_ROWS: u32 = (PerfMetric::ALL.len() * Bucket::ALL.len() * Lang::ALL.len()) as u32;

const fn f(name: &'static str, ty: FieldType) -> Field { Field { name, ty } }

pub const RECORDS: &[Record] = &[
    Record { name: "error_event", fields: &[
        f("error_code", FieldType::Enum(ErrorCode::ALL)),
        f("lang", FieldType::Enum(Lang::ALL)),
        f("mode", FieldType::Enum(Mode::ALL)),
        f("stack_hash", FieldType::Hash64),
    ]},
    Record { name: "event_batch", fields: &[
        f("v", FieldType::Version),
        f("build", FieldType::BuildId),
        f("platform", FieldType::Enum(Platform::ALL)),
        f("session_id", FieldType::Hash64),
        f("events", FieldType::List { of: "error_event", max: QUEUE_MAX_EVENTS }),
        f("perf", FieldType::List { of: "perf_row", max: PERF_MAX_ROWS }),
    ]},
    Record { name: "perf_row", fields: &[
        f("metric", FieldType::Enum(PerfMetric::ALL)),
        f("bucket", FieldType::Enum(Bucket::ALL)),
        f("lang", FieldType::Enum(Lang::ALL)),
        f("count", FieldType::UInt { max: AGG_MAX_COUNT }),
    ]},
    Record { name: "agg_perf_row", fields: &[
        f("metric", FieldType::Enum(PerfMetric::ALL)),
        f("bucket", FieldType::Enum(Bucket::ALL)),
        f("count", FieldType::UInt { max: AGG_MAX_COUNT }),
    ]},
    Record { name: "agg_row", fields: &[
        f("error_code", FieldType::Enum(ErrorCode::ALL)),
        f("count", FieldType::UInt { max: AGG_MAX_COUNT }),
    ]},
    Record { name: "agg_batch", fields: &[
        f("v", FieldType::Version),
        f("build", FieldType::BuildId),
        f("platform", FieldType::Enum(Platform::ALL)),
        f("rows", FieldType::List { of: "agg_row", max: ErrorCode::ALL.len() as u32 }),
        f("perf", FieldType::List { of: "agg_perf_row", max: (PerfMetric::ALL.len() * Bucket::ALL.len()) as u32 }),
    ]},
    Record { name: "flags", fields: &[
        f("telemetry_enabled", FieldType::Bool),
    ]},
];

pub fn record(name: &str) -> Option<&'static Record> {
    RECORDS.iter().find(|r| r.name == name)
}

/// Validate `v` against record `name` exactly as the Worker does: every
/// declared field present and well-typed, nothing undeclared.
pub fn validate(name: &str, v: &serde_json::Value) -> Result<(), String> {
    let rec = record(name).ok_or_else(|| format!("unknown record {name}"))?;
    let obj = v.as_object().ok_or_else(|| format!("{name}: not an object"))?;
    for k in obj.keys() {
        if !rec.fields.iter().any(|fd| fd.name == k) {
            return Err(format!("{name}: undeclared field {k}"));
        }
    }
    for fd in rec.fields {
        let x = obj.get(fd.name).ok_or_else(|| format!("{name}.{}: missing", fd.name))?;
        let ok = match fd.ty {
            FieldType::Enum(vals) => x.as_str().is_some_and(|s| vals.contains(&s)),
            FieldType::UInt { max } => x.as_u64().is_some_and(|n| n <= max),
            FieldType::Bool => x.is_boolean(),
            FieldType::Hash64 => x.as_str().is_some_and(|s| Hash64::parse(s).is_some()),
            FieldType::BuildId => x.as_str().is_some_and(|s| s == "dev" || BuildId::parse(s).0 == s),
            FieldType::Version => x.as_u64() == Some(SCHEMA_VERSION as u64),
            FieldType::List { of, max } => match x.as_array() {
                Some(items) if items.len() <= max as usize => {
                    for it in items {
                        validate(of, it)?;
                    }
                    true
                }
                _ => false,
            },
        };
        if !ok {
            return Err(format!("{name}.{}: bad value {x}", fd.name));
        }
    }
    Ok(())
}

/// The schema as JSON — the Worker's validator input and the JS binding's source.
#[cfg_attr(not(test), allow(dead_code))] // the generator runs under `cargo test`
pub fn schema_json() -> serde_json::Value {
    let recs: serde_json::Map<String, serde_json::Value> = RECORDS
        .iter()
        .map(|r| {
            let fields: Vec<serde_json::Value> = r
                .fields
                .iter()
                .map(|fd| {
                    let ty = match fd.ty {
                        FieldType::Enum(vals) => serde_json::json!({ "enum": vals }),
                        FieldType::UInt { max } => serde_json::json!({ "uint": max }),
                        FieldType::Bool => serde_json::json!("bool"),
                        FieldType::Hash64 => serde_json::json!("hash64"),
                        FieldType::BuildId => serde_json::json!("build"),
                        FieldType::Version => serde_json::json!("version"),
                        FieldType::List { of, max } => serde_json::json!({ "list": of, "max": max }),
                    };
                    serde_json::json!({ "name": fd.name, "type": ty })
                })
                .collect();
            (r.name.to_string(), serde_json::Value::Array(fields))
        })
        .collect();
    serde_json::json!({ "version": SCHEMA_VERSION, "records": recs })
}

/// The generated browser binding: constants the pre-WASM error hook in
/// index.html needs, and nothing it could use to invent a new event.
#[cfg_attr(not(test), allow(dead_code))]
pub fn js_binding() -> String {
    format!(
        "// GENERATED by src/telemetry/schema.rs (bindings_are_current). Do not edit.\n\
         // Bless: TELEMETRY_BLESS=1 cargo test --lib telemetry\n\
         window.SpellTelemetrySchema = Object.freeze({{\n  \
           version: {},\n  \
           errorCodes: Object.freeze({}),\n  \
           jsBufferKey: {:?},\n  \
           jsBufferMax: {},\n\
         }});\n",
        SCHEMA_VERSION,
        serde_json::to_string(ErrorCode::ALL).unwrap_or_default(),
        super::JSBUF_KEY,
        super::JSBUF_MAX,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> ErrorEvent {
        ErrorEvent { error_code: ErrorCode::WasmPanic, lang: Lang::Ru, mode: Mode::Daily, stack_hash: Hash64::of(b"x") }
    }

    /// Acceptance 1 — no unbounded string, no learning-data field (I2, I3).
    /// `FieldType` cannot express a string, so the string half is proven by
    /// construction; this pins it and checks names against the I3 list.
    #[test]
    fn schema_lint() {
        const FORBIDDEN: &[&str] = &[
            "word", "answer", "typed", "attempt", "miss", "hint", "outcome", "verdict",
            "score", "start", "complete", "abandon", "funnel", "email", "account", "user",
            "install", "ip", "text", "audio", "photo", "ocr", "list_", "custom",
        ];
        for r in RECORDS {
            for fd in r.fields {
                for bad in FORBIDDEN {
                    assert!(!fd.name.contains(bad), "{}.{} looks like forbidden data ({bad})", r.name, fd.name);
                }
                if let FieldType::List { of, .. } = fd.ty {
                    assert!(record(of).is_some(), "{}.{} lists unknown record {of}", r.name, fd.name);
                }
                if let FieldType::Enum(vals) = fd.ty {
                    assert!(!vals.is_empty() && vals.iter().all(|v| v.len() <= 32 && v.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')));
                }
            }
        }
    }

    /// Every Rust record serializes to exactly what `RECORDS` declares.
    #[test]
    fn rust_types_match_schema() {
        let ev = sample_event();
        validate("error_event", &serde_json::to_value(&ev).unwrap()).unwrap();
        let batch = EventBatch { v: SCHEMA_VERSION, build: BuildId::parse("0123456789ab"), platform: Platform::Ios, session_id: Hash64(7), events: vec![ev], perf: vec![PerfRow { metric: PerfMetric::TapToAudioMs, bucket: Bucket::Lt250, lang: Lang::Ko, count: 4 }] };
        validate("event_batch", &serde_json::to_value(&batch).unwrap()).unwrap();
        let agg = AggBatch { v: SCHEMA_VERSION, build: BuildId::parse("DEV"), platform: Platform::Web, rows: vec![AggRow { error_code: ErrorCode::JsUncaught, count: 3 }], perf: vec![AggPerfRow { metric: PerfMetric::AudioResolution, bucket: Bucket::Unavailable, count: 1 }] };
        let aj = serde_json::to_value(&agg).unwrap();
        validate("agg_batch", &aj).unwrap();
        assert!(aj.get("session_id").is_none(), "an aggregate carries no identifier (F6)");
        assert!(!aj.to_string().contains("\"lang\""), "an aggregate carries no language (F6)");
        validate("flags", &serde_json::to_value(Flags { telemetry_enabled: false }).unwrap()).unwrap();
    }

    #[test]
    fn validator_rejects_free_text_and_extras() {
        let mut v = serde_json::to_value(sample_event()).unwrap();
        v["lang"] = serde_json::json!("my secret word");
        assert!(validate("error_event", &v).is_err());
        let mut v = serde_json::to_value(sample_event()).unwrap();
        v["note"] = serde_json::json!("hi");
        assert!(validate("error_event", &v).is_err());
        let mut v = serde_json::to_value(sample_event()).unwrap();
        v["stack_hash"] = serde_json::json!("at foo (app.js:1:2)");
        assert!(validate("error_event", &v).is_err());
    }

    #[test]
    fn buckets_are_ranges_and_pair_with_their_metric() {
        assert_eq!(Bucket::of_ms(0.0), Bucket::Lt100);
        assert_eq!(Bucket::of_ms(99.9), Bucket::Lt100);
        assert_eq!(Bucket::of_ms(100.0), Bucket::Lt250);
        assert_eq!(Bucket::of_ms(4999.0), Bucket::Lt5000);
        assert_eq!(Bucket::of_ms(60_000.0), Bucket::Ge5000);
        assert!(Bucket::of_ms(300.0).fits(PerfMetric::TapToAudioMs));
        assert!(!Bucket::of_ms(300.0).fits(PerfMetric::AudioResolution));
        assert!(Bucket::Resolved.fits(PerfMetric::AudioResolution));
        assert!(!Bucket::Unavailable.fits(PerfMetric::WasmInitMs));
    }

    #[test]
    fn build_id_only_accepts_the_stamp() {
        assert_eq!(serde_json::to_value(BuildId::parse("0a1b2c3d4e5f")).unwrap(), "0a1b2c3d4e5f");
        assert_eq!(serde_json::to_value(BuildId::parse("DEV")).unwrap(), "dev");
        assert_eq!(serde_json::to_value(BuildId::parse("hello world!")).unwrap(), "dev");
    }

    /// Acceptance 8 — no event is defined anywhere else. Any file that emits,
    /// stores or reads telemetry has to name these fields; the only files that
    /// may are this schema, its generated bindings, and the listed consumers.
    #[test]
    fn single_source() {
        const MARKERS: &[&str] = &["stack_hash", "session_id", "error_code"];
        const ALLOWED: &[&str] = &[
            "src/telemetry/schema.rs",          // the definition
            "src/telemetry/mod.rs",             // the client transport
            "workers/telemetry/schema.json",    // generated
            "workers/telemetry/src/index.js",   // the endpoint (validates via schema.json)
            "workers/telemetry/migrations/0001_init.sql",
            "workers/telemetry/reports/crashes.sql", // acceptance 9 report
            "workers/telemetry/test/worker.test.js",
            "workers/telemetry/README.md",
            "tests/e2e/specs/telemetry.mjs",
        ];
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut offenders = Vec::new();
        let mut stack = vec![root.join("src"), root.join("workers"), root.join("backend"), root.join("tests"), root.join("scripts")];
        let mut files: Vec<std::path::PathBuf> = ["index.html", "audio-native.js", "native-language-kit.js", "sw.js", "telemetry-schema.js"]
            .iter()
            .map(|f| root.join(f))
            .collect();
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if p.is_dir() {
                    if !["node_modules", "target", "shots", "__pycache__", "venv"].contains(&name) {
                        stack.push(p);
                    }
                } else if [".rs", ".js", ".mjs", ".py", ".json", ".sql", ".html", ".swift", ".ts"].iter().any(|x| name.ends_with(x)) {
                    files.push(p);
                }
            }
        }
        for p in files {
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            if ALLOWED.contains(&rel.as_str()) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            if let Some(m) = MARKERS.iter().find(|m| text.contains(*m)) {
                offenders.push(format!("{rel} ({m})"));
            }
        }
        assert!(offenders.is_empty(), "telemetry fields outside the single source: {offenders:?}");
    }

    /// I9 — the generated bindings on disk equal what this schema produces.
    /// They are generated from the app build, whose modes are a superset of
    /// the site's, so the site configuration does not compare them.
    #[cfg(not(feature = "web"))]
    #[test]
    fn bindings_are_current() {
        let root = env!("CARGO_MANIFEST_DIR");
        let outputs = [
            (format!("{root}/telemetry-schema.js"), js_binding()),
            (format!("{root}/workers/telemetry/schema.json"), format!("{:#}\n", schema_json())),
        ];
        for (path, want) in outputs {
            if std::env::var("TELEMETRY_BLESS").is_ok() {
                std::fs::write(&path, &want).unwrap();
            }
            let have = std::fs::read_to_string(&path).unwrap_or_default();
            assert_eq!(have, want, "{path} is stale: TELEMETRY_BLESS=1 cargo test --lib telemetry");
        }
    }
}
