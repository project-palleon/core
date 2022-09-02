use std::time::SystemTime;

use serde::Serialize;

// struct to hold all data that is relevant to identify exactly one frame AND the frame itself
// i.e. the primary key is (timestamp, source) and the frame is data

#[derive(Clone, Serialize)]
pub struct Image {
    pub data: Vec<u8>,
    pub timestamp: SystemTime,
    pub source: String,
}

impl Image {
    pub(crate) fn new(data: Vec<u8>, source: String) -> Self {
        Image {
            data,
            timestamp: SystemTime::now(),
            source,
        }
    }
}
