pub struct AppConfig {
    pub gateway_bearer_token: String,
    pub bridge_bearer_token: String,
    pub database_url: String,
    pub bridge_url: String,
    pub db_max_connections: u32,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let gateway_bearer_token = std::env::var("GATEWAY_BEARER_TOKEN")
            .unwrap_or_else(|_| panic!("missing env: GATEWAY_BEARER_TOKEN"));
        let bridge_bearer_token =
            std::env::var("BRIDGE_BEARER_TOKEN").unwrap_or_else(|_| gateway_bearer_token.clone());
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| panic!("missing env: DATABASE_URL"));
        let bridge_url =
            std::env::var("BRIDGE_URL").unwrap_or_else(|_| panic!("missing env: BRIDGE_URL"));
        let db_max_connections = std::env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(100);

        AppConfig {
            gateway_bearer_token,
            bridge_bearer_token,
            database_url,
            bridge_url,
            db_max_connections,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn lock_env() -> MutexGuard<'static, ()> {
        env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn test_from_env_all_set() {
        let _guard = lock_env();
        std::env::set_var("GATEWAY_BEARER_TOKEN", "test_token");
        std::env::set_var("BRIDGE_BEARER_TOKEN", "bridge_token");
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::set_var("BRIDGE_URL", "http://bridge");
        std::env::set_var("DB_MAX_CONNECTIONS", "200");

        let cfg = AppConfig::from_env();

        assert_eq!(cfg.gateway_bearer_token, "test_token");
        assert_eq!(cfg.bridge_bearer_token, "bridge_token");
        assert_eq!(cfg.database_url, "postgres://localhost/test");
        assert_eq!(cfg.bridge_url, "http://bridge");
        assert_eq!(cfg.db_max_connections, 200);
    }

    #[test]
    #[should_panic(expected = "missing env: DATABASE_URL")]
    fn test_from_env_missing_database_url() {
        let _guard = lock_env();
        std::env::set_var("GATEWAY_BEARER_TOKEN", "test_token");
        std::env::remove_var("BRIDGE_BEARER_TOKEN");
        std::env::remove_var("DATABASE_URL");
        std::env::set_var("BRIDGE_URL", "http://bridge");
        std::env::remove_var("DB_MAX_CONNECTIONS");

        AppConfig::from_env();
    }
}
