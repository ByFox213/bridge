pub mod handlers;

use async_nats::Client;
use async_nats::jetstream::Context;
use futures::StreamExt;
use log::{debug, info};
use crate::handler::handlers::chat_handler;
use crate::model::{EnvHandler, MsgBridge};
use crate::util::patterns::DD_PATTERNS;

pub async fn main(env: EnvHandler, nats: Client, jetstream: Context) -> Result<(), async_nats::Error> {
    let mut subscriber = nats.queue_subscribe("tw.econ.read.*", "handler".to_string()).await?;

    info!("Handler started");
    while let Some(message) = subscriber.next().await {
        debug!("message received from {}, length {}", message.subject, message.length);
        let msg: MsgBridge = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => serde_json::from_str(json_string).unwrap_or_else(|err| {
                panic!("Error deserializing JSON: {}", err);
            }),
            Err(err) => {
                panic!("Error converting bytes to string: {}", err);
            }
        };
        let message_thread_id = msg.message_thread_id.clone();
        for pattern in DD_PATTERNS.iter() {
            if !pattern.regex.is_match(&msg.text) {
                continue;
            }

            let text = msg.text.clone();
            let caps = pattern.regex.captures(&text).unwrap();

            let json = chat_handler(msg, &env, caps, pattern).await;

            if json.is_empty() {
                break
            }

            debug!("sent json to tw.tg.(id): {}", json);

            jetstream.publish("tw.tg.".to_owned() + message_thread_id.as_ref(), json.into())
                .await
                .expect("Error publish message to tw.messages");
            break
        }
    }


    Ok(())
}