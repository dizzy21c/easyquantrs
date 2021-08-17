use std::thread;

use amiquip::Delivery;
use amiquip::{
    Connection, ConsumerMessage, ConsumerOptions, Exchange, ExchangeDeclareOptions, ExchangeType,
    FieldTable, Publish, QueueDeclareOptions, Result,
};
use crossbeam_channel::bounded;
use crossbeam_channel::{Receiver, Sender};

pub struct QAEventMQ {
    pub amqp: String,
    pub exchange: String,
    pub model: String,
    pub routing_key: String,
    // connection:
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
}
