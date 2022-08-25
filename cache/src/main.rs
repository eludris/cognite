use futures::TryStreamExt;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::Consumer;
use rdkafka::Message;
use redis::aio::Connection;
use redis::{cmd, AsyncCommands, Client};
use reqwest::header::HeaderMap;
use serde::Deserialize;
use std::env;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;

const API_URL: &str = "https://discord.com/api/v10";

#[derive(Debug, Deserialize)]
struct Payload {
    op: u16,
    t: String,
    d: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct Profile {
    id: String,
    username: String,
    discriminator: String,
    avatar: String,
}

async fn handle_payload(conn: Arc<Mutex<Connection>>, payload: String) {
    // TODO: Actually respect Discord API rules.
    // TODO: Add caching for other events.
    match serde_json::from_str::<Payload>(&payload) {
        Ok(payload) => {
            if payload.op == 0 {
                log::debug!("Handling \"{}\" payload: {:#?}", payload.t, payload.d);
                match payload.t.as_str() {
                    "GUILD_CREATE" => {
                        let mut conn = conn.lock().await;
                        conn.set::<&str, String, ()>(
                            &format!("guild:{}", payload.d["id"].as_str().unwrap()),
                            payload.d.to_string(),
                        )
                        .await
                        .unwrap();
                        if let Some(channels) = payload.d["channels"].as_array() {
                            for channel in channels {
                                let mut channel = channel.clone();
                                channel["guild_id"] = serde_json::json!(payload.d["id"]
                                    .as_str()
                                    .unwrap()
                                    .to_string());
                                conn.set::<&str, String, ()>(
                                    &format!("channel:{}", channel["id"].as_str().unwrap()),
                                    channel.to_string(),
                                )
                                .await
                                .unwrap();
                            }
                        }
                        if let Some(roles) = payload.d["roles"].as_array() {
                            for role in roles {
                                let mut role = role.clone();
                                role["guild_id"] = serde_json::json!(payload.d["id"]
                                    .as_str()
                                    .unwrap()
                                    .to_string());
                                conn.set::<&str, String, ()>(
                                    &format!("role:{}", role["id"].as_str().unwrap()),
                                    role.to_string(),
                                )
                                .await
                                .unwrap();
                            }
                        }
                    }
                    "MESSAGE_CREATE" => {
                        let mut conn = conn.lock().await;
                        if let Some(author) = payload.d.get("author") {
                            conn.set::<&str, String, ()>(
                                &format!("user:{}", author["id"].as_str().unwrap()),
                                author.to_string(),
                            )
                            .await
                            .unwrap();
                            if let Some(member) = payload.d.get("member") {
                                conn.set::<&str, String, ()>(
                                    &format!(
                                        "member:{}:{}",
                                        payload.d["guild_id"].as_str().unwrap(),
                                        author["id"].as_str().unwrap()
                                    ),
                                    member.to_string(),
                                )
                                .await
                                .unwrap();
                            }
                        }
                        if let Some(mentions) = payload.d.get("mentions") {
                            for mention in mentions.as_array().unwrap() {
                                if let Some(member) = mention.get("member") {
                                    conn.set::<&str, String, ()>(
                                        &format!(
                                            "member:{}:{}",
                                            payload.d["guild_id"].as_str().unwrap(),
                                            mention["id"].as_str().unwrap()
                                        ),
                                        member.to_string(),
                                    )
                                    .await
                                    .unwrap();
                                }
                                let mut mention = mention.clone();
                                let mention = mention.as_object_mut().unwrap();
                                mention.remove("member");
                                conn.set::<&str, String, ()>(
                                    &format!("user:{}", mention["id"].as_str().unwrap()),
                                    serde_json::to_string(&mention).unwrap(),
                                )
                                .await
                                .unwrap();
                            }
                        }
                    }
                    _ => {
                        log::info!("Unhandled event: {}", payload.t);
                    }
                }
            }
        }
        Err(err) => log::info!("Error parsing payload: {}: {}", err, payload),
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let keydb_url =
        std::env::var("KEYDB_URL").unwrap_or_else(|_| "redis://localhost:6379/1".to_string());
    let topic = env::var("KAFKA_TOPIC").expect("KAFKA_TOPIC must be set");
    let broker = env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:9092".to_string());
    let group = env::var("KAFKA_GROUP").unwrap_or_else(|_| "cognite-cache".to_string());
    let token = env::var("TOKEN").expect("TOKEN must be set");
    let init_script = env::var("INIT_SCRIPT").ok();

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("group.id", &group)
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("consumer creation failed");

    consumer.subscribe(&[&topic]).expect("subscription failed");

    let client = Client::open(keydb_url).expect("failed to connect to keydb");
    let conn = client
        .get_async_connection()
        .await
        .expect("Couldn't connect to KeyDB");
    let conn = Arc::new(Mutex::new(conn));

    let mut headers = HeaderMap::new();
    headers.insert("Authorization", format!("Bot {}", token).parse().unwrap());

    let profile: Profile = reqwest::Client::new()
        .get(&format!("{}/users/@me", API_URL))
        .headers(headers)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    {
        let mut conn = conn.lock().await;
        conn.set::<&str, String, ()>("bot:id", profile.id.to_string())
            .await
            .unwrap();
        conn.set::<&str, String, ()>("bot:username", profile.username.to_string())
            .await
            .unwrap();
        conn.set::<&str, String, ()>("bot:discriminator", profile.discriminator.to_string())
            .await
            .unwrap();
        conn.set::<&str, String, ()>("bot:avatar", profile.avatar.to_string())
            .await
            .unwrap();
    }

    if let Some(init_script) = init_script {
        let script = fs::read_to_string(init_script).unwrap();
        cmd("EVAL")
            .arg(&script)
            .arg(0)
            .query_async::<Connection, ()>(&mut *conn.lock().await)
            .await
            .unwrap();
    }

    consumer
        .stream()
        .try_for_each(|message| {
            let conn = Arc::clone(&conn);
            async move {
                match message.payload_view::<str>() {
                    Some(Ok(payload)) => {
                        tokio::spawn(handle_payload(conn, payload.to_string()));
                    }
                    _ => log::error!("Could not get payload"),
                }
                Ok(())
            }
        })
        .await
        .expect("stream failed");

    consumer.unsubscribe();
}
