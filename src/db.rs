use serde_json::Result;
use warp::Filter;
use mobc::{Connection, Pool};
use mobc_postgres::{tokio_postgres, PgConnectionManager};
use tokio_postgres::{Config, Error, NoTls};
use std::str::FromStr;
use std::time::Duration;
use std::convert::Infallible;
use serde::{Serialize, Deserialize};

const DB_POOL_MAX_OPEN: u64 = 32;
const DB_POOL_MAX_IDLE: u64 = 8;
const DB_POOL_TIMEOUT_SECONDS: u64 = 15;

pub type DBCon = Connection<PgConnectionManager<NoTls>>;
pub type DBPool = Pool<PgConnectionManager<NoTls>>;


#[derive(Serialize, Deserialize, Debug)]
pub struct Standing {
    pub user: User,
    pub roster: Roster,
    pub rank: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct League {
    pub id: String,
    pub name: String,
    pub avatar: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Roster {
    pub user_id: String,
    pub league_id: String,
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
    pub fpts: i32,
    pub fpts_decimal: i32,
    pub fpts_against: i32,
    pub fpts_against_decimal: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub name: String,
    pub avatar: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub team: String,
    pub position: String,
    pub status: String,
    pub starter: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ownership {
    pub user_id: String,
    pub league_id: String,
    pub player_id: String,
    pub starter: i32,
}

pub async fn get_db_con(db_pool: &DBPool) -> DBCon {
    db_pool.get().await.unwrap()
}

pub fn with_db(db_pool: DBPool) -> impl Filter<Extract = (DBPool,), Error = Infallible> + Clone {
    warp::any().map(move || db_pool.clone())
}

pub fn create_pool() -> std::result::Result<DBPool, mobc::Error<Error>> {
    let config = Config::from_str("host=0.0.0.0 user=admin password=password dbname=test")?;

    let manager = PgConnectionManager::new(config, NoTls);
    Ok(Pool::builder()
            .max_open(DB_POOL_MAX_OPEN)
            .max_idle(DB_POOL_MAX_IDLE)
            .get_timeout(Some(Duration::from_secs(DB_POOL_TIMEOUT_SECONDS)))
            .build(manager))
}

pub async fn create_tables(db_pool: &DBPool) -> Result<()>{

    let con = get_db_con(db_pool).await;

    // Create table for the leagues that the users are in
    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS leagues (
            id varchar(64) PRIMARY KEY,
            name varchar(64) NOT NULL,
            avatar varchar(64)
        ) 
        "
    ).await.unwrap();


    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS users (
            id varchar(64) PRIMARY KEY,
            name varchar(64) NOT NULL,
            avatar varchar(64)
        )
        "
    ).await.unwrap();
    
    // Create table for users in the big league
    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS rosters (
            user_id varchar(64) PRIMARY KEY,
            league_id varchar(64) REFERENCES leagues(id),
            wins integer NOT NULL,
            losses integer NOT NULL,
            ties integer NOT NULL,
            fpts integer NOT NULL,
            fpts_decimal integer NOT NULL,
            fpts_against integer NOT NULL,
            fpts_against_decimal integer NOT NULL
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS players (
           id varchar(64) PRIMARY KEY,
           first_name varchar(64) NOT NULL,
           last_name varchar(64) NOT NULL,
           team varchar(64),
           position varchar(64),
           status varchar(64)
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS ownership (
            user_id varchar(64) NOT NULL,
            league_id varchar(64) NOT NULL,
            player_id varchar(64) NOT NULL,
            starter integer,
            PRIMARY KEY (user_id, league_id, player_id)
        )
        "
    ).await.unwrap();

    Ok(())
}
