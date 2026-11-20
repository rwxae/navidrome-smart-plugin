mod audiomuse;
mod scrobble;
mod subsonic;

use std::{collections::HashMap, time::SystemTime};

use anyhow::Error;

use audiomuse::{AlchemyItem, AlchemyReqBody, MoodCentroidsResponse, SongScore};
use nd_pdk::{
    host::{
        config, scheduler,
        scrobbleretriever::{self, ScrobbleOptions, ScrobbleRef},
    },
    lifecycle::InitProvider,
    scheduler::{CallbackProvider, SchedulerCallbackRequest},
};

// TODO: Different users have different activity:
// 1. Heavy user might listen like 100+ songs
// 2. Regular user could listen 10-15 songs
// 3. Inactive user might listen 1-2 songs
//
// So in my opinion each user should have a different window size.
// It makes sense to use time instead of hard-coded count.
// I would take listens from the last 24 hours (starting from the latest
// listen timestamp). Taking older songs might give worse results since
// user can easily change their mood in the next day.
//
// Final formula could look like this:
// count =  min(last_24_hours_count, 100)
const LISTENS_COUNT: i32 = 50;

const PLAYLIST_MAX_SIZE: u32 = 100;
const ALCHEMY_SUBSTRACT_DISTANCE: f32 = 0.2;
const ALCHEMY_TEMPERATURE: f32 = 1.0;

const SCHEDULE_UPDATE_ID: &str = "update";

#[derive(Default)]
struct SmartPlugin;

impl InitProvider for SmartPlugin {
    fn on_init(&self) -> Result<(), nd_pdk::lifecycle::Error> {
        scheduler::schedule_recurring("*/5 * * * *", SCHEDULE_UPDATE_ID, SCHEDULE_UPDATE_ID)
            .unwrap();
        Ok(())
    }
}

impl CallbackProvider for SmartPlugin {
    fn on_callback(&self, _req: SchedulerCallbackRequest) -> Result<(), nd_pdk::scheduler::Error> {
        main().unwrap();
        Ok(())
    }
}

nd_pdk::register_lifecycle_init!(SmartPlugin);
nd_pdk::register_scheduler_callback!(SmartPlugin);

fn get_best_cluster(
    centroids: &MoodCentroidsResponse,
    genres: &HashMap<String, f64>,
    mood: &str,
) -> u32 {
    let Some(clusters) = centroids.get(mood) else {
        panic!("no centroids for mood {mood}")
    };

    let mut doc_freq: HashMap<String, usize> = HashMap::new();
    for cluster in clusters {
        for tag in &cluster.top_tags {
            *doc_freq.entry(tag.into()).or_insert(0) += 1;
        }
    }
    let n_clusters = clusters.len() as f64;
    let idf: HashMap<String, f64> = doc_freq
        .into_iter()
        .map(|(tag, df)| {
            let idf_value = ((n_clusters + 1.0) / (df as f64 + 1.0)).ln() + 1.0;
            (tag, idf_value)
        })
        .collect();

    let user_tf_idf: HashMap<String, f64> = genres
        .iter()
        .map(|(genre, &tf)| {
            let idf_value = idf.get(genre).copied().unwrap_or(0.0);
            (genre.into(), tf * idf_value)
        })
        .collect();

    clusters
        .iter()
        .map(|cluster| {
            let similarity: f64 = cluster
                .top_tags
                .iter()
                .map(|tag| user_tf_idf.get(tag).copied().unwrap_or(0.0))
                .sum();
            (similarity, cluster.cluster_id)
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, cluster_id)| cluster_id)
        .expect("no clusters found")
}

fn get_mood(
    listens: &[ScrobbleRef],
    am_client: &audiomuse::Client,
    now: SystemTime,
) -> Result<Vec<String>, Error> {
    // TODO: review errors
    // TODO: introduce a custom error for this app.
    // TODO: it is a terrible check (listens may have no scores in audiomuse database)
    // TODO: additional moods should be added only if they are significant
    if listens.is_empty() {
        panic!("empty listens")
    }

    let mut avg_mood: HashMap<String, f64> = HashMap::new();
    let mut avg_genres: HashMap<String, f64> = HashMap::new();
    let mut total_recency_weight = 0.0;

    for listen in listens {
        // let query = scrobble::song_query(&listen.track_metadata);
        // let track = am_client.search_track(&query)?;
        // let Some(track) = track else {
        //     continue;
        // };
        let song_score: SongScore = am_client.get_score(&listen.media_file_id)?.into();
        let hours = scrobble::since_listen(now, listen.submission_time).as_secs_f64() / 3600.0;
        let recency_weight = 2.0f64.powf(-hours / 24.0);
        total_recency_weight += recency_weight;

        for (key, value) in song_score.mood {
            *avg_mood.entry(key).or_insert(0.0) += recency_weight * value;
        }

        for (key, value) in song_score.genres {
            *avg_genres.entry(key).or_insert(0.0) += recency_weight * value;
        }
    }

    // TODO: early return if no songs.

    for weight in avg_mood.values_mut() {
        *weight /= total_recency_weight;
    }

    for weight in avg_genres.values_mut() {
        *weight /= total_recency_weight;
    }

    let centroids = am_client.mood_centroids()?;

    let mut sorted_moods = avg_mood.iter().collect::<Vec<_>>();
    sorted_moods.sort_by(|a, b| b.1.total_cmp(a.1));

    let v = sorted_moods
        .iter()
        .take(2)
        .map(|(mood, _)| mood)
        .map(|mood| (mood, get_best_cluster(&centroids, &avg_genres, mood)))
        .map(|(mood, cluster)| format!("{}:{}", mood, cluster))
        .collect();

    Ok(v)
}

// TODO: use anyhow instead
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let am_url =
        config::get("audiomuse_url")?.ok_or_else(|| Error::msg("audiomuse_url is required"))?;
    let am_token =
        config::get("audiomuse_token")?.ok_or_else(|| Error::msg("audiomuse_token is required"))?;
    let nm_user = "rwxae".to_owned();

    let am_client = audiomuse::Client::new(&am_url, &am_token);
    let sb_client = subsonic::Client::new(&nm_user);

    let now = scrobbleretriever::get_last_timestamp(&nm_user).and_then(|maybe_timestamp| {
        maybe_timestamp
            .map(scrobble::to_system_time)
            .ok_or_else(|| Error::msg("no listening history"))
    })?;

    let scrobble_options = ScrobbleOptions {
        descending: true,
        max_items: LISTENS_COUNT,
        ..Default::default()
    };

    let (listens, _) = scrobbleretriever::get_scrobbles(&nm_user, scrobble_options)?;

    let items: Vec<AlchemyItem> = get_mood(&listens, &am_client, now)?
        .iter()
        .map(|mood| AlchemyItem {
            id: mood.into(),
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

    let alchemy_result = am_client.alchemy(&alchemy_body)?;

    sb_client.create_playlist(
        "My Vibe",
        alchemy_result.results.iter().map(|i| i.item_id.clone()),
        Some("5rv8I3rbnM8OKRRcHAqnWs"), // TODO: take from storage
    )?;

    Ok(())
}
