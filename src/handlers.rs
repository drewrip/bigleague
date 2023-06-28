use tera::{Tera, Context};
use serde::{Serialize, Deserialize};
use warp::{http::StatusCode, Filter, Reply, Rejection};
use std::sync::Arc;
use std::convert::Infallible;

use crate::db;


fn render(template: &str, ctx: Context, tera: Arc<Tera>) -> impl Reply {
    let render = tera.render(template, &ctx).unwrap();
    warp::reply::html(render)
}

// GET /league/<id>
pub async fn league_handler(id: String, db_pool: db::DBPool, tera: Arc<Tera>) -> std::result::Result<impl Reply, Rejection> { 
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
    let mut ctx = Context::new();
    ctx.insert("league", &league);
    Ok(render("league.html", ctx, tera)) 
}

pub async fn user_handler(id: String, db_pool: db::DBPool, tera: Arc<Tera>) -> std::result::Result<impl Reply, Rejection> {
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
    };

    let mut ctx = Context::new();
    ctx.insert("user", &user);
    ctx.insert("roster", &roster);
    Ok(render("user.html", ctx, tera))
}

pub async fn standings_handler(db_pool: db::DBPool, tera: Arc<Tera>) -> std::result::Result<impl Reply, Rejection> {


    let db = db::get_db_con(&db_pool)
            .await;

    let rows = db.query("SELECT * FROM users, rosters WHERE users.id = rosters.user_id ORDER BY wins DESC, fpts DESC, fpts_decimal DESC, fpts_against DESC, fpts_decimal DESC", &[])
        .await
        .unwrap();

    let standings: Vec<db::Standing> = rows.into_iter()
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
            };
            
            db::Standing {
                user,
                roster,
            }
        })
        .collect();

    let mut ctx = Context::new();
    ctx.insert("standings", &standings);
    Ok(render("standings.html", ctx, tera))
}

pub async fn health_handler(db_pool: db::DBPool) -> std::result::Result<impl Reply, Rejection> {
    let db = db::get_db_con(&db_pool)
            .await;

    db.execute("SELECT 1", &[])
            .await
            .unwrap();
    Ok(StatusCode::OK)
}

pub async fn not_found_handler(tera: Arc<Tera>) -> std::result::Result<impl Reply, Rejection> {
   let ctx = Context::new();
   Ok(render("notfound.html", ctx, tera)) 
}
