use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, UNIX_EPOCH};

use log::{debug, info};

use crate::config::Config;
use crate::data_manager::DataManager;
use crate::gui_connector::GUI_HANDLER_RUNNING;
use crate::plugin::Plugin;

mod config;
mod input_plugins;
mod data_plugins;
mod image;
mod plugin;
mod data_manager;
mod gui_connector;

fn check_everything_running(input_plugins: &mut Vec<Plugin>, data_plugins: &mut Vec<Plugin>) {
    let iter = input_plugins.iter_mut().chain(data_plugins.iter_mut());

    if iter.map(|k| k.has_erroneously_stopped()).any(|k| k.is_some()) {
        // if ANYTHING (subprocess/child or thread) exists unexpectedly stop the
        // core (and everything else) one could handle this gracefully, but that would
        // be way more complicated
        panic!("some thread or plugin process unexpectedly exited.")
    }
}


fn main() {
    env_logger::init();

    let cfg = config::load(Path::new("config.toml")).expect("loading config failed");

    let data_manager = Arc::new(Mutex::new(DataManager::new()));

    let (gui_image_tx, gui_data_tx, gui_control_rx) = gui_connector::start(&cfg);

    let (mut input_plugins, image_rxs) = input_plugins::start(&cfg);
    let (mut data_plugins, image_txs, data_rxs) = data_plugins::start(&cfg, &data_manager);

    let mut secondly_printer_timer = Instant::now();

    loop {
        check_everything_running(&mut input_plugins, &mut data_plugins);

        // spread all images from input to data
        for image_rx in &image_rxs {
            // try to receive image
            let image = image_rx.recv_timeout(Duration::from_millis(5));
            if image.is_err() { continue; }

            // distribute that image to all data plugins
            let image = image.unwrap();
            for image_tx in &image_txs {
                image_tx.send(image.clone()).expect("failed adding to queue for data plugins");
            }
            // if there is a gui connected, also send that image to the gui
            if GUI_HANDLER_RUNNING.load(Ordering::SeqCst) {
                let _ = gui_image_tx.send(image);
            }

            for data_rx in &data_rxs {
                // now wait BLOCKING-ly for EVERY data plugin to return something
                let data = data_rx.recv();

                let data = data.unwrap();
                // if there is a gui connected, also send the returned data to the gui
                if GUI_HANDLER_RUNNING.load(Ordering::SeqCst) {
                    let _ = gui_data_tx.send((data.0.clone(), data.1, data.2.clone()));
                }

                // add the returned data to the data manager
                data_manager.lock().unwrap().add(data.0, data.1, data.2);
            }
        }

        // if the last "print" more than 1 second ago, print what every is in this if-case
        // this is for regular debug/status messages
        // because otherwise it wouldn't be clear if the core is still running
        if secondly_printer_timer.elapsed() > Duration::from_secs(1) {
            info!("alive");

            // debug print last 10 values in the DataManager from the activity plugin
            if let Some(data_time_series) = data_manager.lock().unwrap().get_last(String::from("activity"), 10) {
                for (i, (timestamp, data)) in data_time_series.iter().enumerate() {
                    debug!("{}: {} {:?}", i, timestamp.duration_since(UNIX_EPOCH).unwrap().as_millis(), data);
                }
            }

            secondly_printer_timer = Instant::now();
        }
    }
}
