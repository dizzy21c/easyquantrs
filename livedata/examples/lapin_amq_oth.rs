use std::sync::Arc;

use deadpool_lapin::Runtime;
use deadpool_lapin::lapin::{
    options::BasicPublishOptions,
    BasicProperties,
};
use dotenv::dotenv;
// # use serde_1 as serde;

#[derive(Debug, serde::Deserialize)]
// # #[serde(crate = "serde_1")]
struct Config {
    #[serde(default)]
    amqp: deadpool_lapin::Config
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut cfg = config::Config::new();
        cfg.merge(config::Environment::new().separator("__"))?;
        cfg.try_into()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let mut cfg = Config::from_env().unwrap();
    // let pool = cfg.amqp.create_pool(Runtime::Tokio1).unwrap();
    let pool = cfg.amqp.create_pool();
    for _ in 1..10 {
        let mut connection = pool.get().await?;
        let channel = connection.create_channel().await?;
        channel.basic_publish(
            "",
            "hello",
            BasicPublishOptions::default(),
            b"hello from deadpool".to_vec(),
            BasicProperties::default(),
        ).await?;
    }
    Ok(())
}