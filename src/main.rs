use tera::Tera;
use warp::Filter;
use std::sync::Arc;
use std::convert::Infallible;

mod db;
mod stats;
mod handlers;
mod config;

fn with_tera(tera: Arc<Tera>) -> impl Filter<Extract = (Arc<Tera>,), Error = Infallible> + Clone {
    warp::any().map(move || tera.clone())
}

#[tokio::main]
async fn main() {

    let config: config::Config = config::read_config("Bigleague.toml").expect("Couldn't parse config file");
    let league_ids = config.bigleague.leagues;

    let pool = db::create_pool().unwrap();
    println!("Creating tables...");   
    db::create_tables(&pool).await.unwrap();

    for lid in league_ids {
        println!("Fetching league {}...", lid);
        stats::fetch_leagues(lid.clone(), &pool).await.unwrap();
        println!("Fetching rosters for league {}...", lid);
        stats::fetch_rosters(lid.clone(), &pool).await.unwrap();
        println!("Fetching users from league {}...", lid);
        stats::fetch_users(lid.clone(), &pool).await.unwrap();
    }
    println!("Fetching players...");
    // Be careful with fetch players in the future.
    // Temporarily reading from json file, but
    // the call to the Sleeper API is large
    // stats::fetch_players(&pool).await.unwrap();

    let mut tera: Tera = Tera::new("templates/**/*").unwrap();
    let tera: Arc<Tera> = Arc::new(tera);

    let health_route = warp::path!("health")
        .and(db::with_db(pool.clone()))
        .and_then(handlers::health_handler);

    let league_route = warp::path!("league" / String)
        .and(db::with_db(pool.clone()))
        .and(with_tera(tera.clone()))
        .and_then(handlers::league_handler);

    let user_route = warp::path!("user" / String)
        .and(db::with_db(pool.clone()))
        .and(with_tera(tera.clone()))
        .and_then(handlers::user_handler);

    let standings_route = warp::path!("standings")
        .and(db::with_db(pool.clone()))
        .and(with_tera(tera.clone()))
        .and_then(handlers::standings_handler);

    let not_found_route = warp::any()
        .and(with_tera(tera.clone()))
        .and_then(handlers::not_found_handler);

    let static_route = warp::path("static")
        .and(warp::fs::dir("static"));

    let routes = warp::get().and(
        health_route
            .or(league_route)
            .or(user_route)
            .or(standings_route)
            .or(static_route)
            .or(not_found_route)
            .with(warp::cors().allow_any_origin())
    );

    println!("Starting server...");
    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}
