extern crate toml;
extern crate reqwest;
#[macro_use] extern crate rocket;

use rocket::State;
use rocket_db_pools::{sqlx, Database, Connection};
use rocket_dyn_templates::{Template, context};
use serde::{Serialize, Deserialize};
use serde_json::{Result, Value};
use std::collections::HashMap;
use postgres::{Client, NoTls, Error};

#[derive(Database)]
#[database("bigleague")]
struct BigLeague(sqlx::PostgresPool);

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

fn create_tables(client: &mut Client) {

    // Create table for the leagues that the users are in
    client.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS leagues (
            id varchar(64) PRIMARY KEY,
            name varchar(64) NOT NULL,
            avatar varchar(64),
        ) 
        "
    ).unwrap();

    // Create table for users in the big league
    client.batch_execute(
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
    ).unwrap();

    // Players later?
}

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

#[get("/league/<id>")]
fn league(mut db: Connection<BigLeague>, id: &str) -> Template 
    let row = sqlx::query("SELECT id, name, avatar FROM leagues WHERE id = ?").bind(id)
        .fetch_one(&mut *db).await.unwrap();
    
    let league = League {
        id: row.try_get(0).unwrap(),
        name: row.try_get(1).unwrap(),
        avatar: row.try_get(2).unwrap(),
    };

    Template::render("league", league)
}

#[get("/user/<id>")]
fn user(id: &str) -> String {
    format!("Getting user {}...", id)
}

#[get("/")]
fn standings() -> Template {
    let mut ctx: Standings = Standings {
        users: vec![],
    };

    Template::render("standings", ctx)
}

#[launch]
fn rocket() -> _ {
    
    let mut client = Client::connect("host=0.0.0.0 user=admin password=password dbname=test", NoTls).unwrap();
    
    for row in client.query("SELECT name, age FROM names", &[]).unwrap(){
        let name: &str = row.get(0);
        let age: i32 = row.get(1);
        println!("Name: {:?}, Age: {:?}", name, age);
    }

    create_tables(&mut client);

    fetch_rosters("940868057520549888", &mut client);

    rocket::build()
        .attach(BigLeague::init())
        .mount("/", routes![league, user, standings])
        .attach(Template::fairing())
}
