use std::{collections::HashMap, time::SystemTime};

use anyhow::Error;
use extism_pdk::warn;
use nd_pdk::host::scrobbleretriever::ScrobbleRef;

use crate::{
    audiomuse::{self, GetScoreResponse, MoodCentroidsResponse, SongScore},
    scrobble,
};

pub fn guess_mood(
    listens: &[ScrobbleRef],
    am_client: &audiomuse::Client,
    now: SystemTime,
) -> Result<Vec<(String, u32)>, Error> {
    // TODO: review errors
    // TODO: introduce a custom error for this app.
    // TODO: it is a terrible check (listens may have no scores in audiomuse database)
    // TODO: additional moods should be added only if they are significant
    if listens.is_empty() {
        return Err(Error::msg("Listening history is empty"));
    }

    let mut avg_mood: HashMap<String, f64> = HashMap::new();
    let mut avg_genres: HashMap<String, f64> = HashMap::new();
    let mut total_recency_weight = 0.0;

    for listen in listens {
        // TODO: should I search track in AudioMuse?
        // let query = scrobble::song_query(&listen.track_metadata);
        // let track = am_client.search_track(&query)?;
        // let Some(track) = track else {
        //     continue;
        // };
        let song_score: SongScore = match am_client.get_score(&listen.media_file_id)? {
            GetScoreResponse::Success(raw) => raw.into(),
            GetScoreResponse::Error(api_error) => {
                warn!("{}", api_error.error_message);
                continue;
            }
        };

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

    let moods = sorted_moods
        .into_iter()
        .take(1) // TODO: add additional mood if it is close enough
        .map(|(mood, _)| {
            (
                mood.to_owned(),
                get_best_cluster(&centroids, &avg_genres, mood),
            )
        })
        .collect();

    Ok(moods)
}

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
