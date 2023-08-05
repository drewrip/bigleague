use std::collections::HashMap;
use log::{trace, error};

use crate::model::types::{User, PlayoffTeam, Bracket};
use crate::model::db::DBCon;

use crate::config;

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
    use crate::model::types::{PlayoffTeam, User};
    use crate::model::util;
    use std::collections::HashMap;

    #[test]
    fn test_resolve_small_bracket() {
        let team1 = PlayoffTeam {
            week: 0,
            rank: 1,
            user: User {
                id: "1".to_string(),
                name: "Todd".to_string(),
                avatar: "cafed00d".to_string(),
            },
            points: 100.0,
        };

        let team2 = PlayoffTeam {
            week: 0,
            rank: 2,
            user: User {
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
        let resolved_bracket = util::resolve_bracket(base, 0, 1, matchups).unwrap();

        assert_eq!(
            resolved_bracket.into_iter().last().unwrap(),
            vec![
                PlayoffTeam {
                    points: 0.0,
                    week: 1,
                    ..team1
                }
            ]
            );
    }
}
