use std::fmt::Display;

use anyhow::Error;
use nd_pdk::host::subsonicapi;

pub fn create_playlist<T>(
    user: &str,
    name: &str,
    songs: T,
    id: Option<&str>,
) -> Result<String, Error>
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
    subsonicapi::call(&uri)
}
