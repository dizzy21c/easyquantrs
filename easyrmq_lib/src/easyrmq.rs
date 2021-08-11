use deadpool_lapin::{Manager, PoolError};
use futures::{join, StreamExt};
use lapin::{options::*, types::FieldTable, BasicProperties, ConnectionProperties};
use std::convert::Infallible;
use std::result::Result as StdResult;
use std::time::Duration;
use thiserror::Error as ThisError;
use tokio_amqp::*;

type Result<T> = StdResult<T, Error>;

type Pool = deadpool::managed::Pool<lapin::Connection, lapin::Error>;
type Connection = deadpool::managed::Object<lapin::Connection, lapin::Error>;

pub struct QAEventMQ {
    pub amqp: String,
    pub exchange: String,
    pub model: String,
    pub routing_key: String,
    // connection:
    pub pool: Pool,
}

#[derive(ThisError, Debug)]
enum Error {
    #[error("rmq error: {0}")]
    RMQError(#[from] lapin::Error),
    #[error("rmq pool error: {0}")]
    RMQPoolError(#[from] PoolError),
}

impl QAEventMQ {
    pub fn consume(eventmq: QAEventMQ, ws_event_tx: Sender<String>) -> Result<()> {
        let client = eventmq;
        let mut connection = Connection::insecure_open(&client.amqp)?;
        let channel = connection.open_channel(None)?;
        let exchange = channel.exchange_declare(
            ExchangeType::Direct,
            &client.exchange,
            ExchangeDeclareOptions::default(),
        )?;
        let queue = channel.queue_declare(
            "",
            QueueDeclareOptions {
                exclusive: true,
                ..QueueDeclareOptions::default()
            },
        )?;
        println!("created exclusive queue {}", queue.name());

        queue.bind(&exchange, client.routing_key.clone(), FieldTable::new())?;

        let consumer = queue.consume(ConsumerOptions {
            no_ack: true,
            ..ConsumerOptions::default()
        })?;

        for (_i, message) in consumer.receiver().iter().enumerate() {
            match message {
                ConsumerMessage::Delivery(delivery) => {
                    let body = String::from_utf8_lossy(&delivery.body);
                    QAEventMQ::callback(&client, &delivery, &ws_event_tx);
                }
                other => {
                    println!("Consumer ended: {:?}", other);
                    break;
                }
            }
        }
        connection.close()
    }

    pub fn publish_fanout(
        amqp: String,
        exchange_name: String,
        context: String,
        routing_key: String,
    ) {
        let mut connection = Connection::insecure_open(&amqp).unwrap();
        let channel = connection.open_channel(None).unwrap();
        let exchange = channel
            .exchange_declare(
                ExchangeType::Fanout,
                &exchange_name,
                ExchangeDeclareOptions::default(),
            )
            .unwrap();

        exchange
            .publish(Publish::new(context.as_bytes(), routing_key.as_str()))
            .unwrap();
        //connection.close();
    }
    pub fn publish_direct(
        amqp: String,
        exchange_name: String,
        context: String,
        routing_key: String,
    ) {
        let mut connection = Connection::insecure_open(&amqp).unwrap();
        let channel = connection.open_channel(None).unwrap();
        let exchange = channel
            .exchange_declare(
                ExchangeType::Direct,
                &exchange_name,
                ExchangeDeclareOptions::default(),
            )
            .unwrap();

        exchange
            .publish(Publish::new(context.as_bytes(), routing_key.as_str()))
            .unwrap();
        //connection.close();
    }
    pub fn callback(eventmq: &QAEventMQ, message: &Delivery, ws_event_tx: &Sender<String>) {
        let msg = message.body.clone();
        let foo = String::from_utf8(msg).unwrap();
        let data = foo.to_string();

        //println!("{:?}",data);
        ws_event_tx.send(data).unwrap();
    }

    pub fn init_main(eventmq: QAEventMQ) -> Result<()> {
        let addr =
            std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://admin:admin@127.0.0.1:5672/%2f".into());
        let manager = Manager::new(addr, ConnectionProperties::default().with_tokio());
        eventmq.pool = deadpool::managed::Pool::new(manager, 10);

        rmq_listen(pool.clone());

        Ok(())
    }

    pub async fn publish_fanout(pool: Pool, exchange_name: String,
                            context: String,
                            routing_key: String
    ) -> StdResult<()> {
        let payload = context;

        let rmq_con = get_rmq_con(pool).await.map_err(|e| {
            eprintln!("can't connect to rmq, {}", e);
            warp::reject::custom(Error::RMQPoolError(e))
        })?;

        let channel = rmq_con.create_channel().await.map_err(|e| {
            eprintln!("can't create channel, {}", e);
            warp::reject::custom(Error::RMQError(e))
        })?;

        channel
            .basic_publish(
                "",
                "hello",
                BasicPublishOptions::default(),
                payload.to_vec(),
                BasicProperties::default(),
            )
            .await
            .map_err(|e| {
                eprintln!("can't publish: {}", e);
                warp::reject::custom(Error::RMQError(e))
            })?
            .await
            .map_err(|e| {
                eprintln!("can't publish: {}", e);
                warp::reject::custom(Error::RMQError(e))
            })?;
        Ok("OK")
    }

    pub async fn get_rmq_con(pool: Pool) -> StdResult<Connection> {
        let connection = pool.get().await?;
        Ok(connection)
    }

    pub fn rmq_listen(pool: Pool) -> Result<()> {
        let mut retry_interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            retry_interval.tick().await;
            println!("connecting rmq consumer...");
            match init_rmq_listen(pool.clone()).await {
                Ok(_) => println!("rmq listen returned"),
                Err(e) => eprintln!("rmq listen had an error: {}", e),
            };
        }
    }

    pub async fn init_rmq_listen(pool: Pool) -> Result<()> {
        let rmq_con = get_rmq_con(pool).await.map_err(|e| {
            eprintln!("could not get rmq con: {}", e);
            e
        })?;
        let channel = rmq_con.create_channel().await?;

        let queue = channel
            .queue_declare(
                "hello",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;
        println!("Declared queue {:?}", queue);

        let mut consumer = channel
            .basic_consume(
                "hello",
                "my_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        println!("rmq consumer connected, waiting for messages");
        while let Some(delivery) = consumer.next().await {
            if let Ok((channel, delivery)) = delivery {
                println!("received msg: {:?}", delivery);
                channel
                    .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                    .await?
            }
        }
        Ok(())
    }
}