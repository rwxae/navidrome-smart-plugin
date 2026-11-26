use std::{
    collections::HashMap,
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Error, anyhow};
use extism_pdk::{info, warn};
use nd_pdk::{
    host::{
        kvstore,
        matcher::{self, MatchOptions},
        scrobbleretriever::{self, ScrobbleOptions, ScrobbleRef},
    },
    types::{SongRef, Track},
};

use crate::{
    analysis::{calc_listening_profile, cluster_songs, guess_mood},
    audiomuse::{self, AlchemyReqBody, AlchemyReqItem},
    subsonic::{self, SubsonicResponseData},
};

const PLAYLIST_MAX_SIZE: u32 = 100;
const ALCHEMY_SUBSTRACT_DISTANCE: f32 = 0.2;
const ALCHEMY_TEMPERATURE: f32 = 0.4;

enum PlaylistKind {
    MyVibe,
    DailyMix(u8),
}

impl Display for PlaylistKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MyVibe => write!(f, "My Vibe"),
            Self::DailyMix(n) => write!(f, "Daily Mix #{}", n),
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

fn get_scrobbles(user: &str, from_timestamp: i64) -> Result<Vec<ScrobbleRef>, Error> {
    let mut options = Some(ScrobbleOptions {
        from_timestamp: Some(from_timestamp),
        descending: true,
        ..Default::default()
    });
    let mut result = Vec::new();
    while let Some(opts) = options.take() {
        let (scrobbles, next_options) = scrobbleretriever::get_scrobbles(user, opts)?;
        result.extend(scrobbles);
        options = next_options;
    }
    Ok(result)
}

fn find_song(id: &str, user: &str) -> Option<Track> {
    matcher::match_songs(
        vec![SongRef {
            id: id.to_owned(),
            ..SongRef::default()
        }],
        MatchOptions {
            username: user.to_owned(),
        },
    )
    .inspect_err(|err| warn!("Failed to match song {}: {}", id, err))
    .ok()
    .and_then(|results| results.into_iter().next())
    .flatten()
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
        .map(|(mood, cluster)| AlchemyReqItem {
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
        .filter_map(|item| find_song(&item.item_id, user))
        .filter(|track| filters.matches(track))
        .map(|song| song.id)
        .collect();

    let playlist = Playlist::new(PlaylistKind::MyVibe, user.to_owned());
    playlist.save(&songs)?;

    Ok(())
}

pub fn generate_daily_mixes(
    am_client: &audiomuse::Client,
    user: &str,
    filters: &SongFilters,
) -> Result<(), Error> {
    let six_months_ago =
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64 - 60 * 60 * 24 * 180;
    let scrobbles = get_scrobbles(user, six_months_ago)?;

    info!("Scrobbles Count: {}", scrobbles.len());

    let profile = calc_listening_profile(&scrobbles);

    let mut listened_songs = profile.iter().collect::<Vec<_>>();
    listened_songs.sort_by(|a, b| b.1.total_cmp(a.1));
    let best_songs: Vec<_> = listened_songs
        .into_iter()
        .filter_map(|(id, _)| find_song(id, user))
        .filter(|track| filters.matches(track))
        .filter_map(|track| {
            am_client
                .get_embedding(&track.id)
                .inspect_err(|err| warn!("Failed to fetch song embedding: {}", err))
                .ok()
        })
        .take(150)
        .collect();

    info!("Best Songs Count: {}", best_songs.len());

    let clusters = cluster_songs(&best_songs)?;

    info!("Clusters Items Count: {}", clusters.len());

    let mut songs_by_cluster: HashMap<usize, Vec<String>> = std::collections::HashMap::new();
    for (song, cluster) in best_songs.into_iter().zip(clusters) {
        songs_by_cluster
            .entry(cluster)
            .or_default()
            .push(song.item_id.clone());
    }

    for (cluster, mut songs) in songs_by_cluster {
        songs.sort_by(|a, b| profile[b].total_cmp(&profile[a]));
        let best_songs: Vec<_> = songs.into_iter().take(17).collect();

        info!("Songs in Cluster #{}: {}", cluster, best_songs.len());

        let items = best_songs
            .iter()
            .map(|id| AlchemyReqItem {
                id: id.clone(),
                op: "ADD".into(),
                r#type: "song".into(),
            })
            .collect();

        let alchemy_body = AlchemyReqBody {
            items,
            n: 50, // Grab a little more so we can skip filtered songs
            subtract_distance: ALCHEMY_SUBSTRACT_DISTANCE,
            temperature: ALCHEMY_TEMPERATURE,
        };

        let mut similar_songs = am_client
            .alchemy(&alchemy_body)?
            .results
            .into_iter()
            .filter_map(|item| find_song(&item.item_id, user))
            .filter(|track| filters.matches(track));

        let mut track_list = Vec::with_capacity(51);
        for song in best_songs {
            track_list.push(song);
            if let Some(similar_song) = similar_songs.next() {
                track_list.push(similar_song.id.clone());
            }
            if let Some(similar_song) = similar_songs.next() {
                track_list.push(similar_song.id.clone());
            }
        }

        info!("Track List Size: {}", track_list.len());

        let playlist = Playlist::new(PlaylistKind::DailyMix((cluster + 1) as u8), user.to_owned());
        playlist.save(&track_list)?;
    }

    Ok(())
}
