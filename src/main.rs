use std::{
    error::Error,
    time::{Duration, UNIX_EPOCH},
};

use itertools::Itertools;
use listenbrainz::raw::{
    Client,
    response::{UserListensListen, UserListensTrackMetadata},
};

// TODO: implement pagination and get songs from 7 days
const LISTENS_COUNT: u64 = 500;
const SEED_COUNT: usize = 3;

// const ALCHEMY_TEMPERATURE: f32 = 0.3;

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
) -> Result<Vec<(f64, UserListensTrackMetadata)>, listenbrainz::Error> {
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
        .take(SEED_COUNT)
        .collect();
    Ok(songs)
}

fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let top_songs = get_top_lb_songs(&client, "rwxae")?;

    for (score, rec) in &top_songs {
        println!(
            "score: {} \nsong: {} - {}\n",
            score, rec.artist_name, rec.track_name
        );
    }

    Ok(())
}
