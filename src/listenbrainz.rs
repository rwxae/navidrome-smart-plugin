use std::time::{Duration, SystemTime, UNIX_EPOCH};

use itertools::Itertools;
use listenbrainz::raw::response::UserListensTrackMetadata;

pub fn to_system_time(listened_at: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(listened_at as u64)
}

pub fn since_listen(now: SystemTime, listened_at: i64) -> Duration {
    now.duration_since(to_system_time(listened_at)).unwrap()
}

pub fn song_query(listen: &UserListensTrackMetadata) -> String {
    let artist = match listen.additional_info.get("artist_names") {
        Some(serde_json::Value::Array(artists)) => {
            artists.iter().filter_map(|v| v.as_str()).join(" ")
        }
        _ => listen.artist_name.clone(),
    };
    format!("{} {}", artist, listen.track_name)
}
