use attohttpc::Session;
use serde::{Deserialize, Serialize};

pub struct Client {
    api_base: String,
    session: Session,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchTrack {
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
    pub results: Vec<AlchemyResult>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AlchemyResult {
    pub item_id: String,
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

    pub fn search_tracks(&self, query: &str) -> attohttpc::Result<Vec<SearchTrack>> {
        self.session
            .get(format!("{}/api/search_tracks", self.api_base))
            .param("search_query", query)
            .send()?
            .json()
    }

    pub fn alchemy(&self, body: &AlchemyReqBody) -> attohttpc::Result<AlchemyResponse> {
        self.session
            .post(format!("{}/api/alchemy", self.api_base))
            .json(&body)?
            .send()?
            .json()
    }
}
