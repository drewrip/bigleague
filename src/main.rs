use tera::Tera;
use warp::Filter;
use std::sync::Arc;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr};
use log::{debug, error, info, warn};

mod db;
mod stats;
mod handlers;
mod config;

fn with_tera(tera: Arc<Tera>) -> impl Filter<Extract = (Arc<Tera>,), Error = Infallible> + Clone {
    warp::any().map(move || tera.clone())
}

#[tokio::main]
async fn main() {

    env_logger::init();

    let config: config::Config = config::read_config("Bigleague.toml").expect("Couldn't parse config file"); 

    let pool = Arc::new(db::create_pool(config.clone()).unwrap());
 
    db::create_tables(pool.clone()).await.unwrap();
    
    let stats_pool = pool.clone();
    let stats_config = config.clone();
    tokio::spawn(async move {
            let _ = stats::stats_loop(stats_config, stats_pool).await;
        }
    );

    let tera: Tera = Tera::new("templates/**/*").unwrap();
    let tera: Arc<Tera> = Arc::new(tera);

    let league_route = warp::path!("league" / String)
        .and(db::with_db(pool.clone()))
        .and(with_tera(tera.clone()))
        .and_then(handlers::league_handler);

    let user_route = warp::path!("user" / String)
        .and(db::with_db(pool.clone()))
        .and(with_tera(tera.clone()))
        .and_then(handlers::user_handler);

    let standings_route = warp::path::end()
        .and(db::with_db(pool.clone()))
        .and(with_tera(tera.clone()))
        .and_then(handlers::standings_handler);

    let not_found_route = warp::any()
        .and(with_tera(tera.clone()))
        .and_then(handlers::not_found_handler);

    let static_route = warp::path("static")
        .and(warp::fs::dir("static"));

    let routes = warp::get().and(
        league_route
            .or(user_route)
            .or(standings_route)
            .or(static_route)
            .or(not_found_route)
            .with(warp::cors().allow_any_origin())
    );

    let ip = match config.clone().web.ip.parse::<IpAddr>() {
        Ok(addr) => addr,
        Err(e) => {
            warn!("couldn't parse ip for warp server: {}, using 127.0.0.1", e);
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
        }
    };

    info!("starting warp server at {}:{}", ip, config.web.port);
    warp::serve(routes).run((ip, config.web.port)).await;
}
