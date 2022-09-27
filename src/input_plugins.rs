use std::thread;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use log::{debug, info};

use crate::{Config, Plugin};
use crate::image::Image;
use crate::plugin::Handler;
use crate::wrapped_stream::WrappedStream;

#[derive(Clone)]
pub struct InputPluginHandler {
    image_tx: Sender<Image>,
}

impl Handler for InputPluginHandler {
    fn handle(&self, input_plugin_name: &String, mut stream: WrappedStream) {
        // TODO clean this mess up using bson
        info!("received connection for plugin {:?}", input_plugin_name);

        loop {
            stream.write(b"i").expect("could not send request for image");

            let mode = stream.recv_32bit_integer();

            if mode == 0 {
                info!("no data (sleeping for one second)");
                thread::sleep(Duration::from_secs(1));
            } else if mode == 1 {
                let mut buf = stream.recv_based_on_32bit_integer();

                self.image_tx.send(Image::new(buf, input_plugin_name.clone())).expect("TODO: panic message");

                // generates too much output, only practicable if nr of incoming frames is not that high
                debug!("received one frame from {:?}", input_plugin_name);
            }
        }
    }
}

pub fn start(cfg: &Config) -> (Vec<Plugin>, Vec<Receiver<Image>>) {
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



