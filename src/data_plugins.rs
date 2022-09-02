use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use bson::{Bson, Document};
use crossbeam_channel::{bounded, Receiver, Sender};
use log::info;

use crate::{Config, DataManager, Plugin};
use crate::image::Image;
use crate::plugin::Handler;

#[derive(Clone)]
pub(crate) struct DataPluginHandler {
    image_rx: Receiver<Image>,
    data_tx: Sender<(String, SystemTime, Bson)>,
    data_mgr: Arc<Mutex<DataManager>>,
}

impl Handler for DataPluginHandler {
    fn handle(&self, name: &String, mut stream: TcpStream) {
        // TODO clean this mess up using bson
        info!("received connection for plugin {:?}", name);

        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut buffer = [0 as u8; 4];

        reader.read_exact(&mut buffer).unwrap();
        let init_size = u32::from_le_bytes(buffer);
        let mut buf = vec![0u8; init_size as usize];
        reader.read_exact(&mut buf).unwrap();

        let mut plugins = vec![];

        let mut skip = 0;
        let mut dep_name = vec![];
        for (i, b) in buf.iter().enumerate() {
            if skip > 0 { continue; }

            if *b == 0 {
                let amount = u32::from_le_bytes(buf[i + 1..i + 5].try_into().unwrap());
                plugins.push((String::from_utf8(dep_name.clone()).unwrap(), amount));
                dep_name.clear();
                skip = 4;
            } else {
                dep_name.push(*b);
            }
        }

        loop {
            let image = self.image_rx.recv().unwrap();

            let mut requested_plugin_data = Document::new();

            for (plugin_name, x) in &plugins {
                if let Some(src_data) = self.data_mgr.lock().unwrap().get_last(plugin_name.clone(), *x as usize) {
                    let mut data = vec![];

                    for (time, bson) in src_data {
                        data.push(Bson::Array(vec![
                            Bson::DateTime(bson::DateTime::from_system_time(time)),
                            bson,
                        ]));
                    }

                    requested_plugin_data.insert(plugin_name, Bson::Array(data));
                }
            }

            // data
            let mut b = Vec::new();
            requested_plugin_data.to_writer(&mut b).unwrap();
            stream.write(u32::to_le_bytes(b.len() as u32).as_ref()).expect("b");
            stream.write(b.as_slice()).expect("b");

            // image tx
            let frame_size = u32::to_le_bytes(image.data.len() as u32);
            stream.write(frame_size.as_ref()).unwrap();
            stream.write(image.data.as_ref()).unwrap();

            // data rx
            reader.read_exact(&mut buffer).unwrap();
            let data_size = u32::from_le_bytes(buffer);
            let mut buf = vec![0u8; data_size as usize];
            reader.read_exact(&mut buf).unwrap();

            let data = Document::from_reader_utf8_lossy(buf.as_slice()).expect("invalid bson received from data client");

            // data tx
            self.data_tx.send((name.clone(), image.timestamp, Bson::from(data))).expect("TODO: panic message");
        };
    }
}


pub(crate) fn start(cfg: &Config, data_mgr: &Arc<Mutex<DataManager>>) -> (Vec<Plugin>, Vec<Sender<Image>>, Vec<Receiver<(String, SystemTime, Bson)>>) {
    let mut image_txs = vec![];
    let mut data_rxs = vec![];
    let mut plugins = vec![];

    for (i, (name, plugin)) in (&cfg.data_plugins).into_iter().enumerate() {
        let (image_tx, image_rx): (Sender<Image>, Receiver<Image>) = bounded(0);
        let (data_tx, data_rx): (Sender<(String, SystemTime, Bson)>, Receiver<(String, SystemTime, Bson)>) = bounded(0);

        let bind_port = cfg.bind_port_range_start + cfg.input_plugins.len() as i32 + i as i32;
        let plugin = Plugin::new(name, &cfg.bind_addr, bind_port, plugin, Box::new(DataPluginHandler { image_rx, data_tx, data_mgr: data_mgr.clone() }));

        plugins.push(plugin);
        image_txs.push(image_tx);
        data_rxs.push(data_rx);
    }

    (plugins, image_txs, data_rxs)
}
