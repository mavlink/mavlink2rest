use actix::{Actor, Addr, AsyncContext, Handler, Message, StreamHandler}; //TODO: Check include orders
use actix_web_actors::ws;
use derivative::Derivative;
use lazy_static::lazy_static;
use mavlink::Message as MavMessage;
use regex::Regex;
use serde::Serialize;
use std::sync::{Arc, Mutex};

use crate::MAVLinkMessage;

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
}

#[derive(Derivative, Default)]
#[derivative(Debug)]
#[allow(clippy::type_complexity)]
pub struct WebsocketManager {
    pub clients: Vec<WebsocketActorContent>,
    #[derivative(Debug = "ignore")]
    pub new_message_callback: Option<Arc<dyn Fn(&String) -> String + Send + Sync>>,
}

impl WebsocketManager {
    pub fn send(&self, value: &serde_json::Value, name: &str) {
        if self.clients.is_empty() {
            return;
        }

        let string = serde_json::to_string_pretty(value).unwrap();
        for client in &self.clients {
            let is_match = client.re.as_ref().map_or(false, |regx| regx.is_match(name));
            if is_match {
                client.actor.do_send(StringMessage(string.clone()));
            }
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

pub fn send(message: &MAVLinkMessage<mavlink::ardupilotmega::MavMessage>) {
    let name = message.message.message_name();
    let value = serde_json::to_value(message).unwrap();
    MANAGER.lock().unwrap().send(&value, name);
}

#[derive(Debug)]
pub struct WebsocketActor {
    server: Arc<Mutex<WebsocketManager>>,
    pub filter: String,
}

impl WebsocketActor {
    pub fn new(message_filter: String) -> Self {
        Self {
            server: MANAGER.clone(),
            filter: message_filter,
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
                let text = match &self.server.lock().unwrap().new_message_callback {
                    Some(callback) => callback(&text),
                    None => serde_json::to_string(&WebsocketError {
                        error: "MAVLink callback does not exist.".to_string(),
                    })
                    .unwrap(),
                };
                ctx.text(text);
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}
