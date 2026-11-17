use std::{collections::HashMap, error::Error, time::SystemTime};

use itertools::Itertools;
use listenbrainz::raw::response::{UserListensListen, UserListensTrackMetadata};

use my_vibe_research::{
    audiomuse::{self, AlchemyItem, AlchemyReqBody, SongMood, SongScore},
    navidrome,
};

// TODO: implement pagination and get songs from 7 days
const LISTENS_COUNT: u64 = 250;
const MOOD_SONGS_COUNT: usize = 25;
const SEED_COUNT: usize = 3;

const PLAYLIST_MAX_SIZE: u32 = 50;
const ALCHEMY_SUBSTRACT_DISTANCE: f32 = 0.2;
const ALCHEMY_TEMPERATURE: f32 = 1.0;

const SUBSONIC_API_VERSION: &str = "1.16.1";
const APP_NAME: &str = "my-vibe";

// TODO: revisit the algorithm
fn calc_song_score(listens: &[&UserListensListen], now: SystemTime) -> f64 {
    listens
        .iter()
        .map(|listen| {
            let hours = my_vibe_research::listenbrainz::since_listen(now, listen.listened_at)
                .as_secs_f64()
                / 3600.0;
            1.0 / (1.0 + hours / 168.0)
        })
        .sum()
}

fn get_top_lb_songs(
    listens: &[UserListensListen],
    now: SystemTime,
) -> Result<Vec<&UserListensTrackMetadata>, listenbrainz::Error> {
    let songs = listens
        .iter()
        .into_group_map_by(|l| l.recording_msid.clone())
        .into_values()
        .map(|listens| {
            (
                calc_song_score(&listens, now),
                &listens.into_iter().next().unwrap().track_metadata,
            )
        })
        .sorted_by(|a, b| b.0.total_cmp(&a.0))
        .map(|(_, v)| v)
        .take(SEED_COUNT)
        .collect();
    Ok(songs)
}

fn get_mood(
    listens: &[UserListensListen],
    am_client: &audiomuse::Client,
    now: SystemTime,
) -> Result<String, attohttpc::Error> {
    // TODO: introduce a custom error for this app.
    if listens.is_empty() {
        panic!("empty listens")
    }

    let listens = listens.iter().take(MOOD_SONGS_COUNT);
    let mut mood = SongMood::default();
    let mut genres: HashMap<String, f64> = HashMap::new();
    let mut total_recency_weight = 0.0;

    for listen in listens {
        let query = my_vibe_research::listenbrainz::song_query(&listen.track_metadata);
        let track = am_client.search_track(&query)?;
        let Some(track) = track else {
            continue;
        };
        let song_score: SongScore = am_client.get_score(&track.item_id)?.into();
        let hours = my_vibe_research::listenbrainz::since_listen(now, listen.listened_at)
            .as_secs_f64()
            / 3600.0;
        let recency_weight = 2.0f64.powf(-hours / 24.0);
        total_recency_weight += recency_weight;

        mood.aggressive += recency_weight * song_score.mood.aggressive;
        mood.happy += recency_weight * song_score.mood.happy;
        mood.party += recency_weight * song_score.mood.party;
        mood.relaxed += recency_weight * song_score.mood.relaxed;
        mood.sad += recency_weight * song_score.mood.sad;
        mood.danceable += recency_weight * song_score.mood.danceable;

        for genre in song_score.genres {
            let value = recency_weight * genre.weight;
            *genres.entry(genre.name).or_insert(value) += value;
        }
    }

    mood.aggressive /= total_recency_weight;
    mood.happy /= total_recency_weight;
    mood.party /= total_recency_weight;
    mood.relaxed /= total_recency_weight;
    mood.sad /= total_recency_weight;
    mood.danceable /= total_recency_weight;

    for weight in genres.values_mut() {
        *weight /= total_recency_weight;
    }

    dbg!(mood, genres);

    // TODO
    Ok("danceable:32".into())
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

    let listens = lb_client
        .user_listens(&lb_user, None, None, Some(LISTENS_COUNT))?
        .payload
        .listens;

    // Just experimenting...
    let now = listens
        .first()
        .map(|listen| my_vibe_research::listenbrainz::to_system_time(listen.listened_at))
        .unwrap_or_else(SystemTime::now);

    let mood = get_mood(&listens, &am_client, now)?;
    let top_songs = get_top_lb_songs(&listens, now)?;

    let mut items = Vec::with_capacity(top_songs.len());

    items.push(AlchemyItem {
        id: mood,
        op: "ADD".into(),
        r#type: "mood".into(),
    });

    for song in &top_songs {
        let query = my_vibe_research::listenbrainz::song_query(song);
        let Some(song) = am_client.search_track(&query)? else {
            continue;
        };
        items.push(AlchemyItem {
            id: song.item_id.clone(),
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
