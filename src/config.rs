use std::{fs, io};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use serde::{Deserialize};

// config is the rust representation of the config.toml
// its being deserialized using the toml and serde package
// they also _kind of_ take care of validating the input,
// but not completely

#[derive(Deserialize, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub bind_port_range_start: i32,
    pub bind_port_gui: i32,
    #[serde(alias = "data")]
    pub data_plugins: HashMap<String, PluginConfig>,
    #[serde(alias = "input")]
    pub input_plugins: HashMap<String, PluginConfig>,
}

#[derive(Deserialize, Debug)]
pub struct PluginConfig {
    pub command: Vec<String>,
    pub environment: Option<HashMap<String, String>>,
    pub working_directory: String,
}

// a plugin has to start another process (the plugin - wow, who would have thought)
// the rust way of representing another not started process is the Command, so it's
// logical to implement a From
// in this case it's a TryFrom because the toml input is not validated enough, which means
// one could pass an empty array [] as command, which of course, won't work

impl TryFrom<&PluginConfig> for Command {
    type Error = &'static str;

    fn try_from(pc: &PluginConfig) -> Result<Self, Self::Error> {
        // what's said above is validated exactly here...
        if pc.command.len() < 1 {
            Err("a plugin command must contain a path to an executable")
        } else {
            // select the executable
            let mut command = Command::new(&pc.command[0]);

            // add the arguments
            for i in 1..pc.command.len() {
                command.arg(&pc.command[i]);
            }

            // add additional environment variables (if there are some)
            match &pc.environment {
                None => {}
                Some(env) => {
                    for (key, value) in env {
                        command.env(key, value);
                    }
                }
            }

            // change the working directory
            command.current_dir(&pc.working_directory);

            // return the command
            Ok(command)
        }
    }
}

// wrap all loading into one simple function

pub fn load(p: &Path) -> Result<Config, io::Error> {
    Ok(toml::from_str(fs::read_to_string(p)?.as_str()).expect("parsing toml config failed"))
}