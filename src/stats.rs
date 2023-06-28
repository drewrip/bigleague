use reqwest;
use serde_json::Value;
use crate::db;
use std::convert::Infallible;

// fetch ...

pub async fn fetch_rosters(id: String, db_pool: &db::DBPool) -> Result<(), Infallible> {

    let con = db::get_db_con(db_pool).await;

    let body = reqwest::get(format!("https://api.sleeper.app/v1/league/{}/rosters", id))
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
            INSERT INTO rosters VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT(user_id) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                league_id = EXCLUDED.league_id,
                wins = EXCLUDED.wins,
                losses = EXCLUDED.losses,
                ties = EXCLUDED.ties,
                fpts = EXCLUDED.fpts,
                fpts_decimal = EXCLUDED.fpts_decimal,
                fpts_against = EXCLUDED.fpts_against,
                fpts_against_decimal = EXCLUDED.fpts_against_decimal
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
            ]
        ).await.unwrap();
    }

    Ok(())
}

pub async fn fetch_leagues(id: String, db_pool: &db::DBPool) -> Result<(), Infallible> {
    
    let con = db::get_db_con(db_pool).await;

    let body = reqwest::get(format!("https://api.sleeper.app/v1/league/{}", id))
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

pub async fn fetch_users(db_pool: &db::DBPool) -> Result<(), Infallible> {
    
    let con = db::get_db_con(db_pool).await;

    let rows = con.query("SELECT user_id FROM rosters", &[]).await.unwrap();

    let user_ids: Vec<String> = rows.into_iter()
        .map(|row| row.get(0))
        .collect();

    for user_id in user_ids {

        let body = reqwest::get(format!("https://api.sleeper.app/v1/user/{}", user_id))
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        let user: Value = serde_json::from_str(&body).unwrap();

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
