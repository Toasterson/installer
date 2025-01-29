use std::fs::create_dir_all;
use platform_dirs::AppDirs;
use serde::{Deserialize, Serialize};
use crate::{Error, Result};

const SATE_FILE_NAME: &str = "state.json";
const APP_NAME: &str = "installadm";

#[derive(Clone, Deserialize, Serialize)]
pub struct State {
    servers: Vec<Server>,
}

impl State {
    pub fn add_server(&mut self, server: Server) {
        self.servers.push(server);
    }

    pub fn get_server(&self, name: &str) -> Option<&Server> {
        self.servers.iter().filter(|server| server.name == name).next().clone()
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Server {
    pub name: String,
    pub uri: String,
    pub claim_token: String,
}

pub fn read_state_file() -> Result<State> {
    let app_dirs = AppDirs::new(Some(APP_NAME), false).ok_or(Error::NoAppDir)?;
    let state_file = app_dirs.config_dir.join(SATE_FILE_NAME);
    if !state_file.exists() {
        create_dir_all(&state_file.parent().ok()?)?;
        std::fs::File::create(state_file)?;
        Ok(State{
            servers: Vec::new(),
        })
    } else {
        let rdr = std::io::BufReader::new(std::fs::File::open(state_file)?);
        Ok(serde_json::from_reader(rdr))
    }
}

pub fn save_state(state: State) -> Result<()> {
    let app_dirs = AppDirs::new(Some(APP_NAME), false).ok_or(Error::NoAppDir)?;
    let state_file = app_dirs.config_dir.join(SATE_FILE_NAME);
    if !state_file.exists() {
        create_dir_all(&state_file.parent().ok()?)?;
    }
    let writer = std::io::BufWriter::new(std::fs::File::create(state_file)?);
    serde_json::to_writer_pretty(writer, &state)?;
}