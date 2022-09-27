use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use bson::{Bson, Document};
use crossbeam_channel::{bounded, Receiver, Sender};
use log::info;

use crate::{Config, DataManager, Plugin};
use crate::image::Image;
use crate::plugin::Handler;
use crate::wrapped_stream::WrappedStream;

#[derive(Clone)]
pub struct DataPluginHandler {
    image_rx: Receiver<Image>,
    data_tx: Sender<(String, String, SystemTime, Bson)>,
    data_mgr: Arc<Mutex<DataManager>>,
}

impl DataPluginHandler {
    pub fn collect_plugin_data(&self, source: &String, plugin_dependencies: &Document) -> Document {
        let mut requested_plugin_data = Document::new();

        for (plugin_name, nr_history) in plugin_dependencies {
            let nr_history = nr_history.as_i32().expect("received invalid nr for the history for dependencies") as usize;
            if let Some(src_data) = self.data_mgr.lock().unwrap().get_last(plugin_name.clone(), source, nr_history) {
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

        requested_plugin_data
    }
}

impl Handler for DataPluginHandler {
    fn handle(&self, data_plugin_name: &String, mut stream: WrappedStream) {
        // TODO clean this mess up using bson
        info!("received connection for plugin {:?}", data_plugin_name);

        // received dependencies
        let plugin_init = stream.recv_bson();
        let wants_image = plugin_init.get_bool("image").expect("data from plugin was invalid");
        let plugin_dependencies = plugin_init.get_document("dependencies").expect("expected 'dependencies' key in document");

        loop {
            // exc: once had the case that the data plugin was not entering its loop such that
            //      there was no exception from the plugin and it seemed like "the connection broke"
            //      without a reason but in reality it falsely closed normally
            let image = self.image_rx.recv().expect("somehow this channel collapsed.");

            // send data from other plugins
            let requested_plugin_data = self.collect_plugin_data(&image.input_source, plugin_dependencies);
            stream.send_bson(&requested_plugin_data).expect("could not send collected data");

            // image tx
            let timestamp = image.timestamp.clone();
            let input_source_name = image.input_source.clone();

            let mut doc = Document::from(image);
            if !wants_image {
                doc.remove("data").expect("could not remove data key from document");
            }
            stream.send_bson(&doc).expect("could not send image data");

            // data rx
            let data = stream.recv_bson();

            // data tx
            self.data_tx.send((data_plugin_name.clone(), input_source_name, timestamp, Bson::from(data))).expect("TODO: panic message");
        };
    }
}


pub fn start(cfg: &Config, data_mgr: &Arc<Mutex<DataManager>>) -> (Vec<Plugin>, Vec<Sender<Image>>, Vec<Receiver<(String, String, SystemTime, Bson)>>) {
    let mut image_txs = vec![];
    let mut data_rxs = vec![];
    let mut plugins = vec![];

    for (i, (name, plugin)) in (&cfg.data_plugins).into_iter().enumerate() {
        let (image_tx, image_rx): (Sender<Image>, Receiver<Image>) = bounded(0);
        let (data_tx, data_rx): (Sender<(String, String, SystemTime, Bson)>, Receiver<(String, String, SystemTime, Bson)>) = bounded(0);

        let bind_port = cfg.bind_port_range_start + cfg.input_plugins.len() as i32 + i as i32;
        let plugin = Plugin::new(name, &cfg.bind_addr, bind_port, plugin, Box::new(DataPluginHandler { image_rx, data_tx, data_mgr: data_mgr.clone() }));

        plugins.push(plugin);
        image_txs.push(image_tx);
        data_rxs.push(data_rx);
    }

    (plugins, image_txs, data_rxs)
}
