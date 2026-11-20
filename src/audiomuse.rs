use std::collections::HashMap;

use anyhow::Error;
use nd_pdk::host::http::{self, HTTPResponse};
use serde::{Deserialize, Serialize};

pub struct Client {
    api_base: String,
    access_token: String,
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
    pub results: Vec<AlchemyResponseItem>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AlchemyResponseItem {
    pub item_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetScoreResponse {
    mood_vector: String,
    other_features: String,
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
    pub fn new(url: &str, access_token: &str) -> Self {
        Self {
            api_base: url.to_owned(),
            access_token: access_token.to_owned(),
        }
    }

    fn headers(&self) -> HashMap<String, String> {
        HashMap::from([
            (
                "Authorization".to_owned(),
                format!("Bearer {}", self.access_token),
            ),
            ("Content-Type".to_owned(), "application/json".to_owned()),
        ])
    }

    fn send_http(&self, method: &str, url: &str, body: Vec<u8>) -> Result<HTTPResponse, Error> {
        let request = http::HTTPRequest {
            method: method.to_owned(),
            url: url.to_owned(),
            headers: self.headers(),
            body,
            ..Default::default()
        };
        http::send(request)?.ok_or_else(|| Error::msg("audiomuse: empty response"))
    }

    pub fn search_tracks(&self, query: &str) -> Result<Vec<SearchTrackResponse>, Error> {
        let url = format!("{}/api/search_tracks?search_query={}", self.api_base, query);
        self.send_http("GET", &url, Default::default())
            .and_then(|res| serde_json::from_slice(&res.body).map_err(Into::into))
    }

    pub fn search_track(&self, query: &str) -> Result<Option<SearchTrackResponse>, Error> {
        Ok(self.search_tracks(query)?.into_iter().next())
    }

    pub fn alchemy(&self, body: &AlchemyReqBody) -> Result<AlchemyResponse, Error> {
        let url = format!("{}/api/alchemy", self.api_base);
        self.send_http("POST", &url, serde_json::to_vec(body).unwrap())
            .and_then(|res| serde_json::from_slice(&res.body).map_err(Into::into))
    }

    pub fn get_score(&self, id: &str) -> Result<GetScoreResponse, Error> {
        let url = format!("{}/external/get_score?id={}", self.api_base, id);
        self.send_http("GET", &url, Default::default())
            .and_then(|res| serde_json::from_slice(&res.body).map_err(Into::into))
    }

    pub fn mood_centroids(&self) -> Result<MoodCentroidsResponse, Error> {
        let url = format!("{}/api/mood_centroids", self.api_base);
        self.send_http("GET", &url, Default::default())
            .and_then(|res| serde_json::from_slice(&res.body).map_err(Into::into))
    }
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
            .mood_vector
            .split(",")
            .filter_map(|v| {
                v.split_once(":")
                    .and_then(|(n, w)| w.parse::<f64>().ok().map(|v| (n.into(), v)))
            })
            .collect();
        Self { mood, genres }
    }
}
