use std::{collections::HashMap, error::Error, time::SystemTime};

use itertools::Itertools;
use listenbrainz::raw::response::UserListensListen;

use my_vibe_research::{
    audiomuse::{self, AlchemyItem, AlchemyReqBody, MoodCentroidsResponse, SongScore},
    navidrome,
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
const LISTENS_COUNT: u64 = 50;

const PLAYLIST_MAX_SIZE: u32 = 100;
const ALCHEMY_SUBSTRACT_DISTANCE: f32 = 0.2;
const ALCHEMY_TEMPERATURE: f32 = 1.0;

const SUBSONIC_API_VERSION: &str = "1.16.1";
const APP_NAME: &str = "my-vibe";

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
    listens: &[UserListensListen],
    am_client: &audiomuse::Client,
    now: SystemTime,
) -> Result<Vec<String>, attohttpc::Error> {
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

    let centroids = am_client.mood_centroids(None)?;

    let v = avg_mood
        .iter()
        .sorted_by(|a, b| b.1.total_cmp(a.1))
        .take(2)
        .map(|(mood, _)| mood)
        .map(|mood| (mood, get_best_cluster(&centroids, &avg_genres, mood)))
        .map(|(mood, cluster)| format!("{}:{}", mood, cluster))
        .collect();

    Ok(v)
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

    nm_client.create_playlist(
        "My Vibe",
        alchemy_result.results.iter().map(|i| i.item_id.clone()),
    )?;

    Ok(())
}
