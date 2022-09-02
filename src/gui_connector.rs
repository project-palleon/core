use std::{io, thread};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use bson::{Bson, doc};
use bson::spec::BinarySubtype;
use crossbeam_channel::{bounded, Receiver, Sender};

use crate::Config;
use crate::image::Image;

pub static GUI_HANDLER_RUNNING: AtomicBool = AtomicBool::new(false);

fn handle_stream(mut stream: TcpStream, image_rx: &Receiver<Image>, data_rx: &Receiver<(String, SystemTime, Bson)>, control_tx: &Sender<(String, String)>) -> Result<bool, io::Error> {
    loop {
        // collect all images in the queue
        let images = {
            let mut images = vec![];
            loop {
                let image = image_rx.recv_timeout(Duration::from_millis(5));
                if image.is_err() { break; }
                let image = image.unwrap();
                images.push(Bson::Array(vec![
                    Bson::DateTime(bson::DateTime::from_system_time(image.timestamp)),
                    Bson::String(image.source),
                    Bson::Binary(bson::Binary { subtype: BinarySubtype::Generic, bytes: image.data }),
                ]));
            }
            Bson::Array(images)
        };

        // collect all responses from the database
        let data = {
            let mut data = vec![];
            loop {
                let datum = data_rx.recv_timeout(Duration::from_millis(5));
                if datum.is_err() { break; }
                let datum = datum.unwrap();
                data.push(Bson::Array(vec![
                    Bson::String(datum.0),
                    Bson::DateTime(bson::DateTime::from_system_time(datum.1)),
                    datum.2,
                ]));
            }
            Bson::Array(data)
        };

        // create dict/hashmap/document to send to the gui
        // should be sufficiently fast using bson
        let doc = doc! {
            "images": images,
            "data": data,
        };
        let mut buf = Vec::new();
        doc.to_writer(&mut buf).unwrap();

        stream.write(u32::to_le_bytes(buf.len() as u32).as_ref())?;
        stream.write(&buf)?;

        // TODO receive control
    }
}

pub(crate) fn start(cfg: &Config) -> (Sender<Image>, Sender<(String, SystemTime, Bson)>, Receiver<(String, String)>) {
    // create channels with a size of 10 (small buffer)
    let (image_tx, image_rx) = bounded(10);
    let (data_tx, data_rx) = bounded(10);
    let (control_tx, control_rx) = bounded(10);

    let bind_str = format!("{}:{}", cfg.bind_addr, cfg.bind_port_gui);

    // only one gui connection at a time
    thread::spawn(move || {
        let listener = TcpListener::bind(bind_str).expect("binding the gui listener failed");

        for stream in listener.incoming() {
            let stream = stream.expect("opening the gui's tcp stream failed");
            // to always know if there is a gui running...
            GUI_HANDLER_RUNNING.store(true, Ordering::SeqCst);
            let _ = handle_stream(stream, &image_rx, &data_rx, &control_tx);
            // store if there is a handler running :D
            GUI_HANDLER_RUNNING.store(false, Ordering::SeqCst);
        }

        drop(listener);
    });

    (image_tx, data_tx, control_rx)
}
