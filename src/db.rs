use warp::Filter;
use mobc::{Connection, Pool};
use mobc_postgres::{tokio_postgres, PgConnectionManager};
use tokio_postgres::{Config, Error, NoTls};
use std::str::FromStr;
use std::time::Duration;
use std::convert::Infallible;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::collections::HashMap;
use log::{info, error, trace};

use crate::config;

pub type DBCon = Connection<PgConnectionManager<NoTls>>;
pub type DBPool = Pool<PgConnectionManager<NoTls>>;


#[derive(Serialize, Deserialize, Debug)]
pub struct Standing {
    pub user: User,
    pub roster: Roster,
    pub league: League,
    pub rank: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct League {
    pub id: String,
    pub name: String,
    pub avatar: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Roster {
    pub user_id: String,
    pub league_id: String,
    pub wins: i32,
    pub losses: i32,
    pub ties: i32,
    pub fpts: i32,
    pub fpts_decimal: i32,
    pub fpts_against: i32,
    pub fpts_against_decimal: i32,
    pub roster_id: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct User {
    pub id: String,
    pub name: String,
    pub avatar: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub team: String,
    pub position: String,
    pub status: String,
    pub starter: i32,
    pub points: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ownership {
    pub user_id: String,
    pub league_id: String,
    pub player_id: String,
    pub starter: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub season: i32,
    pub week: i32,
    pub league_season: i32,
    pub display_week: i32,
    pub season_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Matchup {
    pub season: i32,
    pub week: i32,
    pub league_id: String,
    pub user_id: String,
    pub opponent_id: String,
    pub points: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Score {
    pub player_id: String,
    pub league_id: String,
    pub season: i32,
    pub week: i32,
    pub points: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Week {
    pub league_id: String,
    pub season: i32,
    pub week: i32,
    pub user_id: String,
    pub user_name: String,
    pub user_avatar: String,
    pub user_points: f32,
    pub opponent_id: String,
    pub opponent_name: String,
    pub opponent_avatar: String,
    pub opponent_points: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Bracket {
    pub num_teams: usize,
    pub start_week: i32,
    pub champ_week: i32,
    pub stages: Vec<Vec<PlayoffTeam>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PlayoffTeam {
    pub week: i32,
    pub rank: i64,
    pub user: User,
    pub points: f32,
}

pub async fn get_db_con(db_pool: &DBPool) -> DBCon {
    db_pool.get().await.unwrap()
}

pub fn with_db(db_pool: Arc<DBPool>) -> impl Filter<Extract = (Arc<DBPool>,), Error = Infallible> + Clone {
    warp::any().map(move || db_pool.clone())
}

pub fn create_pool(bl_config: config::Config) -> std::result::Result<DBPool, mobc::Error<Error>> {
    
    info!("creating database pool");

    let pg_config = Config::from_str(
        &format!("host={} port={} user={} password={} dbname={}",
            bl_config.database.host,
            bl_config.database.port.unwrap_or(5432),
            bl_config.database.user,
            bl_config.database.password,
            bl_config.database.dbname,
        )
    )?;

    let manager = PgConnectionManager::new(pg_config, NoTls);
    Ok(Pool::builder()
            .max_open(bl_config.database.max_open)
            .max_idle(bl_config.database.max_idle)
            .get_timeout(Some(Duration::from_secs(bl_config.database.timeout)))
            .build(manager))
}

pub async fn create_tables(db_pool: Arc<DBPool>) -> Result<(), tokio_postgres::Error>{

    info!("creating tables");

    let con = get_db_con(&db_pool).await;

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


    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS users (
            id varchar(64) PRIMARY KEY,
            name varchar(64) NOT NULL,
            avatar varchar(64)
        )
        "
    ).await.unwrap();
    
    // Create table for users in the big league
    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS rosters (
            user_id varchar(64) PRIMARY KEY,
            league_id varchar(64),
            wins integer NOT NULL,
            losses integer NOT NULL,
            ties integer NOT NULL,
            fpts integer NOT NULL,
            fpts_decimal integer NOT NULL,
            fpts_against integer NOT NULL,
            fpts_against_decimal integer NOT NULL,
            roster_id integer NOT NULL
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS players (
           id varchar(64) PRIMARY KEY,
           first_name varchar(64) NOT NULL,
           last_name varchar(64) NOT NULL,
           team varchar(64),
           position varchar(64),
           status varchar(64)
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS ownership (
            user_id varchar(64) NOT NULL,
            league_id varchar(64) NOT NULL,
            player_id varchar(64) NOT NULL,
            starter integer,
            PRIMARY KEY (user_id, league_id, player_id)
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS state (
            season integer NOT NULL,
            week integer NOT NULL,
            league_season integer NOT NULL,
            display_week integer NOT NULL,
            season_type varchar(64) NOT NULL,
            PRIMARY KEY (season, week)
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS matchups (
            season integer NOT NULL,
            week integer NOT NULL,
            league_id varchar(64) NOT NULL,
            user_id varchar(64) NOT NULL,
            opponent_id varchar(64) NOT NULL,
            points real NOT NULL,
            PRIMARY KEY (season, week, league_id, user_id, opponent_id)
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS scores (
            player_id varchar(64) NOT NULL,
            league_id varchar(64) NOT NULL,
            season integer NOT NULL,
            week integer NOT NULL,
            points real NOT NULL,
            PRIMARY KEY (player_id, league_id, season, week)
        )
        "
    ).await.unwrap();

    con.batch_execute(
        "
        CREATE OR REPLACE VIEW ranks AS
            SELECT user_id, ROW_NUMBER() OVER (ORDER BY wins DESC, fpts DESC, fpts_decimal DESC, fpts_against DESC, fpts_against_decimal DESC) as rank
            FROM rosters, users 
            WHERE rosters.user_id = users.id
        "
    ).await.unwrap();
    Ok(())
}

pub async fn get_num_leagues(con: DBCon) -> Result<i64, tokio_postgres::Error> {
    Ok(
        con.query(
            "SELECT count(*) FROM leagues",
            &[])
        .await?[0]
        .get::<usize, i64>(0))
}

pub async fn get_time_period(con: &DBCon) -> Result<(i32, i32), tokio_postgres::Error> {
    let time = &con.query("
            SELECT SEASON,
                WEEK
            FROM STATE
            ORDER BY SEASON DESC,
                WEEK DESC
            LIMIT 1
              ",
              &[])
        .await?[0];
    Ok((time.get("season"), time.get("week")))
}

pub fn resolve_bracket(initial_round: Vec<PlayoffTeam>, start_week: i32, end_week: i32, weeks: HashMap<(i32, i64), f32>) -> Option<Vec<Vec<PlayoffTeam>>> {
    let mut bracket = vec![initial_round.clone()];
    let mut curr_round = initial_round.clone();
    for r in start_week..end_week {
        let next_round: Vec<PlayoffTeam> = curr_round
            .chunks_exact(2)
            .map(|matchup| {
                let team1_pts = weeks.get(&(r, matchup[0].rank))?;
                let team2_pts = weeks.get(&(r, matchup[1].rank))?;
                trace!(
                    "team1 ({:?}) = {}  vs  team2 ({:?}) = {}",
                    matchup[0].user.name,
                    team1_pts,
                    matchup[1].user.name,
                    team2_pts,
                    );
                if team1_pts > team2_pts {
                    trace!("team1 ({:?}) wins!", matchup[0].user.name);
                    Some(
                        PlayoffTeam {
                            week: r+1,
                            points: *weeks.get(&(r+1, matchup[0].rank)).unwrap_or(&0.0),
                            ..matchup[0].clone()
                        }
                    )
                } else {
                    trace!("team2 ({:?}) wins!", matchup[1].user.name);
                    Some(
                        PlayoffTeam {
                            week: r+1,
                            points: *weeks.get(&(r+1, matchup[1].rank)).unwrap_or(&0.0),
                            ..matchup[1].clone()
                        }
                    )
                }
            })
            .collect::<Option<Vec<PlayoffTeam>>>()?;

        bracket.push(next_round.clone());
        curr_round = next_round;
    }

    Some(bracket)
}

pub async fn get_bracket(con: DBCon, config: config::Config) -> Result<Bracket, tokio_postgres::Error>{

    let start_week = config.bigleague.playoffs_start_week;
    let champ_week = config.bigleague.playoffs_championship_week;
    let bids = config.bigleague.playoffs_at_large.unwrap().bids;

    let (curr_season, curr_week) = get_time_period(&con).await?;

    let possible_user_weeks: Vec<PlayoffTeam> = con.query("
            SELECT WEEK,
                RANKS.RANK as RANK,
                USERS.ID,
                USERS.NAME,
                USERS.AVATAR,
                POINTS
            FROM MATCHUPS,
                RANKS,
                USERS
            WHERE WEEK >= $1
                AND SEASON = $2
                AND RANKS.USER_ID = MATCHUPS.USER_ID
                AND RANKS.USER_ID = USERS.ID
                AND RANKS.RANK <= $3
            ORDER BY WEEK ASC, RANK ASC;
              ",
              &[&curr_week, &curr_season, &bids])
        .await?
        .into_iter()
        .map(|row| {
            PlayoffTeam {
                week: row.get("week"),
                rank: row.get("rank"),
                user: User {
                    id: row.get("id"),
                    name: row.get("name"),
                    avatar: row.get("avatar"),
                },
                points: row.get("points"),
            }
        })
        .collect();

    // week_rank: (week, rank) -> points
    let week_rank: HashMap<(i32, i64), f32> = possible_user_weeks
        .clone()
        .into_iter()
        .map(|user_week| ((user_week.week, user_week.rank), user_week.points))
        .collect();

    let base: Vec<PlayoffTeam> = possible_user_weeks
        .into_iter()
        .filter(|team| team.week == start_week)
        .collect();

    let top_half = base
        .clone()
        .into_iter()
        .take((bids as usize)/2);

    let bottom_half = base
        .clone()
        .into_iter()
        .skip((bids as usize)/2)
        .take((bids as usize)/2)
        .rev();

    let matched: Vec<PlayoffTeam> = top_half
        .zip(bottom_half)
        .map(|m| vec![m.0, m.1])
        .flatten()
        .collect();

    let stages: Vec<Vec<PlayoffTeam>> = 
        match resolve_bracket(
            matched,
            start_week,
            if curr_week < start_week { start_week } else { curr_week },
            week_rank
        ) {
            Some(s) => s,
            None => {
                error!("Couldn't resolve playoff bracket");
                vec![]
            },
        };

    Ok(
        Bracket {
            num_teams: base.len(),
            start_week,
            champ_week,
            stages,
        }
    )
}

#[cfg(test)]
mod tests {
    use crate::db;
    use std::collections::HashMap;

    #[test]
    fn test_resolve_small_bracket() {
        let team1 = db::PlayoffTeam {
            week: 0,
            rank: 1,
            user: db::User {
                id: "1".to_string(),
                name: "Todd".to_string(),
                avatar: "cafed00d".to_string(),
            },
            points: 100.0,
        };

        let team2 = db::PlayoffTeam {
            week: 0,
            rank: 2,
            user: db::User {
                id: "1".to_string(),
                name: "Eve".to_string(),
                avatar: "deadbeef".to_string(),
            },
            points: 99.0,
        };
        
        let matchups: HashMap<(i32, i64), f32> = HashMap::from([
            ((0, 1), 100.0),
            ((0, 2), 99.0),
        ]);

        let base = vec![team1.clone(), team2.clone()];
        let resolved_bracket = db::resolve_bracket(base, 0, 1, matchups).unwrap();

        assert_eq!(
            resolved_bracket.into_iter().last().unwrap(),
            vec![
                db::PlayoffTeam {
                    points: 0.0,
                    week: 1,
                    ..team1
                }
            ]
            );
    }
}
