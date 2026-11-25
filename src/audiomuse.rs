use std::{
    collections::HashMap,
    ops::{AddAssign, DivAssign, Mul},
};

use anyhow::{Error, anyhow};
use nd_pdk::host::http::{self, HTTPResponse};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use urlencoding::encode;

pub struct Client {
    api_base: String,
    access_token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiError {
    pub error: String,
    pub error_class: String,
    pub error_code: u32,
    pub error_message: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}): {}",
            self.error_class, self.error_code, self.error_message
        )
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug, Deserialize, Serialize)]
pub struct AlchemyReqBody {
    pub items: Vec<AlchemyReqItem>,
    pub n: u32,
    pub subtract_distance: f32,
    pub temperature: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AlchemyReqItem {
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
pub struct ScoreDataResponse {
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

#[derive(Debug, Deserialize, Serialize)]
pub struct MoodCentroidsResponse {
    pub aggressive: Vec<CentroidMetadata>,
    pub danceable: Vec<CentroidMetadata>,
    pub happy: Vec<CentroidMetadata>,
    pub party: Vec<CentroidMetadata>,
    pub relaxed: Vec<CentroidMetadata>,
    pub sad: Vec<CentroidMetadata>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetEmbeddingResponse {
    embedding: Vec<f64>,
    hyperbolic_radius: f64,
    item_id: String,
    poincare_embedding: Vec<f64>,
}

impl Client {
    pub fn new(api_base: String, access_token: Option<String>) -> Self {
        Self {
            api_base,
            access_token,
        }
    }

    fn headers(&self) -> HashMap<String, String> {
        let mut headers =
            HashMap::from([("Content-Type".to_owned(), "application/json".to_owned())]);
        if let Some(token) = &self.access_token {
            headers.insert("Authorization".to_owned(), format!("Bearer {}", token));
        }
        headers
    }

    fn send_http(&self, method: &str, url: &str, body: Vec<u8>) -> Result<HTTPResponse, Error> {
        let request = http::HTTPRequest {
            method: method.to_owned(),
            url: url.to_owned(),
            headers: self.headers(),
            body,
            ..Default::default()
        };
        http::send(request)?.ok_or_else(|| anyhow!("audiomuse: empty response"))
    }

    fn parse_response<T: DeserializeOwned>(res: HTTPResponse) -> Result<T, Error> {
        if (200..300).contains(&res.status_code) {
            serde_json::from_slice(&res.body).map_err(Into::into)
        } else {
            serde_json::from_slice::<ApiError>(&res.body)
                .map_err(Into::<Error>::into)
                .and_then(|err| Err(Error::new(err)))
        }
    }

    pub fn alchemy(&self, body: &AlchemyReqBody) -> Result<AlchemyResponse, Error> {
        let url = format!("{}/api/alchemy", self.api_base);
        let body = serde_json::to_vec(body)?;
        let res = self.send_http("POST", &url, body)?;
        Self::parse_response(res)
    }

    pub fn get_score(&self, id: &str) -> Result<ScoreDataResponse, Error> {
        let url = format!("{}/external/get_score?id={}", self.api_base, encode(id));
        let res = self.send_http("GET", &url, Default::default())?;
        Self::parse_response(res)
    }

    pub fn mood_centroids(&self) -> Result<MoodCentroidsResponse, Error> {
        let url = format!("{}/api/mood_centroids", self.api_base);
        let res = self.send_http("GET", &url, Default::default())?;
        Self::parse_response(res)
    }

    pub fn get_embedding(&self, id: &str) -> Result<GetEmbeddingResponse, Error> {
        let url = format!("{}/external/get_embedding?id={}", self.api_base, encode(id));
        let res = self.send_http("GET", &url, Default::default())?;
        Self::parse_response(res)
    }
}

#[derive(Debug)]
pub struct SongScore {
    pub mood: SongScoreMood,
    pub genres: HashMap<String, f64>,
}

impl From<ScoreDataResponse> for SongScore {
    fn from(value: ScoreDataResponse) -> Self {
        let mut mood = SongScoreMood::default();
        value
            .other_features
            .split(",")
            .filter_map(|entry| entry.split_once(":"))
            .filter_map(|(k, v)| v.parse::<f64>().ok().map(|v| (k, v)))
            .for_each(|(k, v)| match k {
                "aggressive" => mood.aggressive = v,
                "danceable" => mood.danceable = v,
                "happy" => mood.happy = v,
                "party" => mood.party = v,
                "relaxed" => mood.relaxed = v,
                "sad" => mood.sad = v,
                _ => unreachable!("There are only 6 moods"),
            });
        let genres = value
            .mood_vector
            .split(",")
            .filter_map(|entry| entry.split_once(":"))
            .filter_map(|(k, v)| v.parse::<f64>().ok().map(|v| (k.into(), v)))
            .collect();
        Self { mood, genres }
    }
}

#[derive(Debug, Default)]
pub struct SongScoreMood {
    pub aggressive: f64,
    pub danceable: f64,
    pub happy: f64,
    pub party: f64,
    pub relaxed: f64,
    pub sad: f64,
}

impl AddAssign for SongScoreMood {
    fn add_assign(&mut self, rhs: Self) {
        self.aggressive += rhs.aggressive;
        self.danceable += rhs.danceable;
        self.happy += rhs.happy;
        self.party += rhs.party;
        self.relaxed += rhs.relaxed;
        self.sad += rhs.sad;
    }
}

impl DivAssign<f64> for SongScoreMood {
    fn div_assign(&mut self, rhs: f64) {
        self.aggressive /= rhs;
        self.danceable /= rhs;
        self.happy /= rhs;
        self.party /= rhs;
        self.relaxed /= rhs;
        self.sad /= rhs;
    }
}

impl Mul<f64> for SongScoreMood {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            aggressive: self.aggressive * rhs,
            danceable: self.danceable * rhs,
            happy: self.happy * rhs,
            party: self.party * rhs,
            relaxed: self.relaxed * rhs,
            sad: self.sad * rhs,
        }
    }
}
