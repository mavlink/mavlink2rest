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
    components: HashMap<u8, MAVLinkVehicleComponentData>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MAVLinkVehiclesData {
    vehicles: HashMap<u8, MAVLinkVehicleData>,
}

impl MAVLinkVehiclesData {
    fn update(&mut self, message: MAVLinkMessage) {
        // If vehicle does not exist for us, adds it
        let vehicle_id = message.header.system_id;
        if !self.vehicles.contains_key(&vehicle_id) {
            self.vehicles.insert(
                message.header.system_id,
                MAVLinkVehicleData {
                    id: message.header.system_id,
                    components: HashMap::new(),
                },
            );
        }

        // If component does not exist for vehicle, adds it
        let component_id = message.header.component_id;
        if !self.vehicles[&vehicle_id]
            .components
            .contains_key(&component_id)
        {
            self.vehicles
                .get_mut(&vehicle_id)
                .unwrap()
                .components
                .insert(
                    component_id,
                    MAVLinkVehicleComponentData {
                        id: message.header.component_id,
                        messages: HashMap::new(),
                    },
                );
        }

        // Add new message for vehicle/component
        self.vehicles
            .get_mut(&vehicle_id)
            .unwrap()
            .components
            .get_mut(&component_id)
            .unwrap()
            .messages
            .insert(message.message.message_name().into(), message.message);
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
