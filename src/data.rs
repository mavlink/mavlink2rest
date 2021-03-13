use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mavlink::{self, Message};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct MAVLinkMessage {
    header: mavlink::MavHeader,
    message: mavlink::ardupilotmega::MavMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MAVLinkVehicleComponentData {
    id: u8,
    messages: HashMap<String, mavlink::ardupilotmega::MavMessage>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MAVLinkVehicleData {
    id: u8,
    components: Vec<MAVLinkVehicleComponentData>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MAVLinkVehiclesData {
    vehicles: Vec<MAVLinkVehicleData>,
}

impl MAVLinkVehiclesData {
    fn update(&mut self, message: MAVLinkMessage) {
        if let Some(vehicle) = self
            .vehicles
            .iter_mut()
            .find(|vehicle| vehicle.id == message.header.system_id)
        {
            if let Some(component) = vehicle
                .components
                .iter_mut()
                .find(|component| component.id == message.header.component_id)
            {
                component
                    .messages
                    .insert(message.message.message_name().into(), message.message);
                return;
            }

            let mut messages = HashMap::new();
            messages.insert(message.message.message_name().into(), message.message);
            vehicle.components.push(MAVLinkVehicleComponentData {
                id: message.header.component_id,
                messages,
            });
            return;
        }

        self.vehicles.push(MAVLinkVehicleData {
            id: message.header.system_id,
            components: vec![],
        });
        self.update(message);
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

    let messages = DATA.messages.lock().unwrap();
    //println!(">{} {:#?}", messages.len(), messages);
}

pub fn messages() -> MAVLinkVehiclesData {
    let messages = DATA.messages.lock().unwrap();
    return messages.clone();
}
