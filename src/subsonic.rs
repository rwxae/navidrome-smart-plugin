use anyhow::Error;
use nd_pdk::host::subsonicapi;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Serialize, Deserialize)]
pub struct SubsonicResponse<T> {
    #[serde(rename = "subsonic-response")]
    pub response: SubsonicResponseBody<T>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsonicResponseBody<T> {
    pub status: String,
    pub version: String,
    pub r#type: String,
    pub server_version: String,
    pub open_subsonic: bool,

    pub error: Option<SubsonicError>,

    #[serde(flatten)]
    pub data: Option<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubsonicError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePlaylistResponse {
    pub playlist: Playlist,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
}

pub fn create_playlist<T>(
    user: &str,
    name: &str,
    songs: T,
    id: Option<&str>,
) -> Result<SubsonicResponse<CreatePlaylistResponse>, Error>
where
    T: IntoIterator,
    T::Item: Display,
{
    let songs_param: String = songs
        .into_iter()
        .map(|id| format!("&songId={}", id))
        .collect();
    let mut uri = format!(
        "rest/createPlaylist?u={}&name={}{}",
        user, name, songs_param
    );
    if let Some(id) = id {
        uri += &format!("&playlistId={}", id);
    }
    serde_json::from_str(&subsonicapi::call(&uri)?).map_err(Into::into)
}
