use serde_json::Result;
use warp::Filter;
use mobc::{Connection, Pool};
use mobc_postgres::{tokio_postgres, PgConnectionManager};
use tokio_postgres::{Config, Error, NoTls};
use std::str::FromStr;
use std::time::Duration;
use std::convert::Infallible;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use log::info;

use crate::config;

pub type DBCon = Connection<PgConnectionManager<NoTls>>;
pub type DBPool = Pool<PgConnectionManager<NoTls>>;


#[derive(Serialize, Deserialize, Debug)]
pub struct Standing {
    pub user: User,
    pub roster: Roster,
    pub league: League,
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
    pub roster_id: i32,
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
    pub points: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ownership {
    pub user_id: String,
    pub league_id: String,
    pub player_id: String,
    pub starter: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub season: i32,
    pub week: i32,
    pub league_season: i32,
    pub display_week: i32,
    pub season_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Matchup {
    pub season: i32,
    pub week: i32,
    pub league_id: String,
    pub user_id: String,
    pub opponent_id: String,
    pub points: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Score {
    pub player_id: String,
    pub league_id: String,
    pub season: i32,
    pub week: i32,
    pub points: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Week {
    pub league_id: String,
    pub season: i32,
    pub week: i32,
    pub user_id: String,
    pub user_name: String,
    pub user_avatar: String,
    pub user_points: f32,
    pub opponent_id: String,
    pub opponent_name: String,
    pub opponent_avatar: String,
    pub opponent_points: f32,
}

pub async fn get_db_con(db_pool: &DBPool) -> DBCon {
    db_pool.get().await.unwrap()
}

pub fn with_db(db_pool: Arc<DBPool>) -> impl Filter<Extract = (Arc<DBPool>,), Error = Infallible> + Clone {
    warp::any().map(move || db_pool.clone())
}

pub fn create_pool(bl_config: config::Config) -> std::result::Result<DBPool, mobc::Error<Error>> {
    
    info!("creating database pool");

    let pg_config = Config::from_str(
        &format!("host={} port={} user={} password={} dbname={}",
            bl_config.database.host,
            bl_config.database.port.unwrap_or(5432),
            bl_config.database.user,
            bl_config.database.password,
            bl_config.database.dbname,
        )
    )?;

    let manager = PgConnectionManager::new(pg_config, NoTls);
    Ok(Pool::builder()
            .max_open(bl_config.database.max_open)
            .max_idle(bl_config.database.max_idle)
            .get_timeout(Some(Duration::from_secs(bl_config.database.timeout)))
            .build(manager))
}

pub async fn create_tables(db_pool: Arc<DBPool>) -> Result<()>{

    info!("creating tables");

    let con = get_db_con(&db_pool).await;

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
            league_id varchar(64),
            wins integer NOT NULL,
            losses integer NOT NULL,
            ties integer NOT NULL,
            fpts integer NOT NULL,
            fpts_decimal integer NOT NULL,
            fpts_against integer NOT NULL,
            fpts_against_decimal integer NOT NULL,
            roster_id integer NOT NULL
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

    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS state (
            season integer NOT NULL,
            week integer NOT NULL,
            league_season integer NOT NULL,
            display_week integer NOT NULL,
            season_type varchar(64) NOT NULL,
            PRIMARY KEY (season, week)
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS matchups (
            season integer NOT NULL,
            week integer NOT NULL,
            league_id varchar(64) NOT NULL,
            user_id varchar(64) NOT NULL,
            opponent_id varchar(64) NOT NULL,
            points real NOT NULL,
            PRIMARY KEY (season, week, league_id, user_id, opponent_id)
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS scores (
            player_id varchar(64) NOT NULL,
            league_id varchar(64) NOT NULL,
            season integer NOT NULL,
            week integer NOT NULL,
            points real NOT NULL,
            PRIMARY KEY (player_id, league_id, season, week)
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE OR REPLACE VIEW ranks AS
            SELECT user_id, ROW_NUMBER() OVER (ORDER BY wins DESC, fpts DESC, fpts_decimal DESC, fpts_against DESC, fpts_against_decimal DESC) as rank
            FROM rosters, users 
            WHERE rosters.user_id = users.id
        "
    ).await.unwrap();
    Ok(())
}
