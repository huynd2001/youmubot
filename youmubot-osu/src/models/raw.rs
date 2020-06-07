use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(crate) struct Beatmap {
    pub approved: String,
    pub submit_date: String,
    pub approved_date: Option<String>,
    pub last_update: String,
    pub artist: String,
    pub beatmap_id: String,
    pub beatmapset_id: String,
    pub bpm: String,
    pub creator: String,
    pub creator_id: String,
    pub difficultyrating: String,
    pub diff_aim: Option<String>,
    pub diff_speed: Option<String>,
    pub diff_size: String,
    pub diff_overall: String,
    pub diff_approach: String,
    pub diff_drain: String,
    pub hit_length: String,
    pub source: Option<String>,
    pub genre_id: String,
    pub language_id: String,
    pub title: String,
    pub total_length: String,
    pub version: String,
    pub file_md5: String,
    pub mode: String,
    pub tags: String,
    pub favourite_count: String,
    pub rating: String,
    pub playcount: String,
    pub passcount: String,
    pub count_normal: String,
    pub count_slider: String,
    pub count_spinner: String,
    pub max_combo: Option<String>,
    pub download_unavailable: String,
    pub audio_unavailable: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct User {
    pub user_id: String,
    pub username: String,
    pub join_date: String,
    pub country: String,
    pub count300: Option<String>,
    pub count100: Option<String>,
    pub count50: Option<String>,
    pub playcount: Option<String>,
    pub ranked_score: Option<String>,
    pub total_score: Option<String>,
    pub pp_rank: Option<String>,
    pub level: Option<String>,
    pub pp_raw: Option<String>,
    pub accuracy: Option<String>,
    pub count_rank_ss: Option<String>,
    pub count_rank_ssh: Option<String>,
    pub count_rank_s: Option<String>,
    pub count_rank_sh: Option<String>,
    pub count_rank_a: Option<String>,
    pub total_seconds_played: Option<String>,
    pub pp_country_rank: Option<String>,
    pub events: Vec<UserEvent>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UserEvent {
    pub display_html: String,
    pub beatmap_id: Option<String>,
    pub beatmapset_id: Option<String>,
    pub date: String,
    pub epicfactor: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Score {
    pub score_id: Option<String>,
    pub beatmap_id: Option<String>,
    pub score: String,
    pub count300: String,
    pub count100: String,
    pub count50: String,
    pub countmiss: String,
    pub maxcombo: String,
    pub countkatu: String,
    pub countgeki: String,
    pub perfect: String,
    pub enabled_mods: String,
    pub user_id: String,
    pub date: String,
    pub rank: String,
    pub pp: Option<String>,
    pub replay_available: Option<String>,
}
