use std::sync::Arc;

use deadpool_lapin::{Config, Manager, Pool, Runtime};
use deadpool_lapin::lapin::{
    options::BasicPublishOptions,
    BasicProperties,
};
use tokio_amqp::LapinTokioExt as _;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = Config::default();
    cfg.url = Some("amqp://admin:admin@127.0.0.1:5672/%2f".into());
    // let pool = cfg.create_pool(Runtime::Tokio1)?;
    let pool = cfg.create_pool();
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