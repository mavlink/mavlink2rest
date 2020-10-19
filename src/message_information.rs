use serde::Serialize;

#[derive(Serialize, Debug, Copy, Clone)]
struct Time {
    first_message: chrono::DateTime<chrono::Local>,
    last_message: chrono::DateTime<chrono::Local>,
}

#[derive(Serialize, Debug, Copy, Clone)]
pub struct MessageInformation {
    counter: u32,
    frequency: f32,
    time: Time,
}

impl Default for MessageInformation {
    fn default() -> MessageInformation {
        MessageInformation {
            counter: 0,
            frequency: 0.0,
            time: Time {
                first_message: chrono::Local::now(),
                last_message: chrono::Local::now(),
            },
        }
    }
}

impl MessageInformation {
    pub fn update(&mut self) {
        self.counter += 1;
        self.time.last_message = chrono::Local::now();
        self.frequency = (self.counter as f32)
            / ((self.time.last_message - self.time.first_message).num_seconds() as f32);
    }
}
