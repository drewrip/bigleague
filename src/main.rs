use reqwest;
use tera::{Tera, Context};
use serde::{Serialize, Deserialize};
use serde_json::{Result, Value};
use std::collections::HashMap;
use warp::{http::StatusCode, Filter, reject, Reply, Rejection};
use std::sync::Arc;
use mobc::{Connection, Pool};
use mobc_postgres::{tokio_postgres, PgConnectionManager};
use tokio_postgres::{Config, Error, NoTls};
use std::fs;
use std::str::FromStr;
use std::time::Duration;
use std::convert::Infallible;
use std::future::Future;

const DB_POOL_MAX_OPEN: u64 = 32;
const DB_POOL_MAX_IDLE: u64 = 8;
const DB_POOL_TIMEOUT_SECONDS: u64 = 15;

type DBCon = Connection<PgConnectionManager<NoTls>>;
type DBPool = Pool<PgConnectionManager<NoTls>>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Standings {
    users: Vec<User>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct League {
   id: String,
   name: String,
   avatar: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    id: String,
    wins: u64,
    losses: u64,
    ties: u64,
    fpts: u64,
    fpts_decimal: u64,
    fpts_against: u64,
    fpts_against_decimal: u64,
    league: String,
    avatar: String,
}


async fn create_tables(db_pool: &DBPool) -> Result<()>{

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

    // Create table for users in the big league
    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS users (
            id varchar(64) PRIMARY KEY,
            wins integer NOT NULL,
            losses integer NOT NULL,
            ties integer NOT NULL,
            fpts integer NOT NULL,
            fpts_decimal integer NOT NULL,
            fpts_against integer NOT NULL,
            fpts_against_decimal integer NOT NULL,
            league varchar(64) REFERENCES leagues(id),
            avatar varchar(64)
        )
        "
    ).await.unwrap();

    Ok(())
    // Players later?
}
/*
fn fetch_rosters(id: &str, client: &mut Client) -> Result<()> {
    let body = reqwest::blocking::get(format!("https://api.sleeper.app/v1/league/{}/rosters", id)).unwrap().text().unwrap();
    let roster_list: Vec<Value> = serde_json::from_str(&body)?;

    // This is really ugly, should improve the deserialization later
    for r in roster_list {
        client.execute(
            "
            INSERT INTO users VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT(id) DO UPDATE SET
                id = EXCLUDED.id,
                wins = EXCLUDED.wins,
                losses = EXCLUDED.losses,
                ties = EXCLUDED.ties,
                fpts = EXCLUDED.fpts,
                fpts_decimal = EXCLUDED.fpts_decimal,
                fpts_against = EXCLUDED.fpts_against,
                fpts_against_decimal = EXCLUDED.fpts_against_decimal,
                league = EXCLUDED.league
                avatar = EXCLUDED.avatar
            ",
            &[
                &r["owner_id"].as_str().unwrap(),
                &i32::try_from(r["settings"]["wins"].as_i64().unwrap()).unwrap(),
                &i32::try_from(r["settings"]["losses"].as_i64().unwrap()).unwrap(),
                &i32::try_from(r["settings"]["ties"].as_i64().unwrap()).unwrap(),
                &i32::try_from(r["settings"]["fpts"].as_i64().unwrap()).unwrap(),
                &0i32, //&r["settings"]["fpts_decimal"].as_u64().unwrap().to_string(),
                &0i32, //&r["settings"]["fpts_against"].as_u64().unwrap().to_string(),
                &0i32, //&r["settings"]["fpts_against_decimal"].as_u64().unwrap().to_string(),
                &r["league_id"].as_str().unwrap(),
                &r["avatar"].as_str().unwrap(),
            ]
        ).unwrap();
    }

    Ok(())
}

fn fetch_leagues(id: &str, client: &mut Client) -> Result<()> {
    let body = reqwest::blocking::get(format!("https://api.sleeper.app/v1/league/{}", id)).unwrap().text().unwrap();
    let league: Value = serde_json::from_str(&body)?;

    // This is really ugly, should improve the deserialization later
    client.execute(
        "
        INSERT INTO leagues VALUES ($1, $2, $3)
        ON CONFLICT(id) DO UPDATE SET
            id = EXCLUDED.id,
            name = EXCLUDED.name,
            avatar = EXCLUDED.avatar
        ",
        &[
            &league["league_id"].as_str().unwrap(),
            &league["name"].as_str().unwrap(),
            &league["avatar"].as_str().unwrap(),
        ]
    ).unwrap();

    Ok(())
}
*/

// GET /league/<id>
async fn league(id: i64, db_pool: DBPool, tera: Arc<Tera>) -> std::result::Result<impl Reply, Rejection> { 
    let db = get_db_con(&db_pool)
            .await;

    let idstr = id.to_string();
    let rows = db.query("SELECT * FROM leagues WHERE id = $1", &[&idstr])
            .await
            .unwrap();

    let league = League {
        id: rows[0].get(0),
        name: rows[0].get(1),
        avatar: rows[0].get(2),
    };
   let mut ctx = Context::new();
   ctx.insert("league", &league);
   Ok(render("league.html", ctx, tera)) 
}

fn user(id: &str) -> String {
    "user".to_string()
}

fn standings() -> String {
    "standings".to_string()
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

pub async fn get_db_con(db_pool: &DBPool) -> DBCon {
    db_pool.get().await.unwrap()
}

fn with_db(db_pool: DBPool) -> impl Filter<Extract = (DBPool,), Error = Infallible> + Clone {
    warp::any().map(move || db_pool.clone())
}

fn with_tera(tera: Arc<Tera>) -> impl Filter<Extract = (Arc<Tera>,), Error = Infallible> + Clone {
    warp::any().map(move || tera.clone())
}

pub async fn health_handler(db_pool: DBPool) -> std::result::Result<impl Reply, Rejection> {
    let db = get_db_con(&db_pool)
            .await;

    db.execute("SELECT 1", &[])
            .await
            .unwrap();
    Ok(StatusCode::OK)
}

fn render(template: &str, ctx: Context, tera: Arc<Tera>) -> impl Reply {
    let render = tera.render(template, &ctx).unwrap();
    warp::reply::html(render)
}

#[tokio::main]
async fn main() {
    
    //let mut client = Client::connect("host=0.0.0.0 user=admin password=password dbname=test", NoTls).unwrap();
 /*   
    for row in client.query("SELECT name, age FROM names", &[]).unwrap(){
        let name: &str = row.get(0);
        let age: i32 = row.get(1);
        println!("Name: {:?}, Age: {:?}", name, age);
    }
*/
    //create_tables(&mut client);

    //fetch_rosters("940868057520549888", &mut client);

    let pool = create_pool().unwrap();
   
    create_tables(&pool).await.unwrap();

    let mut tera: Tera = Tera::new("templates/**/*").unwrap();
    let tera: Arc<Tera> = Arc::new(tera);

    let health_route = warp::path!("health")
        .and(with_db(pool.clone()))
        .and_then(health_handler);

    let league_route = warp::path!("league" / i64)
        .and(with_db(pool.clone()))
        .and(with_tera(tera.clone()))
        .and_then(league);

    let user_route = warp::path!("user" / u64).map(|id| format!("showing user {}", id));
    let standings_route = warp::path::end().map(|| "Look at all those standings!");

    let routes = warp::get().and(
        health_route
            .or(league_route)
            .or(user_route)
            .or(standings_route) 
            .with(warp::cors().allow_any_origin())
    );

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}
