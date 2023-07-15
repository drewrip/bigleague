use reqwest;
use serde_json::Value;
use crate::db;
use std::convert::Infallible;
use std::error::Error;
use tokio::time;
use std::sync::Arc;
use log::{info, warn};
use std::collections::{HashMap, HashSet};

use crate::config;

pub async fn stats_loop(config: config::Config, db_pool: Arc<db::DBPool>) -> Result<(), Box<dyn Error>> {

    info!("starting stats loop");

    let dev_mode = match config.clone().stats.dev_mode {
        Some(m) => m,
        None => false,
    };

    if dev_mode {
        warn!("running in dev mode");
    }

    let players_path = match config.clone().stats.players_path {
        Some(p) => p,
        None => {
            match dev_mode {
                true => String::from("data/players.json"),
                false => String::from(""),
            }
        },
    };

    let mut rosters_interval = time::interval(
        time::Duration::from_secs(config.stats.rosters_interval)
    );
    // Change the missed tick behavior so getting behind doesn't accidentally
    // result in a burst of calls to Sleeper's API.
    rosters_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    let mut users_interval = time::interval(
        time::Duration::from_secs(config.stats.users_interval)
    );
    users_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    let mut leagues_interval = time::interval(
        time::Duration::from_secs(config.stats.leagues_interval)
    );
    leagues_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    let mut players_interval = time::interval(
        time::Duration::from_secs(config.stats.players_interval)
    );
    players_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    let mut state_interval = time::interval(
        time::Duration::from_secs(config.stats.state_interval)
    );
    state_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    let mut matchups_interval = time::interval(
        time::Duration::from_secs(config.stats.matchups_interval)
    );
    matchups_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = rosters_interval.tick() => {
                for league_id in config.clone().bigleague.leagues {
                    let _ = fetch_rosters(&db_pool, league_id).await;
                }
            }
            _ = users_interval.tick() => {
                for league_id in config.clone().bigleague.leagues {
                    let _ = fetch_users(&db_pool, league_id).await;
                }
            }
            _ = leagues_interval.tick() => {
                for league_id in config.clone().bigleague.leagues {
                    let _ = fetch_leagues(&db_pool, league_id).await;
                }
            }
            _ = players_interval.tick() => {
                let _ = fetch_players(&db_pool, dev_mode, players_path.clone()).await;
            }
            _ = state_interval.tick() => {
                let _ = fetch_state(&db_pool).await;
            }
            _ = matchups_interval.tick() => {
                for league_id in config.clone().bigleague.leagues {
                    let _ = fetch_matchups(&db_pool, league_id).await;
                }
            }
        }    
    }

    Ok(())
}

pub async fn fetch_rosters(db_pool: &db::DBPool, league_id: String) -> Result<(), Infallible> {

    info!("fetching rosters for league: {}", league_id);

    let con = db::get_db_con(db_pool).await;

    let body = reqwest::get(format!("https://api.sleeper.app/v1/league/{}/rosters", league_id))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let roster_list: Vec<Value> = serde_json::from_str(&body).unwrap();

    // This is really ugly, should improve the deserialization later
    for r in roster_list {
        con.execute(
            "
            INSERT INTO rosters VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT(user_id) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                league_id = EXCLUDED.league_id,
                wins = EXCLUDED.wins,
                losses = EXCLUDED.losses,
                ties = EXCLUDED.ties,
                fpts = EXCLUDED.fpts,
                fpts_decimal = EXCLUDED.fpts_decimal,
                fpts_against = EXCLUDED.fpts_against,
                fpts_against_decimal = EXCLUDED.fpts_against_decimal,
                roster_id = EXCLUDED.roster_id
            ",
            &[
                &r["owner_id"].as_str().unwrap(),
                &r["league_id"].as_str().unwrap(),
                &i32::try_from(r["settings"]["wins"].as_i64().unwrap_or(0)).unwrap(),
                &i32::try_from(r["settings"]["losses"].as_i64().unwrap_or(0)).unwrap(),
                &i32::try_from(r["settings"]["ties"].as_i64().unwrap_or(0)).unwrap(),
                &i32::try_from(r["settings"]["fpts"].as_i64().unwrap_or(0)).unwrap(),
                &i32::try_from(r["settings"]["fpts_decimal"].as_i64().unwrap_or(0)).unwrap(),
                &i32::try_from(r["settings"]["fpts_against"].as_u64().unwrap_or(0)).unwrap(),
                &i32::try_from(r["settings"]["fpts_against_decimal"].as_u64().unwrap_or(0)).unwrap(), 
                &i32::try_from(r["roster_id"].as_u64().unwrap_or(0)).unwrap(),
            ]
        ).await.unwrap();

        let players: Vec<String> = r["players"]
            .as_array()
            .unwrap()
            .into_iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect();

        let starters: Vec<String> = r["starters"]
            .as_array()
            .unwrap()
            .into_iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect();

        // Add all their players first
        for p in players {
            con.execute(
                "
                INSERT INTO ownership VALUES ($1, $2, $3, $4)
                ON CONFLICT(user_id, league_id, player_id) DO UPDATE SET
                    user_id = EXCLUDED.user_id,
                    league_id = EXCLUDED.league_id,
                    player_id = EXCLUDED.player_id,
                    starter = EXCLUDED.starter
                ",
                &[
                    &r["owner_id"].as_str().unwrap(),
                    &r["league_id"].as_str().unwrap(),
                    &p,
                    &0i32,
                ]
            ).await.unwrap();
        }

        // Now update the players for which are starters
        for s in starters {
            con.execute(
                "
                UPDATE ownership
                SET starter = 1
                WHERE user_id = $1 AND
                league_id = $2 AND
                player_id = $3
                ",
                &[
                    &r["owner_id"].as_str().unwrap(),
                    &r["league_id"].as_str().unwrap(),
                    &s,
                ]
            ).await.unwrap();
        }
    }

    Ok(())
}

