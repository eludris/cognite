use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

/// A simple abstraction for a one-topic and one-key producer.
pub struct Producer {
    producer: FutureProducer,
    topic: String,
    key: String,
}

impl Producer {
    pub fn new(broker: String, topic: String, key: String, timeout: String) -> Producer {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &broker)
            .set("message.timeout.ms", &timeout)
            .create()
            .expect("producer creation failed");
        Producer {
            producer,
            topic,
            key,
        }
    }

    pub async fn send(&self, message: &str) {
        let record = FutureRecord::to(&self.topic)
            .key(&self.key)
            .payload(message);
        match self.producer.send(record, Duration::from_secs(0)).await {
            Ok(delivery) => log::debug!("Delivered message to {:?}", delivery),
            Err(err) => log::warn!("Failed to deliver message: {:?}", err),
        }
    }
}
