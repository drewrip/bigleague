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

    let rows = db.query("SELECT * FROM users WHERE id = $1", &[&id])
            .await
            .unwrap();


    let user = db::User {
        id: rows[0].get(0),
        wins: rows[0].get(1),
        losses: rows[0].get(2),
        ties: rows[0].get(3),
        fpts: rows[0].get(4),
        fpts_decimal: rows[0].get(5),
        fpts_against: rows[0].get(6),
        fpts_against_decimal: rows[0].get(7),
        league: rows[0].get(8),
        avatar: rows[0].get(9),
    };

    let mut ctx = Context::new();
    ctx.insert("user", &user);
    Ok(render("user.html", ctx, tera))
}

pub async fn standings_handler(db_pool: db::DBPool, tera: Arc<Tera>) -> std::result::Result<impl Reply, Rejection> {

    let standings = db::Standings {
        users: vec![],
    };

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
   let mut ctx = Context::new();
   Ok(render("notfound.html", ctx, tera)) 
}