pub async fn fetch_leagues(db_pool: &db::DBPool, league_id: String) -> Result<(), Infallible> {

    info!("fetching info about league: {}", league_id);

    let con = db::get_db_con(db_pool).await;

    let body = reqwest::get(format!("https://api.sleeper.app/v1/league/{}", league_id))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let league: Value = serde_json::from_str(&body).unwrap();

    con.execute(
        "
        INSERT INTO leagues VALUES ($1, $2, $3)
        ON CONFLICT(id) DO UPDATE SET
            id = EXCLUDED.id,
            name = EXCLUDED.name,
            avatar = EXCLUDED.avatar
        ",
        &[
            &league["league_id"].as_str().unwrap(),
            &league["name"].as_str().unwrap_or("NA"),
            &league["avatar"].as_str().unwrap_or("NA"),
        ]
    ).await.unwrap();

    Ok(())
}

pub async fn fetch_users(db_pool: &db::DBPool, league_id: String) -> Result<(), Infallible> {

    info!("fetching users for league: {}", league_id);

    let con = db::get_db_con(db_pool).await;

    let rows = con.query("SELECT user_id FROM rosters", &[]).await.unwrap();

    let user_ids: Vec<String> = rows.into_iter()
        .map(|row| row.get(0))
        .collect();

    let body = reqwest::get(format!("https://api.sleeper.app/v1/league/{}/users", league_id))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let users: Vec<Value> = serde_json::from_str(&body).unwrap();
    for user in users {
        con.execute(
            "
            INSERT INTO users VALUES ($1, $2, $3)
            ON CONFLICT(id) DO UPDATE SET
                id = EXCLUDED.id,
                name = EXCLUDED.name,
                avatar = EXCLUDED.avatar
            ",
            &[
                &user["user_id"].as_str().unwrap(),
                &user["display_name"].as_str().unwrap_or("NA"),
                &user["avatar"].as_str().unwrap_or("NA"),
            ]
        ).await.unwrap();
    }
    Ok(())
}

// Be careful with fetch players!
// The underlying call to Sleeper's API is expensive and
// according to their docs, we shouldn't call this more
// than once a day
pub async fn fetch_players(db_pool: &db::DBPool, dev_mode: bool, players_path: String) -> Result<(), Infallible> {

    info!("fetching all players");

    let con = db::get_db_con(db_pool).await;

    let body = if dev_mode {
        std::fs::read_to_string(players_path).expect("couldn't read temp players.json file")
    } else {
        reqwest::get("https://api.sleeper.app/v1/players/nfl")
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    };

    let players: Value = serde_json::from_str(&body).unwrap();

    for (player_id, player_data) in players.as_object().unwrap() {
        con.execute(
            "
            INSERT INTO players VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(id) DO UPDATE SET
                id = EXCLUDED.id,
                first_name = EXCLUDED.first_name,
                last_name = EXCLUDED.last_name,
                team = EXCLUDED.team,
                position = EXCLUDED.position,
                status = EXCLUDED.status
            ",
            &[
                &player_id,
                &player_data["first_name"].as_str().unwrap_or("NA"),
                &player_data["last_name"].as_str().unwrap_or("NA"),
                &player_data["team"].as_str().unwrap_or("None"),
                &player_data["position"].as_str().unwrap_or("NA"),
                &player_data["status"].as_str().unwrap_or(""),
            ]
        ).await.unwrap();
    }

    Ok(())
}

