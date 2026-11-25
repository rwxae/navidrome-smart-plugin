use std::fmt::Display;

use anyhow::{Error, anyhow};
use extism_pdk::info;
use nd_pdk::{
    host::{
        kvstore,
        matcher::{self, MatchOptions},
        scrobbleretriever::{self, ScrobbleOptions},
    },
    types::{SongRef, Track},
};

use crate::{
    analysis::guess_mood,
    audiomuse::{self, AlchemyItem, AlchemyReqBody},
    subsonic::{self, SubsonicResponseData},
};

const PLAYLIST_MAX_SIZE: u32 = 100;
const ALCHEMY_SUBSTRACT_DISTANCE: f32 = 0.2;
const ALCHEMY_TEMPERATURE: f32 = 0.4;

enum PlaylistKind {
    MyVibe,
}

impl Display for PlaylistKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MyVibe => write!(f, "My Vibe"),
        }
    }
}

struct Playlist {
    kind: PlaylistKind,
    owner: String,
}

impl Playlist {
    fn new(kind: PlaylistKind, owner: String) -> Self {
        Self { kind, owner }
    }

    fn get_cache_key(&self) -> String {
        format!("{}:{}", self.owner, self.kind)
    }

    fn save(&self, songs: &[String]) -> Result<(), Error> {
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

pub fn generate_my_vibe(
    am_client: &audiomuse::Client,
    user: &str,
    filters: &SongFilters,
) -> Result<(), Error> {
    let (scrobbles, _) = scrobbleretriever::get_scrobbles(
        user,
        ScrobbleOptions {
            descending: true,
            max_items: 15,
            ..Default::default()
        },
    )?;

    let items: Vec<_> = guess_mood(&scrobbles, am_client)?
        .into_iter()
        .inspect(|(mood, cluster)| info!("Seeding: mood:{mood}, cluster:{cluster}"))
        .map(|(mood, cluster)| AlchemyItem {
            id: format!("{mood}:{cluster}"),
            op: "ADD".into(),
            r#type: "mood".into(),
        })
        .collect();

    let alchemy_body = AlchemyReqBody {
        items,
        n: PLAYLIST_MAX_SIZE,
        subtract_distance: ALCHEMY_SUBSTRACT_DISTANCE,
        temperature: ALCHEMY_TEMPERATURE,
    };

    let songs: Vec<_> = am_client
        .alchemy(&alchemy_body)?
        .results
        .into_iter()
        .filter_map(|item| {
            matcher::match_songs(
                vec![SongRef {
                    id: item.item_id,
                    ..SongRef::default()
                }],
                MatchOptions {
                    username: user.to_owned(),
                },
            )
            .ok()
        })
        .filter_map(|results| results.into_iter().next())
        .flatten()
        .filter(|track| filters.matches(track))
        .map(|song| song.id)
        .collect();

    let playlist = Playlist::new(PlaylistKind::MyVibe, user.to_owned());
    playlist.save(&songs)?;

    Ok(())
}
