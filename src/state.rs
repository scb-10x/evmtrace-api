use anyhow::Result;
use deadpool_postgres::{Pool as PostgresPool, Runtime};
use once_cell::sync::Lazy;
use redis::Client as RedisClient;
use redis_pool::{RedisPool, SingleRedisPool};
use tokio_postgres::NoTls;

use crate::config::CONFIG;

pub static STATE: Lazy<State> = Lazy::new(|| State::new().expect("Failed to create state"));

#[derive(Clone)]
pub struct State {
    pub postgres_pool: PostgresPool,
    pub redis_pool: SingleRedisPool,
}

impl State {
    pub fn new() -> Result<Self> {
        Ok(Self {
            postgres_pool: CONFIG
                .postgres_config()
                .create_pool(Some(Runtime::Tokio1), NoTls)?,
            redis_pool: RedisPool::from(RedisClient::open(CONFIG.redis.as_str())?),
        })
    }
}