pub async fn fetch_state(db_pool: &db::DBPool) -> Result<(), Infallible> {

    info!("fetching the state of the NFL");

    let con = db::get_db_con(db_pool).await;

    let body = reqwest::get("https://api.sleeper.app/v1/state/nfl")
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

    let state: Value = serde_json::from_str(&body).unwrap();

    con.execute(
        "
        INSERT INTO state VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT(season, week) DO UPDATE SET
            season = EXCLUDED.season,
            week = EXCLUDED.week,
            league_season = EXCLUDED.league_season,
            display_week = EXCLUDED.display_week,
            season_type = EXCLUDED.season_type
            
        ",
        &[
            &state["season"].as_str().unwrap_or("0").parse::<i32>().unwrap_or(0),
            &i32::try_from(state["week"].as_u64().unwrap_or(0)).unwrap(),
            &state["league_season"].as_str().unwrap_or("0").parse::<i32>().unwrap_or(0),
            &i32::try_from(state["display_week"].as_u64().unwrap_or(0)).unwrap(),
            &state["season_type"].as_str().unwrap_or("NA"),
        ]
    ).await.unwrap();

    Ok(())
}

pub async fn fetch_matchups(db_pool: &db::DBPool, league_id: String) -> Result<(), Infallible> {

    info!("fetching matchups for league: {}", league_id);

    let con = db::get_db_con(db_pool).await;

    let time = con.query("SELECT season, week FROM state ORDER BY season DESC, week DESC LIMIT 1", &[])
        .await
        .unwrap();

    let season: i32 = 2022;//time[0].get("season");
    let week: i32 = 12;//time[0].get("week");

    let body = reqwest::get(format!("https://api.sleeper.app/v1/league/{}/matchups/{}", league_id, week))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let matchups: Vec<Value> = serde_json::from_str(&body).unwrap();

    let roster_map_for_league: HashMap<i32, String> = 
        con.query("
            SELECT roster_id, user_id FROM rosters WHERE league_id = $1
            ",
            &[&league_id])
        .await
        .unwrap()
        .iter()
        .map(|row| (row.get("roster_id"), row.get("user_id")))
        .collect();

    let mut opponent_map: HashMap<String, String> = HashMap::new();
    let mut opponent_resolutions: HashMap<u64, u64> = HashMap::new();
    for matchup in matchups.clone() {
        let m_id = matchup["matchup_id"].as_u64().unwrap();
        let r_id = matchup["roster_id"].as_u64().unwrap();

        if opponent_resolutions.contains_key(&m_id) {
            let opponents = (
                roster_map_for_league.get(&i32::try_from(r_id).unwrap()).unwrap(),
                roster_map_for_league.get(
                    &i32::try_from(
                        *opponent_resolutions.get(&m_id).unwrap()
                    ).unwrap()
                ).unwrap(),
            );
            opponent_map.insert(opponents.0.to_string(), opponents.1.to_string());
            opponent_map.insert(opponents.1.to_string(), opponents.0.to_string());
        } else {
            opponent_resolutions.insert(m_id, r_id);
        }
    }

    for matchup in matchups {
        let user = roster_map_for_league.get(
            &i32::try_from(matchup["roster_id"].as_u64().unwrap()).unwrap()
        ).unwrap();

        con.execute(
            "
            INSERT INTO matchups VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(season, week, league_id, user_id, opponent_id) DO UPDATE SET
                season = EXCLUDED.season,
                week = EXCLUDED.week,
                league_id = EXCLUDED.league_id,
                user_id = EXCLUDED.user_id,
                opponent_id = EXCLUDED.opponent_id,
                points = EXCLUDED.points
            ",
            &[
                &season,
                &week,
                &league_id,
                user,
                &opponent_map.get(
                    user
                ).unwrap(),
                &(matchup["points"].as_f64().unwrap() as f32),
            ]
        ).await.unwrap();

        for (player, points) in matchup["players_points"].as_object().unwrap() {
            con.execute(
            "
            INSERT INTO scores VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT(player_id, league_id, season, week) DO UPDATE SET
                player_id = EXCLUDED.player_id,
                league_id = EXCLUDED.league_id,
                season = EXCLUDED.season,
                week = EXCLUDED.week,
                points = EXCLUDED.points
            ",
            &[
                player,
                &league_id,
                &season,
                &week,
                &(points.as_f64().unwrap() as f32),
            ]
        ).await.unwrap();
    
        }
    } 
    Ok(())
}
