use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mavlink::{self, Message};
use serde::{Deserialize, Serialize};

//TODO: break all this types to a new file
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Temporal {
    first_update: chrono::DateTime<chrono::Local>,
    last_update: chrono::DateTime<chrono::Local>,
    counter: i64,
    frequency: f32,
}

impl Default for Temporal {
    fn default() -> Self {
        Self {
            first_update: chrono::Local::now(),
            last_update: chrono::Local::now(),
            counter: 1,
            frequency: 0.0,
        }
    }
}

impl Temporal {
    fn update(&mut self) {
        self.last_update = chrono::Local::now();
        self.counter = self.counter.wrapping_add(1);
        self.frequency =
            (self.counter as f32) / ((self.last_update - self.first_update).num_seconds() as f32);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Status {
    time: Temporal,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            time: Temporal::default(),
        }
    }
}

impl Status {
    fn update(&mut self) -> &mut Self {
        self.time.update();
        return self;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MAVLinkMessage<T> {
    pub header: mavlink::MavHeader,
    pub message: T,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MAVLinkMessageStatus {
    message: mavlink::ardupilotmega::MavMessage,
    status: Status,
}

impl MAVLinkMessageStatus {
    fn update(&mut self, message: &MAVLinkMessage<mavlink::ardupilotmega::MavMessage>) {
        self.message = message.message.clone();
        self.status.update();
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MAVLinkVehicleComponentData {
    id: u8,
    messages: HashMap<String, MAVLinkMessageStatus>,
}

impl MAVLinkVehicleComponentData {
    fn update(&mut self, message: &MAVLinkMessage<mavlink::ardupilotmega::MavMessage>) {
        // If message does not exist, add it
        let message_name = message.message.message_name().into();
        if !self.messages.contains_key(&message_name) {
            self.messages.insert(
                message_name,
                MAVLinkMessageStatus {
                    message: message.message.clone(),
                    status: Status::default(),
                },
            );
            return;
        }

        self.messages
            .get_mut(&message_name)
            .unwrap()
            .update(&message);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MAVLinkVehicleData {
    id: u8,
    components: HashMap<u8, MAVLinkVehicleComponentData>,
}

impl MAVLinkVehicleData {
    fn update(&mut self, message: &MAVLinkMessage<mavlink::ardupilotmega::MavMessage>) {
        // If component does not exist, adds it
        let component_id = message.header.component_id;
        if !self.components.contains_key(&component_id) {
            self.components.insert(
                component_id,
                MAVLinkVehicleComponentData {
                    id: component_id,
                    messages: HashMap::new(),
                },
            );
        }

        self.components
            .get_mut(&component_id)
            .unwrap()
            .update(&message);
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MAVLinkVehiclesData {
    vehicles: HashMap<u8, MAVLinkVehicleData>,
}

impl MAVLinkVehiclesData {
    //TODO: Move message to reference
    fn update(&mut self, message: MAVLinkMessage<mavlink::ardupilotmega::MavMessage>) {
        // If vehicle does not exist for us, adds it
        let vehicle_id = message.header.system_id;
        if !self.vehicles.contains_key(&vehicle_id) {
            self.vehicles.insert(
                vehicle_id,
                MAVLinkVehicleData {
                    id: vehicle_id,
                    components: HashMap::new(),
                },
            );
        }

        self.vehicles.get_mut(&vehicle_id).unwrap().update(&message);
    }

    pub fn pointer(&self, path: &str) -> String {
        let path = format!("/{}", path);
        if path == "/" {
            return serde_json::to_string_pretty(self).unwrap();
        };

        if path == "/vehicles" {
            return serde_json::to_string_pretty(&self.vehicles).unwrap();
        };

        let value = serde_json::to_value(self).unwrap();
        return match value.pointer(&path) {
            Some(content) => serde_json::to_string_pretty(content).unwrap(),
            None => "None".into(),
        };
    }
}

#[derive(Debug)]
struct Data {
    messages: Arc<Mutex<MAVLinkVehiclesData>>,
}

lazy_static! {
    static ref DATA: Data = Data {
        messages: Arc::new(Mutex::new(MAVLinkVehiclesData::default())),
    };
}

pub fn update((header, message): (mavlink::MavHeader, mavlink::ardupilotmega::MavMessage)) {
    DATA.messages
        .lock()
        .unwrap()
        .update(MAVLinkMessage { header, message });

    //let messages = DATA.messages.lock().unwrap();
    //println!(">{} {:#?}", messages.len(), messages);
}

pub fn messages() -> MAVLinkVehiclesData {
    let messages = DATA.messages.lock().unwrap();
    return messages.clone();
}
