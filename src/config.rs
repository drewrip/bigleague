use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Stats {
    pub rosters_interval: i32,
    pub players_interval: i32,
    pub users_interval: i32,
    pub leagues_interval: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Database {
    pub host: String,
    pub port: Option<String>,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Bigleague {
    pub leagues: Vec<String>,
    pub playoff_teams: i32,
    pub playoff_start_week: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub stats: Stats,
    pub database: Database,
    pub bigleague: Bigleague,
}

pub fn read_config(path: &str) -> Result<Config, toml::de::Error> {
    let raw_config = std::fs::read_to_string(path).expect("Can't find config file");
    toml::from_str(&raw_config)
}
