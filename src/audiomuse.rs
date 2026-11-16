use serde::{Deserialize, Serialize};

pub struct Client {
    api_base: String,
    access_token: String,
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
    item_id: String,
}

impl Client {
    pub fn new(base: &str, token: &str) -> Self {
        Self {
            api_base: base.into(),
            access_token: token.into(),
        }
    }

    pub fn search_tracks(&self, query: &str) -> attohttpc::Result<Vec<SearchTrack>> {
        attohttpc::get(format!("{}/api/search_tracks", self.api_base))
            .param("search_query", query)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()?
            .json()
    }

    pub fn alchemy(&self, body: &AlchemyReqBody) -> attohttpc::Result<AlchemyResponse> {
        attohttpc::post(format!("{}/api/alchemy", self.api_base))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&body)?
            .send()?
            .json()
    }
}
