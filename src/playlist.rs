use std::fmt::Display;

use anyhow::{Error, anyhow};
use nd_pdk::{host::kvstore, types::Track};

use crate::subsonic::{self, SubsonicResponseData};

pub enum PlaylistKind {
    MyVibe,
}

impl Display for PlaylistKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MyVibe => write!(f, "My Vibe"),
        }
    }
}

pub struct Playlist {
    pub kind: PlaylistKind,
    pub owner: String,
}

impl Playlist {
    pub fn new(kind: PlaylistKind, owner: String) -> Self {
        Self { kind, owner }
    }

    fn get_cache_key(&self) -> String {
        format!("{}:{}", self.owner, self.kind)
    }

    pub fn save(&self, songs: &[String]) -> Result<(), Error> {
        let cache_key = &self.get_cache_key();
        let title = &self.kind.to_string();

        let maybe_id = kvstore::get(cache_key)?.and_then(|bytes| String::from_utf8(bytes).ok());
        let is_cached = maybe_id.is_some();

        let response = subsonic::create_playlist(&self.owner, title, songs, maybe_id)?;

        match response.response.data {
            SubsonicResponseData::Success(data) => {
                if !is_cached {
                    kvstore::set(cache_key, data.playlist.id.clone().into_bytes())?;
                }
                Ok(())
            }
            SubsonicResponseData::Failed(error) => {
                if error.error.code != 70 {
                    return Err(anyhow!(
                        "Failed to save {} playlist: {}",
                        title,
                        error.error.message
                    ));
                }
                let response = subsonic::create_playlist(&self.owner, title, songs, None::<&str>)?;
                match response.response.data {
                    SubsonicResponseData::Success(data) => {
                        kvstore::set(cache_key, data.playlist.id.clone().into_bytes())?;
                        Ok(())
                    }
                    SubsonicResponseData::Failed(error) => Err(anyhow!(
                        "Failed to re-create playlist {}: {}",
                        title,
                        error.error.message
                    )),
                }
            }
        }
    }
}

#[derive(Default)]
pub struct SongFilters {
    exclude_libraries: Vec<i32>,
    // Other Ideas:
    // include_missing: bool (default: false)
    // rating_gte: Option<i32>
    // avg_rating_gte: Option<f64>
    // exclude_tags: Vec<String>
    // exclude_genres: Vec<String>
}

impl SongFilters {
    pub fn exclude_libraries(mut self, libraries: impl IntoIterator<Item = i32>) -> Self {
        self.exclude_libraries.extend(libraries);
        self
    }

    pub fn matches(&self, track: &Track) -> bool {
        if self.exclude_libraries.contains(&track.library_id) {
            return false;
        }
        true
    }
}
