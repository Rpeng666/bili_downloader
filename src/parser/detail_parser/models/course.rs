use serde_derive::Deserialize;

#[derive(Debug, Clone)]
pub struct CourseDownloadChoice {
    pub season_id: i64,
    pub selected_episodes: Vec<i64>, // 选中要下载的集数的 ep_id
}

#[derive(Debug, Deserialize)]
pub struct CourseInfo {
    pub title: String,
    pub season_id: i64,
    pub cover: String,
    pub paid_view: bool,
    pub episodes: Vec<CourseEpisode>,
}

#[derive(Debug, Deserialize)]
pub struct CourseEpisode {
    pub id: i64, // ep_id
    pub aid: i64,
    pub cid: i64,
    pub duration: i32,
    pub title: String,
    pub release_date: i64,
}
