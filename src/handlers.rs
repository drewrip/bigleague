use tera::{Tera, Context};
use warp::{Reply, Rejection};
use tokio_postgres::row::Row;
use std::sync::Arc;
use log::info;

use crate::db;

fn render(template: &str, ctx: Context, tera: Arc<Tera>) -> impl Reply {
    let render = tera.render(template, &ctx).unwrap();
    warp::reply::html(render)
}

pub async fn league_handler(id: String, db_pool: Arc<db::DBPool>, tera: Arc<Tera>) -> std::result::Result<impl Reply, Rejection> {

    info!("GET /league/{}", id);

    let db = db::get_db_con(&db_pool)
            .await;

    let idstr = id.to_string();
    let rows = db.query("SELECT * FROM leagues WHERE id = $1", &[&idstr])
            .await
            .unwrap();

    let league = db::League {
        id: rows[0].get(0),
        name: rows[0].get(1),
        avatar: rows[0].get(2),
    };

    let rows = db.query("SELECT * FROM users, rosters, leagues, ranks WHERE users.id = rosters.user_id AND leagues.id = rosters.league_id AND ranks.user_id = users.id AND leagues.id = $1 ORDER BY ranks.rank ASC", &[&idstr])
        .await
        .unwrap();

    let standings = collect_standings(rows);

    let mut ctx = Context::new();
    ctx.insert("league", &league);
    ctx.insert("standings", &standings);
    Ok(render("league.html", ctx, tera)) 
}

pub async fn user_handler(id: String, db_pool: Arc<db::DBPool>, tera: Arc<Tera>) -> std::result::Result<impl Reply, Rejection> {

    info!("GET /user/{}", id);

    let db = db::get_db_con(&db_pool)
            .await;

    let rows = db.query("SELECT * FROM users, rosters WHERE users.id = $1 AND users.id = rosters.user_id", &[&id])
            .await
            .unwrap();

    let row = &rows[0];

    let user = db::User {
        id: row.get(0),
        name: row.get(1),
        avatar: row.get(2),
    };

    let roster = db::Roster {
        user_id: row.get(3),
        league_id: row.get(4),
        wins: row.get(5),
        losses: row.get(6),
        ties: row.get(7),
        fpts: row.get(8),
        fpts_decimal: row.get(9),
        fpts_against: row.get(10),
        fpts_against_decimal: row.get(11),
        roster_id: row.get(12),
    };

    let player_rows = db.query(
        "
        SELECT * FROM players, ownership
        WHERE players.id = ownership.player_id AND ownership.user_id = $1 
        ORDER BY starter DESC,
        (CASE position
            WHEN 'QB' THEN 1 
            WHEN 'RB' THEN 2
            WHEN 'WR' THEN 3
            WHEN 'TE' THEN 4
            WHEN 'K' THEN 5
            WHEN 'DEF' THEN 6
            END) ASC
        ",
        &[&id]
    ).await.unwrap();

    let players: Vec<db::Player> = player_rows.into_iter()
        .map(|player| {
            db::Player {
                id: player.get(0),
                first_name: player.get(1),
                last_name: player.get(2),
                team: player.get(3),
                position: player.get(4),
                status: player.get(5),
                starter: player.get(9),
            }
        })
        .collect();

    let season: i32 = db.query("SELECT season FROM state ORDER BY season DESC, week DESC LIMIT 1", &[])
        .await
        .unwrap()[0].get(0);

    let matchups_rows = db.query(
           "
           SELECT M1.league_id, M1.week, M1.user_id as home_user, M1.points as home_points, M2.user_id as away_user, M2.points as away_points
           FROM matchups M1, matchups M2
           WHERE M1.user_id = M2.opponent_id AND M1.opponent_id = M2.user_id AND M1.league_id = M2.league_id AND M1.season = $1 AND M1.user_id = $2
           ",
           &[&season, &id]
        )
        .await
        .unwrap();

    let matchups: Vec<db::Week> = matchups_rows
        .iter()
        .map(|row| {
            db::Week {
                league_id: id.clone(),
                season: season,
                week: row.get(1),
                home_user: row.get(2),
                home_points: row.get(3),
                away_user: row.get(4),
                away_points: row.get(5),
            } 
        })
        .collect();

    let mut ctx = Context::new();
    ctx.insert("user", &user);
    ctx.insert("matchups", &matchups);
    ctx.insert("roster", &roster);
    ctx.insert("players", &players);
    Ok(render("user.html", ctx, tera))
}

pub async fn standings_handler(db_pool: Arc<db::DBPool>, tera: Arc<Tera>) -> std::result::Result<impl Reply, Rejection> {

    info!("GET /");

    let db = db::get_db_con(&db_pool)
            .await;

    let rows = db.query("SELECT * FROM users, rosters, leagues, ranks WHERE users.id = rosters.user_id AND leagues.id = rosters.league_id AND ranks.user_id = users.id", &[])
        .await
        .unwrap();

    let standings = collect_standings(rows);

    let mut ctx = Context::new();
    ctx.insert("standings", &standings);
    Ok(render("standings.html", ctx, tera))
}

fn collect_standings(rows: Vec<Row>) -> Vec<db::Standing> {
    rows.into_iter()
        .map(|row| {
            let user = db::User {
                id: row.get(0),
                name: row.get(1),
                avatar: row.get(2),
            };

            let roster = db::Roster {
                user_id: row.get(3),
                league_id: row.get(4),
                wins: row.get(5),
                losses: row.get(6),
                ties: row.get(7),
                fpts: row.get(8),
                fpts_decimal: row.get(9),
                fpts_against: row.get(10),
                fpts_against_decimal: row.get(11),
                roster_id: row.get(12),
            };
          
            let league = db::League {
                id: row.get(13),
                name: row.get(14),
                avatar: row.get(15),
            };

            let rank: i64 = row.get(17);

            db::Standing {
                user,
                roster,
                league,
                rank,
            }
        })
        .collect()
}

pub async fn not_found_handler(tera: Arc<Tera>) -> std::result::Result<impl Reply, Rejection> {

    info!("GET unknown endpoint");

    let ctx = Context::new();
    Ok(render("notfound.html", ctx, tera)) 
}
