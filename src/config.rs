use deadpool_postgres::{Config as PostgresConfig, ManagerConfig, RecyclingMethod};
use dotenvy::var;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use structstruck::strike;

pub static CONFIG: Lazy<Config> = Lazy::new(Config::new);

strike! {
    #[strikethrough[derive(Debug, Clone, Serialize, Deserialize, Default)]]
    pub struct Config {
        pub postgres:
            pub struct {
                pub host: String,
                pub username: String,
                pub password: String,
                pub db: String,
            }
        ,
        pub redis: String,
        pub port: u16,
        pub is_dev: bool,
    }
}

impl Config {
    pub fn new() -> Self {
        Config {
            postgres: Postgres {
                host: var("POSTGRES_HOST").expect("POSTGRES_HOST must be set"),
                username: var("POSTGRES_USERNAME").expect("POSTGRES_USERNAME must be set"),
                password: var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be set"),
                db: var("POSTGRES_DB").expect("POSTGRES_DB must be set"),
            },
            redis: var("REDIS_URL").expect("REDIS_URL must be set"),
            port: var("PORT")
                .unwrap_or("8080".to_string())
                .parse()
                .expect("PORT must be a number"),
            is_dev: var("MODE").map(|m| m == "dev").unwrap_or_default(),
        }
    }

    pub fn postgres_config(&self) -> PostgresConfig {
        self.into()
    }
}

impl From<&Config> for PostgresConfig {
    fn from(val: &Config) -> Self {
        PostgresConfig {
            host: Some(val.postgres.host.to_string()),
            user: Some(val.postgres.username.to_string()),
            password: Some(val.postgres.password.to_string()),
            dbname: Some(val.postgres.db.to_string()),
            manager: Some(ManagerConfig {
                recycling_method: RecyclingMethod::Fast,
            }),
            ..Default::default()
        }
    }
}
