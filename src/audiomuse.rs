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
pub struct SongGenre {
    pub name: String,
    pub weight: f64,
}

#[derive(Default, Debug)]
pub struct SongMood {
    pub aggressive: f64,
    pub happy: f64,
    pub party: f64,
    pub relaxed: f64,
    pub sad: f64,
    pub danceable: f64,
}

#[derive(Debug)]
pub struct SongScore {
    pub mood: SongMood,
    pub genres: Vec<SongGenre>,
}

impl From<GetScoreResponse> for SongScore {
    fn from(value: GetScoreResponse) -> Self {
        let mut mood = SongMood::default();
        value
            .other_features
            .split(",")
            .map(|v| {
                v.split_once(":")
                    .map(|(n, w)| (n, w.parse::<f64>().unwrap()))
                    .unwrap()
            })
            .for_each(|(n, w)| match n {
                "danceable" => mood.danceable = w,
                "aggressive" => mood.aggressive = w,
                "happy" => mood.happy = w,
                "party" => mood.party = w,
                "relaxed" => mood.relaxed = w,
                "sad" => mood.sad = w,
                _ => (),
            });
        let genres = value
            .mood_vector
            .split(",")
            .map(|v| {
                v.split_once(":")
                    .map(|(n, w)| (n, w.parse::<f64>().unwrap()))
                    .unwrap()
            })
            .map(|(n, weight)| SongGenre {
                name: n.into(),
                weight,
            })
            .collect();
        Self { mood, genres }
    }
}

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
}
