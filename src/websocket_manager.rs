use actix::{Actor, Addr, AsyncContext, Handler, Message, StreamHandler}; //TODO: Check include orders
use actix_web_actors::ws;
use derivative::Derivative;
use lazy_static::lazy_static;
use mavlink::Message as MavMessage;
use regex::Regex;
use serde::Serialize;
use std::sync::{Arc, Mutex};

use crate::data::MAVLinkMessage;
use crate::legacy::{self, JsonSchema};

pub struct StringMessage(String);

impl Message for StringMessage {
    type Result = ();
}

#[derive(Serialize, Debug)]
pub struct WebsocketError {
    pub error: String,
}

#[derive(Debug)]
pub struct WebsocketActorContent {
    pub actor: Addr<WebsocketActor>,
    pub re: Option<Regex>,
    pub system_id: Option<u8>,
    pub component_id: Option<u8>,
    pub schema: JsonSchema,
}

#[derive(Derivative, Default)]
#[derivative(Debug)]
#[allow(clippy::type_complexity)]
pub struct WebsocketManager {
    pub clients: Vec<WebsocketActorContent>,
    #[derivative(Debug = "ignore")]
    pub new_message_callback: Option<Arc<dyn Fn(&String, JsonSchema) -> String + Send + Sync>>,
}

impl WebsocketManager {
    pub fn send(&self, value: &serde_json::Value, header: &mavlink::MavHeader, name: &str) {
        if self.clients.is_empty() {
            return;
        }

        let modern = serde_json::to_string_pretty(value).unwrap();
        let legacy = match legacy::modern_to_legacy(value) {
            Ok(legacy_value) => Some(serde_json::to_string_pretty(&legacy_value).unwrap()),
            Err(error) => {
                log::error!("Failed to convert websocket payload to legacy JSON: {error}");
                None
            }
        };

        for client in &self.clients {
            let is_match = client.re.as_ref().map_or(false, |regx| regx.is_match(name))
                && client.system_id.map_or(true, |id| id == header.system_id)
                && client
                    .component_id
                    .map_or(true, |id| id == header.component_id);
            if !is_match {
                continue;
            }
            let payload = match client.schema {
                JsonSchema::Modern => modern.clone(),
                JsonSchema::Legacy => match &legacy {
                    Some(content) => content.clone(),
                    None => serde_json::to_string(&WebsocketError {
                        error: "Failed to convert message to legacy JSON.".to_string(),
                    })
                    .unwrap(),
                },
            };
            client.actor.do_send(StringMessage(payload));
        }
    }
}

lazy_static! {
    static ref MANAGER: Arc<Mutex<WebsocketManager>> =
        Arc::new(Mutex::new(WebsocketManager::default()));
}

pub fn manager() -> Arc<Mutex<WebsocketManager>> {
    MANAGER.clone()
}

pub fn send(message: &MAVLinkMessage<mavlink::dialects::ardupilotmega::MavMessage>) {
    let name = message.message.message_name();
    let value = serde_json::to_value(message).unwrap();
    MANAGER.lock().unwrap().send(&value, &message.header, name);
}

#[derive(Debug)]
pub struct WebsocketActor {
    server: Arc<Mutex<WebsocketManager>>,
    pub filter: String,
    pub system_id: Option<u8>,
    pub component_id: Option<u8>,
    pub schema: JsonSchema,
}

impl WebsocketActor {
    pub fn new(
        message_filter: String,
        system_id: Option<u8>,
        component_id: Option<u8>,
        schema: JsonSchema,
    ) -> Self {
        Self {
            server: MANAGER.clone(),
            filter: message_filter,
            system_id,
            component_id,
            schema,
        }
    }
}

impl Handler<StringMessage> for WebsocketActor {
    type Result = ();

    fn handle(&mut self, message: StringMessage, context: &mut Self::Context) {
        context.text(message.0);
    }
}

impl Actor for WebsocketActor {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketActor {
    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Starting websocket, add itself in manager.");
        self.server
            .lock()
            .unwrap()
            .clients
            .push(WebsocketActorContent {
                actor: ctx.address(),
                re: Regex::new(&self.filter).ok(),
                system_id: self.system_id,
                component_id: self.component_id,
                schema: self.schema,
            });
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        println!("Finishing websocket, remove itself from manager.");
        self.server
            .lock()
            .unwrap()
            .clients
            .retain(|x| x.actor != ctx.address());
    }

    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                // Release the manager lock before the callback, it sends to the other clients
                let callback = self.server.lock().unwrap().new_message_callback.clone();
                let schema = self.schema;
                let text = match callback {
                    Some(callback) => callback(&text, schema),
                    None => serde_json::to_string(&WebsocketError {
                        error: "MAVLink callback does not exist.".to_string(),
                    })
                    .unwrap(),
                };
                ctx.text(text);
            }
            Ok(ws::Message::Close(msg)) => ctx.close(msg),
            _ => (),
        }
    }
}
