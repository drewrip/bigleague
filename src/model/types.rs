use serde::{Serialize, Deserialize};

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

