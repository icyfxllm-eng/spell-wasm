//! §8 — configuration, all from environment variables.

use std::net::SocketAddr;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_public: SocketAddr,
    pub bind_admin: SocketAddr,
    pub trust_proxy: bool,
    pub db_path: String,
    pub tts_base: String,
    pub allowed_origins: Vec<String>,
    pub max_conns: usize,
    pub protocol_min: u32,
    pub drain: Duration,
    pub log: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing(&'static str),
    Invalid(&'static str, String),
    /// A4.2: the client IP comes from CF-Connecting-IP, which is only safe to
    /// trust when nothing but the local tunnel can reach the public listener.
    TrustProxyOnPublicBind(SocketAddr),
    AdminNotLoopback(SocketAddr),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Missing(k) => write!(f, "{k} is required and not set"),
            ConfigError::Invalid(k, v) => write!(f, "{k} has an invalid value: {v:?}"),
            ConfigError::TrustProxyOnPublicBind(a) => write!(
                f,
                "refusing to start: SPELL_TRUST_PROXY=true but SPELL_BIND_PUBLIC={a} is not loopback; \
                 a non-loopback listener would let anyone forge CF-Connecting-IP"
            ),
            ConfigError::AdminNotLoopback(a) => {
                write!(f, "refusing to start: SPELL_BIND_ADMIN={a} must be a loopback address")
            }
        }
    }
}

impl Config {
    /// Reads the process environment.
    pub fn from_env() -> Result<Config, ConfigError> {
        Config::from_lookup(|k| std::env::var(k).ok())
    }

    /// Reads configuration through `get` (tests pass a map).
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Config, ConfigError> {
        let addr = |k: &'static str, default: &str| -> Result<SocketAddr, ConfigError> {
            let v = get(k).unwrap_or_else(|| default.to_string());
            v.parse().map_err(|_| ConfigError::Invalid(k, v))
        };
        let boolean = |k: &'static str, default: bool| -> Result<bool, ConfigError> {
            match get(k).as_deref() {
                None => Ok(default),
                Some("true" | "1") => Ok(true),
                Some("false" | "0") => Ok(false),
                Some(v) => Err(ConfigError::Invalid(k, v.to_string())),
            }
        };
        let number = |k: &'static str, default: u64| -> Result<u64, ConfigError> {
            match get(k) {
                None => Ok(default),
                Some(v) => v.parse().map_err(|_| ConfigError::Invalid(k, v)),
            }
        };
        let tts_base = get("SPELL_TTS_BASE").filter(|s| !s.is_empty()).ok_or(ConfigError::Missing("SPELL_TTS_BASE"))?;
        if !(tts_base.starts_with("http://") || tts_base.starts_with("https://")) {
            return Err(ConfigError::Invalid("SPELL_TTS_BASE", tts_base));
        }
        let cfg = Config {
            bind_public: addr("SPELL_BIND_PUBLIC", "127.0.0.1:8080")?,
            bind_admin: addr("SPELL_BIND_ADMIN", "127.0.0.1:8081")?,
            trust_proxy: boolean("SPELL_TRUST_PROXY", true)?,
            db_path: get("SPELL_DB_PATH").unwrap_or_else(|| "/data/spell.db".into()),
            tts_base: tts_base.trim_end_matches('/').to_string(),
            allowed_origins: get("SPELL_ALLOWED_ORIGINS")
                .unwrap_or_else(|| "capacitor://localhost".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            max_conns: number("SPELL_MAX_CONNS", 2000)? as usize,
            protocol_min: number("SPELL_PROTOCOL_MIN", 1)? as u32,
            drain: Duration::from_secs(number("SPELL_DRAIN_SECS", 900)?),
            log: get("SPELL_LOG").unwrap_or_else(|| "info".into()),
        };
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.trust_proxy && !self.bind_public.ip().is_loopback() {
            return Err(ConfigError::TrustProxyOnPublicBind(self.bind_public));
        }
        if !self.bind_admin.ip().is_loopback() {
            return Err(ConfigError::AdminNotLoopback(self.bind_admin));
        }
        if self.protocol_min > crate::protocol::PROTOCOL_CURRENT {
            return Err(ConfigError::Invalid("SPELL_PROTOCOL_MIN", self.protocol_min.to_string()));
        }
        Ok(())
    }

    /// A configuration for in-process tests.
    pub fn for_tests(db_path: &str) -> Config {
        Config {
            bind_public: "127.0.0.1:0".parse().unwrap(),
            bind_admin: "127.0.0.1:0".parse().unwrap(),
            trust_proxy: true,
            db_path: db_path.to_string(),
            tts_base: "http://127.0.0.1:9".into(),
            allowed_origins: vec!["capacitor://localhost".into()],
            max_conns: 2000,
            protocol_min: 1,
            drain: Duration::from_secs(900),
            log: "info".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn with(pairs: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let m: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Config::from_lookup(|k| m.get(k).cloned())
    }

    #[test]
    fn defaults_match_section_8() {
        let c = with(&[("SPELL_TTS_BASE", "http://100.64.0.2:8000")]).unwrap();
        assert_eq!(c.bind_public.to_string(), "127.0.0.1:8080");
        assert_eq!(c.bind_admin.to_string(), "127.0.0.1:8081");
        assert!(c.trust_proxy);
        assert_eq!(c.db_path, "/data/spell.db");
        assert_eq!(c.max_conns, 2000);
        assert_eq!(c.protocol_min, 1);
        assert_eq!(c.drain, Duration::from_secs(900));
        assert_eq!(c.allowed_origins, vec!["capacitor://localhost".to_string()]);
    }

    #[test]
    fn a4_2_trust_proxy_on_a_public_bind_is_refused() {
        let e = with(&[("SPELL_TTS_BASE", "http://x"), ("SPELL_BIND_PUBLIC", "0.0.0.0:8080")]).unwrap_err();
        assert!(matches!(e, ConfigError::TrustProxyOnPublicBind(_)));
        assert!(e.to_string().contains("not loopback"));
        // Without trust-proxy the same bind is allowed.
        assert!(with(&[
            ("SPELL_TTS_BASE", "http://x"),
            ("SPELL_BIND_PUBLIC", "0.0.0.0:8080"),
            ("SPELL_TRUST_PROXY", "false")
        ])
        .is_ok());
    }

    #[test]
    fn tts_base_is_required_no_silent_default() {
        assert_eq!(with(&[]).unwrap_err(), ConfigError::Missing("SPELL_TTS_BASE"));
    }

    #[test]
    fn admin_must_be_loopback() {
        let e = with(&[("SPELL_TTS_BASE", "http://x"), ("SPELL_BIND_ADMIN", "0.0.0.0:8081")]).unwrap_err();
        assert!(matches!(e, ConfigError::AdminNotLoopback(_)));
    }
}
