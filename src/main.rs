use std::{
    error::Error,
    time::{Duration, UNIX_EPOCH},
};

use itertools::Itertools;
use listenbrainz::raw::{
    Client,
    response::{UserListensListen, UserListensTrackMetadata},
};

use my_vibe_research::{
    audiomuse::{self, AlchemyItem, AlchemyReqBody},
    navidrome,
};

// TODO: implement pagination and get songs from 7 days
const LISTENS_COUNT: u64 = 1000;
const SEED_COUNT: usize = 5;

const PLAYLIST_MAX_SIZE: u32 = 50;
const ALCHEMY_SUBSTRACT_DISTANCE: f32 = 0.2;
const ALCHEMY_TEMPERATURE: f32 = 0.5;

const SUBSONIC_API_VERSION: &str = "1.16.1";
const APP_NAME: &str = "my-vibe";

fn calc_song_score(listens: &[UserListensListen]) -> f64 {
    listens
        .iter()
        .map(|listen| {
            let hours = (UNIX_EPOCH + Duration::from_secs(listen.listened_at as u64))
                .elapsed()
                .unwrap()
                .as_secs_f64()
                / 3600.0;
            1.0 / (1.0 + hours / 168.0)
        })
        .sum()
}

fn get_top_lb_songs(
    client: &Client,
    user: &str,
) -> Result<Vec<UserListensTrackMetadata>, listenbrainz::Error> {
    let songs = client
        .user_listens(user, None, None, Some(LISTENS_COUNT))?
        .payload
        .listens
        .into_iter()
        .into_group_map_by(|l| l.recording_msid.clone())
        .into_values()
        .map(|listens| {
            (
                calc_song_score(&listens),
                listens.into_iter().next().unwrap().track_metadata,
            )
        })
        .sorted_by(|a, b| b.0.total_cmp(&a.0))
        .map(|(_, v)| v)
        .take(SEED_COUNT)
        .collect();
    Ok(songs)
}

fn main() -> Result<(), Box<dyn Error>> {
    let lb_user = std::env::var("LISTENBRAINZ_USERNAME")?;

    let am_url = std::env::var("AUDIOMUSE_URL")?;
    let am_token = std::env::var("AUDIOMUSE_TOKEN")?;

    let nm_url = std::env::var("SUBSONIC_URL")?;
    let nm_user = std::env::var("SUBSONIC_USERNAME")?;
    let nm_pass = std::env::var("SUBSONIC_PASSWORD")?;

    let lb_client = listenbrainz::raw::Client::new();
    let am_client = audiomuse::Client::new(&am_url, &am_token);
    let nm_client =
        navidrome::Client::new(&nm_url, APP_NAME, SUBSONIC_API_VERSION, &nm_user, &nm_pass);

    let top_songs = get_top_lb_songs(&lb_client, &lb_user)?;

    let mut items = Vec::with_capacity(top_songs.len());

    for song in &top_songs {
        let candidates =
            am_client.search_tracks(&format!("{} {}", song.artist_name, song.track_name))?;
        let Some(track) = candidates.first() else {
            continue;
        };
        items.push(AlchemyItem {
            id: track.item_id.clone(),
            op: "ADD".into(),
            r#type: "song".into(),
        });
    }

    let alchemy_body = AlchemyReqBody {
        items,
        n: PLAYLIST_MAX_SIZE,
        subtract_distance: ALCHEMY_SUBSTRACT_DISTANCE,
        temperature: ALCHEMY_TEMPERATURE,
    };

    let alchemy_result = am_client.alchemy(&alchemy_body)?;

    nm_client.create_playlist(
        "My Vibe",
        alchemy_result.results.iter().map(|i| i.item_id.clone()),
    )?;

    Ok(())
}
