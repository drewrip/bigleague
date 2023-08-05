use tera::{Tera, Context};
use warp::{Reply, Rejection};
use tokio_postgres::row::Row;
use std::sync::Arc;
use log::{info, error};

use crate::model::types::{League, User, Roster, Player, Bracket, Week, Standing};
use crate::model::db;
use crate::model::util;
use crate::config;

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

    let league = League {
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

    let current_period = db.query("SELECT season, week FROM state ORDER BY season DESC, week DESC LIMIT 1", &[])
        .await
        .unwrap();

    let season: i32 = current_period[0].get("season");
    let week: i32 = current_period[0].get("week");

    let rows = db.query("SELECT * FROM users, rosters WHERE users.id = $1 AND users.id = rosters.user_id", &[&id])
            .await
            .unwrap();

    let row = &rows[0];

    let user = User {
        id: row.get(0),
        name: row.get(1),
        avatar: row.get(2),
    };

    let roster = Roster {
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
        SELECT ID,
            FIRST_NAME,
            LAST_NAME,
            TEAM,
            POSITION,
            STATUS,
            STARTER,
            POINTS
        FROM PLAYERS,
            OWNERSHIP,
            SCORES
        WHERE PLAYERS.ID = OWNERSHIP.PLAYER_ID
            AND OWNERSHIP.USER_ID = $1
            AND SCORES.SEASON = $2
            AND SCORES.WEEK = $3
            AND SCORES.PLAYER_ID = PLAYERS.ID
            AND SCORES.LEAGUE_ID = $4
        ORDER BY STARTER DESC, (CASE POSITION
                        WHEN 'QB' THEN 1
                        WHEN 'RB' THEN 2
                        WHEN 'WR' THEN 3
                        WHEN 'TE' THEN 4
                        WHEN 'K' THEN 5
                        WHEN 'DEF' THEN 6
        END) ASC
        ",
        &[&id, &season, &week, &roster.league_id]
    ).await.unwrap();

    let players: Vec<Player> = player_rows.into_iter()
        .map(|player| {
            Player {
                id: player.get("id"),
                first_name: player.get("first_name"),
                last_name: player.get("last_name"),
                team: player.get("team"),
                position: player.get("position"),
                status: player.get("status"),
                starter: player.get("starter"),
                points: player.get("points"),
            }
        })
        .collect();

    let matchups_rows = db.query(
           "
            SELECT M1.WEEK,
                M1.USER_ID,
                U1.NAME AS USER_NAME,
                U1.AVATAR AS USER_AVATAR,
                M1.POINTS AS USER_POINTS,
                M1.OPPONENT_ID,
                U2.NAME AS OPPONENT_NAME,
                U2.AVATAR AS OPPONENT_AVATAR,
                M2.POINTS AS OPPONENT_POINTS
            FROM MATCHUPS AS M1,
                MATCHUPS AS M2,
                USERS AS U1,
                USERS AS U2
            WHERE M1.OPPONENT_ID = M2.USER_ID
                AND M1.WEEK = M2.WEEK
                AND U1.ID = M1.USER_ID
                AND U2.ID = M1.OPPONENT_ID
                AND M1.SEASON = $1
                AND M1.USER_ID = $2
           ",
           &[&season, &id]
        )
        .await
        .unwrap();

    let matchups: Vec<Week> = matchups_rows
        .iter()
        .map(|row| {
            Week {
                league_id: id.clone(),
                season: season,
                week: row.get("week"),
                user_id: row.get("user_id"),
                user_name: row.get("user_name"),
                user_avatar: row.get("user_avatar"),
                user_points: row.get("user_points"),
                opponent_id: row.get("opponent_id"),
                opponent_name: row.get("opponent_name"),
                opponent_avatar: row.get("opponent_avatar"),
                opponent_points: row.get("opponent_points"),
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

pub async fn standings_handler(db_pool: Arc<db::DBPool>, tera: Arc<Tera>, config: config::Config) -> std::result::Result<impl Reply, Rejection> {

    info!("GET /");

    let db = db::get_db_con(&db_pool)
            .await;

    let current_period = db.query("SELECT season, week FROM state ORDER BY season DESC, week DESC LIMIT 1", &[])
        .await
        .unwrap();

    let season: i32 = current_period[0].get("season");
    let week: i32 = current_period[0].get("week");

    let rows = db.query("SELECT * FROM users, rosters, leagues, ranks WHERE users.id = rosters.user_id AND leagues.id = rosters.league_id AND ranks.user_id = users.id", &[])
        .await
        .unwrap();

    let standings = collect_standings(rows);
    let bracket = match util::get_bracket(db, config).await {
        Ok(b) => b,
        Err(e) => {
            error!("Couldn't get bracket: {}", e);
            Bracket {
                num_teams: 0,
                start_week: 0,
                champ_week: 0,
                stages: vec![],
            }
        }
    };

    let mut ctx = Context::new();
    ctx.insert("standings", &standings);
    ctx.insert("bracket", &bracket);
    Ok(render("standings.html", ctx, tera))
}

fn collect_standings(rows: Vec<Row>) -> Vec<Standing> {
    rows.into_iter()
        .map(|row| {
            let user = User {
                id: row.get(0),
                name: row.get(1),
                avatar: row.get(2),
            };

            let roster = Roster {
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
          
            let league = League {
                id: row.get(13),
                name: row.get(14),
                avatar: row.get(15),
            };

            let rank: i64 = row.get(17);

            Standing {
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
