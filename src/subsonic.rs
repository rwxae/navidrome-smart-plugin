use std::fmt::Display;

use anyhow::Error;
use nd_pdk::host::subsonicapi;

pub struct Client {
    user: String,
}

impl Client {
    pub fn new(user: &str) -> Self {
        Self { user: user.into() }
    }

    pub fn create_playlist<T>(
        &self,
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
            "rest/createPlaylist?user={}&name={}{}",
            self.user, name, songs_param
        );
        if let Some(id) = id {
            uri += &format!("&playlistId={}", id);
        }
        subsonicapi::call(&uri)
    }
}
