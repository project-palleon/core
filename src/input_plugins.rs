use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use log::{debug, info};

use crate::{Config, Plugin};
use crate::image::Image;
use crate::plugin::Handler;

#[derive(Clone)]
pub(crate) struct InputPluginHandler {
    image_tx: Sender<Image>,
}

impl Handler for InputPluginHandler {
    fn handle(&self, name: &String, mut stream: TcpStream) {
        // TODO clean this mess up using bson
        info!("received connection for plugin {:?}", name);

        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut buffer = [0 as u8; 4];

        loop {
            stream.write(b"i").expect("");

            reader.read_exact(&mut buffer).expect("receiving the response name failed");
            let mode = u32::from_le_bytes(buffer);

            if mode == 0 {
                info!("no data (sleeping for one second)");
                thread::sleep(Duration::from_secs(1));
            } else if mode == 1 {
                reader.read_exact(&mut buffer).expect("receiving the length failed");
                let size = u32::from_le_bytes(buffer);

                let mut buf = vec![0u8; size as usize];
                reader.read_exact(&mut buf).expect("receiving the image data failed");

                self.image_tx.send(Image::new(buf, name.clone())).expect("TODO: panic message");

                // generates too much output, only practicable if nr of incoming frames is not that high
                debug!("received one frame from {:?} ({} bytes)", name, size);
            }
        }
    }
}

pub(crate) fn start(cfg: &Config) -> (Vec<Plugin>, Vec<Receiver<Image>>) {
    let mut image_receivers = vec![];
    let mut plugins = vec![];

    for (i, (name, plugin)) in (&cfg.input_plugins).into_iter().enumerate() {
        let (image_tx, image_rx) = bounded(0);

        let plugin = Plugin::new(name, &cfg.bind_addr, cfg.bind_port_range_start + i as i32, plugin, Box::new(InputPluginHandler { image_tx }));

        image_receivers.push(image_rx);
        plugins.push(plugin);
    }

    (plugins, image_receivers)
}



