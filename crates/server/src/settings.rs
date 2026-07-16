//! Layered runtime config via the [`config`](https://docs.rs/config) crate.
//! Precedence (later wins): built-in defaults → optional `config/mtgfr.toml` → env (`__` separator).

use axum::http::HeaderValue;
use serde::Deserialize;

/// Runtime settings. Env mapping: `HOST`, `PORT`, `DATABASE_URL`, `INSTANCE_ID`, `DRAIN`,
/// `COOKIE_SECURE`, `COOKIE_DOMAIN`, `CORS_ORIGIN`, `ADMIN_TOKEN`, `VERSION`.
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    /// Default `127.0.0.1`; prod `0.0.0.0`.
    pub host: String,
    /// Default `8080`.
    pub port: u16,
    /// From `config/mtgfr.toml` or `DATABASE_URL` — no built-in default.
    pub database_url: String,
    /// Stable per-Deployment id (e.g. `edh-api`), not the pod name. Default `local`.
    pub instance_id: String,
    /// Startup default only; live drain is `POST /admin/drain`.
    pub drain: bool,
    /// Session cookie `Secure`. Default `false` (localhost http).
    pub cookie_secure: bool,
    /// Session cookie `Domain`. Default `""` (host-only); prod `.example.com`.
    pub cookie_domain: String,
    /// CORS allow-origin. Default `""` (no layer — Vite proxy is same-origin).
    pub cors_origin: String,
    /// Surfaced on `/health/live`. Default: crate version.
    pub version: String,
    /// Guards `/admin/drain` and `/health/drain`. Empty = open (NetworkPolicy still applies).
    pub admin_token: String,
}

impl Settings {
    pub fn load() -> Result<Self, config::ConfigError> {
        Self::load_from("config/mtgfr")
    }

    fn load_from(config_path: &str) -> Result<Self, config::ConfigError> {
        let settings: Settings = config::Config::builder()
            .set_default("host", "127.0.0.1")?
            .set_default("port", 8080)?
            .set_default("instance_id", "local")?
            .set_default("drain", false)?
            .set_default("cookie_secure", false)?
            .set_default("cookie_domain", "")?
            .set_default("cors_origin", "")?
            .set_default("version", env!("CARGO_PKG_VERSION"))?
            .set_default("admin_token", "")?
            .add_source(config::File::with_name(config_path).required(false))
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()?;
        validate_cors_origin(&settings.cors_origin)?;
        Ok(settings)
    }

    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn validate_cors_origin(origin: &str) -> Result<(), config::ConfigError> {
    if origin.is_empty() {
        return Ok(());
    }
    HeaderValue::from_str(origin).map(|_| ()).map_err(|e| {
        config::ConfigError::Message(format!(
            "cors_origin {origin:?} is not a valid header value: {e}"
        ))
    })
}

#[cfg(test)]
pub(crate) fn for_test() -> Settings {
    Settings {
        host: "127.0.0.1".to_string(),
        port: 0,
        database_url: "sqlite::memory:".to_string(),
        instance_id: "test".to_string(),
        drain: false,
        cookie_secure: false,
        cookie_domain: String::new(),
        cors_origin: String::new(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        admin_token: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempConfig {
        dir: std::path::PathBuf,
    }

    impl TempConfig {
        fn write(contents: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "mtgfr-settings-test-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            std::fs::create_dir_all(&dir).expect("create temp config dir");
            std::fs::write(dir.join("mtgfr.toml"), contents).expect("write temp config");
            TempConfig { dir }
        }

        fn base_path(&self) -> String {
            self.dir.join("mtgfr").to_string_lossy().into_owned()
        }
    }

    impl Drop for TempConfig {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn load_from_applies_built_in_defaults_over_a_file_that_only_sets_database_url() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved = std::env::var("DATABASE_URL").ok();
        unsafe { std::env::remove_var("DATABASE_URL") };

        let cfg =
            TempConfig::write("database_url = \"postgresql://mtgfr:mtgfr@localhost:5432/mtgfr\"\n");
        let settings =
            Settings::load_from(&cfg.base_path()).expect("settings load from the temp config");

        assert_eq!(settings.host, "127.0.0.1");
        assert_eq!(settings.port, 8080);
        assert_eq!(
            settings.database_url,
            "postgresql://mtgfr:mtgfr@localhost:5432/mtgfr"
        );
        assert_eq!(settings.instance_id, "local");
        assert!(!settings.drain);
        assert!(!settings.cookie_secure);
        assert_eq!(settings.cookie_domain, "");
        assert_eq!(settings.cors_origin, "");
        assert_eq!(settings.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(settings.admin_token, "");
        assert_eq!(settings.listen_addr(), "127.0.0.1:8080");

        if let Some(v) = saved {
            unsafe { std::env::set_var("DATABASE_URL", v) };
        }
    }

    #[test]
    fn env_vars_override_the_file_and_the_built_in_defaults() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let cfg = TempConfig::write(
            "host = \"0.0.0.0\"\nport = 9999\ndatabase_url = \"postgresql://file\"\ndrain = false\n",
        );

        let saved: Vec<(&str, Option<String>)> = ["PORT", "DATABASE_URL", "COOKIE_SECURE"]
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect();
        unsafe {
            std::env::set_var("PORT", "7000");
            std::env::set_var("DATABASE_URL", "postgresql://env");
            std::env::set_var("COOKIE_SECURE", "true");
        }

        let settings =
            Settings::load_from(&cfg.base_path()).expect("settings load from the temp config");

        assert_eq!(settings.host, "0.0.0.0");
        assert_eq!(settings.port, 7000);
        assert_eq!(settings.database_url, "postgresql://env");
        assert!(settings.cookie_secure);

        unsafe {
            for (k, v) in saved {
                match v {
                    Some(v) => std::env::set_var(k, v),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    #[test]
    fn load_from_rejects_a_cors_origin_that_is_not_a_valid_header_value() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let cfg = TempConfig::write(
            "database_url = \"sqlite::memory:\"\ncors_origin = \"bad\\nvalue\"\n",
        );
        let err = Settings::load_from(&cfg.base_path()).expect_err("invalid cors_origin");
        assert!(matches!(err, config::ConfigError::Message(_)));
    }

    #[test]
    fn load_from_accepts_an_empty_or_valid_cors_origin() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let cfg = TempConfig::write(
            "database_url = \"sqlite::memory:\"\ncors_origin = \"https://edh.example.com\"\n",
        );
        let settings = Settings::load_from(&cfg.base_path()).expect("valid cors_origin");
        assert_eq!(settings.cors_origin, "https://edh.example.com");
    }
}
