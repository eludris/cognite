use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use cognite_gateway::Producer;

const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

async fn handle_payload(
    tx: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    producer: Arc<Producer>,
    payload: String,
) {
    producer.send(&payload).await;

    let payload: HashMap<String, serde_json::Value> = serde_json::from_str(&payload).unwrap();

    match payload.get("op") {
        Some(op) => {
            let op = op.as_u64().unwrap();
            match op {
                0 => {}
                10 => {
                    let heartbeat = payload
                        .get("d")
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .get("heartbeat_interval")
                        .unwrap()
                        .as_u64()
                        .unwrap();
                    loop {
                        let payload = serde_json::json!({
                            "op": 1,
                            "d": Option::<u32>::None
                        });
                        log::debug!("Sending heartbeat: {:?}", payload);
                        tx.lock()
                            .await
                            .send(Message::Text(payload.to_string()))
                            .await
                            .unwrap();
                        time::sleep(time::Duration::from_millis(heartbeat)).await;
                    }
                }
                11 => {}
                _ => {
                    log::debug!("Unhandled OP code: {}", op);
                }
            }
        }
        None => log::error!("Could not find OP code in payload"),
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let topic = env::var("KAFKA_TOPIC").expect("KAFKA_TOPIC must be set");
    let broker = env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:9092".to_string());
    let key = env::var("KAFKA_KEY").unwrap_or_else(|_| "".to_string());
    let timeout = env::var("KAFKA_TIMEOUT").unwrap_or_else(|_| "5000".to_string());
    let token = env::var("TOKEN").expect("TOKEN must be set");
    let intents = env::var("INTENTS")
        .unwrap_or_else(|_| "0".to_string())
        .parse::<u64>()
        .unwrap();

    let producer = Arc::new(Producer::new(broker, topic, key, timeout));

    let (socket, _) = connect_async(GATEWAY_URL).await.unwrap();
    let (tx, mut rx) = socket.split();

    let tx = Arc::new(Mutex::new(tx));

    tx.lock()
        .await
        .send(Message::Text(
            serde_json::json!({
                "op": 2_u8,
                "d": {
                    "token": token,
                    "intents": intents,
                    "properties": {
                        "$os": "linux",
                        "$device": "cognite-gateway",
                        "$platform": "cognite-gateway",
                    }
                }
            })
            .to_string(),
        ))
        .await
        .expect("Failed sending authorization payload");

    while let Some(msg) = rx.next().await {
        if let Ok(Message::Text(msg)) = msg {
            tokio::spawn(handle_payload(Arc::clone(&tx), Arc::clone(&producer), msg));
        }
    }
}
