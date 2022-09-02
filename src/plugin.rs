use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::thread;
use std::thread::JoinHandle;
use log::info;
use crate::config::PluginConfig;

// implements the functionality shared by input and data plugins, which is
// - starting the plugin
// - starting the tcp listener thread
// it allows it's streams to be used by a Handler that contains a function
// which holds the functionality that is specific for each type of plugin

pub struct Plugin {
    pub socket_thread: JoinHandle<()>,
    pub plugin_process: Child,
}

pub enum PluginStoppedReason {
    SocketThread,
    PluginProcess,
}

pub trait Handler: Send + Sync  {
    fn handle(&self, name: &String, stream: TcpStream);
}


impl Plugin {
    // has the subprocess or the thread finished?
    pub fn has_erroneously_stopped(&mut self) -> Option<PluginStoppedReason> {
        if self.socket_thread.is_finished() { Some(PluginStoppedReason::SocketThread) } else if self.plugin_process.try_wait().unwrap().is_some() { Some(PluginStoppedReason::PluginProcess) } else { None }
    }

    // create the thread and the start the subprocess
    pub fn new(name: &String, bind_addr: &String, bind_port: i32, plugin: &PluginConfig, handler: Box<dyn Handler>) -> Plugin {
        let bind_str = format!("{}:{}", bind_addr, bind_port);

        info!("starting plugin {:?} using {}:{}", name, bind_addr, bind_port);

        let name = name.clone();
        let socket_thread = thread::spawn(move || {
            let listener = TcpListener::bind(&bind_str).expect("binding failed");

            for stream in listener.incoming() {
                let stream = stream.expect("opening the tcp stream failed");
                handler.handle(&name, stream);
            }

            drop(listener);
        });

        let mut cmd = Command::try_from(plugin).unwrap();
        cmd.env("PALLEON_HOST", bind_addr);
        cmd.env("PALLEON_PORT", bind_port.to_string());
        let child = cmd.spawn().expect("starting the data plugin failed");

        Plugin { socket_thread, plugin_process: child }
    }
}
