use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Web {
    pub ip: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stats {
    pub rosters_interval: u64,
    pub players_interval: u64,
    pub users_interval: u64,
    pub leagues_interval: u64,
    pub dev_mode: Option<bool>,
    pub players_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Database {
    pub host: String,
    pub port: Option<u16>,
    pub user: String,
    pub password: String,
    pub dbname: String,
    pub max_open: u64,
    pub max_idle: u64,
    pub timeout: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Bigleague {
    pub leagues: Vec<String>,
    pub playoff_teams: i32,
    pub playoff_start_week: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub web: Web,
    pub stats: Stats,
    pub database: Database,
    pub bigleague: Bigleague,
}

pub fn read_config(path: &str) -> Result<Config, toml::de::Error> {
    let raw_config = std::fs::read_to_string(path).expect("Can't find config file");
    toml::from_str(&raw_config)
}
