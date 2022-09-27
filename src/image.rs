use std::time::SystemTime;

use bson::{Bson, doc, Document};
use bson::spec::BinarySubtype;
use serde::Serialize;

// struct to hold all data that is relevant to identify exactly one frame AND the frame itself
// i.e. the primary key is (timestamp, source) and the frame is data

#[derive(Clone, Serialize)]
pub struct Image {
    pub data: Vec<u8>,
    pub timestamp: SystemTime,
    pub input_source: String,
}

impl Image {
    pub fn new(data: Vec<u8>, input_source: String) -> Self {
        Image {
            data,
            timestamp: SystemTime::now(),
            input_source,
        }
    }
}

impl From<Image> for Document {
    fn from(image: Image) -> Self {
        doc! {
            "data": Bson::Binary(bson::Binary {
                subtype: BinarySubtype::Generic,
                bytes: image.data,
            }),
            "input_source": image.input_source,
            "timestamp": Bson::DateTime(bson::DateTime::from_system_time(image.timestamp)),
        }
    }
}
