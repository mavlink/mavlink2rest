use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use mavlink::{self, Message};
use serde::{Deserialize, Serialize};

//TODO: break all this types to a new file
#[derive(Clone, Debug, Deserialize)]
struct Temporal {
    first_update: chrono::DateTime<chrono::Local>,
    last_update: chrono::DateTime<chrono::Local>,
    counter: i64,
    frequency: f32,
}

impl Serialize for Temporal {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        // Computed when the request is served, so the client does not need to parse dates
        let since_last_update =
            (chrono::Local::now() - self.last_update).num_milliseconds() as f64 / 1000.0;
        let mut state = serializer.serialize_struct("Temporal", 5)?;
        state.serialize_field("first_update", &self.first_update)?;
        state.serialize_field("last_update", &self.last_update)?;
        state.serialize_field("last_update_seconds_ago", &since_last_update)?;
        state.serialize_field("counter", &self.counter)?;
        state.serialize_field("frequency", &self.frequency)?;
        state.end()
    }
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

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
struct Status {
    time: Temporal,
}

impl Status {
    fn update(&mut self) -> &mut Self {
        self.time.update();
        self
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MAVLinkMessage<T> {
    #[serde(with = "MavHeaderDef")]
    pub header: mavlink::MavHeader,
    pub message: T,
}

// Mirror of mavlink::MavHeader to make sequence optional on input
#[derive(Deserialize, Serialize)]
#[serde(remote = "mavlink::MavHeader")]
struct MavHeaderDef {
    system_id: u8,
    component_id: u8,
    #[serde(default)]
    sequence: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MAVLinkMessageStatus {
    message: mavlink::dialects::ardupilotmega::MavMessage,
    status: Status,
}

impl MAVLinkMessageStatus {
    fn update(&mut self, message: &MAVLinkMessage<mavlink::dialects::ardupilotmega::MavMessage>) {
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
    fn update(&mut self, message: &MAVLinkMessage<mavlink::dialects::ardupilotmega::MavMessage>) {
        let message_name = message.message.message_name().to_string();
        self.messages
            .entry(message_name)
            .or_insert(MAVLinkMessageStatus {
                message: message.message.clone(),
                status: Status::default(),
            })
            .update(message);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MAVLinkVehicleData {
    id: u8,
    components: HashMap<u8, MAVLinkVehicleComponentData>,
}

impl MAVLinkVehicleData {
    fn update(&mut self, message: &MAVLinkMessage<mavlink::dialects::ardupilotmega::MavMessage>) {
        let component_id = message.header.component_id;
        self.components
            .entry(component_id)
            .or_insert(MAVLinkVehicleComponentData {
                id: component_id,
                messages: HashMap::new(),
            })
            .update(message);
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MAVLinkVehiclesData {
    vehicles: HashMap<u8, MAVLinkVehicleData>,
}

impl MAVLinkVehiclesData {
    //TODO: Move message to reference
    fn update(&mut self, message: MAVLinkMessage<mavlink::dialects::ardupilotmega::MavMessage>) {
        let vehicle_id = message.header.system_id;
        self.vehicles
            .entry(vehicle_id)
            .or_insert(MAVLinkVehicleData {
                id: vehicle_id,
                components: HashMap::new(),
            })
            .update(&message);
    }

    pub fn pointer_with_schema(
        &self,
        path: &str,
        schema: crate::legacy::JsonSchema,
    ) -> Result<String, String> {
        let value = serde_json::to_value(self).map_err(|error| error.to_string())?;
        let value = match schema {
            crate::legacy::JsonSchema::Modern => value,
            crate::legacy::JsonSchema::Legacy => crate::legacy::modern_to_legacy(&value)?,
        };

        if path.is_empty() {
            return serde_json::to_string_pretty(&value).map_err(|error| error.to_string());
        }

        let path = format!("/{path}");
        dbg!(&path);

        if path == "/vehicles" {
            let vehicles = value.get("vehicles").ok_or_else(|| "None".to_string())?;
            return serde_json::to_string_pretty(vehicles).map_err(|error| error.to_string());
        }

        match value.pointer(&path) {
            Some(content) => {
                serde_json::to_string_pretty(content).map_err(|error| error.to_string())
            }
            None => Ok("None".into()),
        }
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

pub fn update(
    (header, message): (
        mavlink::MavHeader,
        mavlink::dialects::ardupilotmega::MavMessage,
    ),
) {
    DATA.messages
        .lock()
        .unwrap()
        .update(MAVLinkMessage { header, message });
}

pub fn messages() -> MAVLinkVehiclesData {
    let messages = DATA.messages.lock().unwrap();
    messages.clone()
}
