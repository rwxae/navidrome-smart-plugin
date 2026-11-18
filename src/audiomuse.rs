use std::collections::HashMap;

use attohttpc::Session;
use serde::{Deserialize, Serialize};

pub struct Client {
    api_base: String,
    session: Session,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchTrackResponse {
    pub album: String,
    pub album_artist: String,
    pub author: String,
    pub item_id: String,
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AlchemyReqBody {
    pub items: Vec<AlchemyItem>,
    pub n: u32,
    pub subtract_distance: f32,
    pub temperature: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AlchemyItem {
    pub id: String,
    pub op: String,
    pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AlchemyResponse {
    pub results: Vec<AlchemyResponseResult>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AlchemyResponseResult {
    pub item_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetScoreResponse {
    mood_vector: String,
    other_features: String,
}

#[derive(Debug)]
pub struct SongScore {
    pub mood: HashMap<String, f64>,
    pub genres: HashMap<String, f64>,
}

impl From<GetScoreResponse> for SongScore {
    fn from(value: GetScoreResponse) -> Self {
        let mood = value
            .other_features
            .split(",")
            .filter_map(|v| {
                v.split_once(":")
                    .and_then(|(n, w)| w.parse::<f64>().ok().map(|v| (n.into(), v)))
            })
            .collect();
        let genres = value
            .other_features
            .split(",")
            .filter_map(|v| {
                v.split_once(":")
                    .and_then(|(n, w)| w.parse::<f64>().ok().map(|v| (n.into(), v)))
            })
            .collect();
        Self { mood, genres }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CentroidMetadata {
    pub cluster_id: u32,
    pub index: u32,
    pub mood_score: f64,
    pub n_songs: u32,
    pub top_tags: Vec<String>,
}

pub type MoodCentroidsResponse = HashMap<String, Vec<CentroidMetadata>>;

impl Client {
    pub fn new(url: &str, token: &str) -> Self {
        let mut session = Session::new();

        session.header("Authorization", format!("Bearer {}", token));
        session.header("Content-Type", "application/json");

        Self {
            api_base: url.into(),
            session,
        }
    }

    pub fn search_tracks(&self, query: &str) -> attohttpc::Result<Vec<SearchTrackResponse>> {
        self.session
            .get(format!("{}/api/search_tracks", self.api_base))
            .param("search_query", query)
            .send()?
            .json()
    }

    pub fn search_track(&self, query: &str) -> attohttpc::Result<Option<SearchTrackResponse>> {
        Ok(self.search_tracks(query)?.into_iter().next())
    }

    pub fn alchemy(&self, body: &AlchemyReqBody) -> attohttpc::Result<AlchemyResponse> {
        self.session
            .post(format!("{}/api/alchemy", self.api_base))
            .json(&body)?
            .send()?
            .json()
    }

    pub fn get_score(&self, id: &str) -> attohttpc::Result<GetScoreResponse> {
        self.session
            .get(format!("{}/external/get_score", self.api_base))
            .param("id", id)
            .send()?
            .json()
    }

    pub fn mood_centroids(&self, mood: Option<&str>) -> attohttpc::Result<MoodCentroidsResponse> {
        let mut request = self
            .session
            .get(format!("{}/api/mood_centroids", self.api_base));

        if let Some(mood) = mood {
            request = request.param("mood", mood);
        }

        request.send()?.json()
    }
}
