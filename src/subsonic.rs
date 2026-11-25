use std::fmt::Display;

use anyhow::Error;
use nd_pdk::host::subsonicapi;
use serde::{Deserialize, Serialize};
use urlencoding::encode;

#[derive(Debug, Deserialize, Serialize)]
pub struct SubsonicResponse<T> {
    #[serde(rename = "subsonic-response")]
    pub response: SubsonicResponseBody<T>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsonicResponseBody<T> {
    pub status: String,
    pub version: String,
    pub r#type: String,
    pub server_version: String,
    pub open_subsonic: bool,

    #[serde(flatten)]
    pub data: SubsonicResponseData<T>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SubsonicResponseData<T> {
    Success(T),
    Failed(SubsonicError),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubsonicError {
    pub error: SubsonicErrorContent,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubsonicErrorContent {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePlaylistResponse {
    pub playlist: CreatePlaylistResponseContent,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePlaylistResponseContent {
    pub id: String,
}

pub fn create_playlist<T>(
    user: &str,
    name: &str,
    songs: T,
    id: Option<impl Display>,
) -> Result<SubsonicResponse<CreatePlaylistResponse>, Error>
where
    T: IntoIterator,
    T::Item: Display,
{
    let songs_param: String = songs
        .into_iter()
        .map(|id| format!("&songId={}", encode(&id.to_string())))
        .collect();
    let mut uri = format!(
        "rest/createPlaylist?u={}&name={}{}",
        encode(user),
        encode(name),
        songs_param
    );
    if let Some(id) = id {
        uri += &format!("&playlistId={}", encode(&id.to_string()));
    }
    serde_json::from_str(&subsonicapi::call(&uri)?).map_err(Into::into)
}
