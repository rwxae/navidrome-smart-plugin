use std::{
    collections::HashMap,
    time::{Duration, UNIX_EPOCH},
};

use anyhow::{Error, anyhow};
use extism_pdk::warn;
use nd_pdk::host::scrobbleretriever::ScrobbleRef;

use crate::audiomuse::{self, CentroidMetadata, SongScore, SongScoreMood};

pub fn guess_mood(
    scrobbles: &[ScrobbleRef],
    am_client: &audiomuse::Client,
) -> Result<[(String, u32); 3], Error> {
    let mut scrobbles = scrobbles
        .iter()
        .filter_map(|scrobble| {
            am_client
                .get_score(&scrobble.media_file_id)
                .inspect_err(|err| warn!("{}", err))
                .ok()
                .map(SongScore::from)
                .map(|score| {
                    (
                        score,
                        UNIX_EPOCH + Duration::from_secs(scrobble.submission_time as u64),
                    )
                })
        })
        .peekable();

    let latest_timestamp = scrobbles
        .peek()
        .map(|scrobble| scrobble.1 + Duration::from_hours(1))
        .ok_or_else(|| anyhow!("Empty listening history"))?;

    let mut avg_mood = SongScoreMood::default();
    let mut avg_genres: HashMap<String, f64> = HashMap::new();
    let mut total_recency_weight = 0.0;

    for (song_score, listened_at) in scrobbles {
        let hours_elapsed = latest_timestamp
            .duration_since(listened_at)
            .map(|d| d.as_secs_f64() / 3600.0)
            .expect("Latest submission_time + 1 hour must exceed every scrobble submission_time");

        let recency_weight = 2.0f64.powf(-hours_elapsed / 24.0);
        total_recency_weight += recency_weight;

        avg_mood += song_score.mood * recency_weight;

        for (key, value) in song_score.genres {
            *avg_genres.entry(key).or_insert(0.0) += recency_weight * value;
        }
    }

    avg_mood /= total_recency_weight;

    for weight in avg_genres.values_mut() {
        *weight /= total_recency_weight;
    }

    let centroids = am_client.mood_centroids()?;

    let moods = [
        if avg_mood.aggressive >= avg_mood.party {
            (
                "aggressive".into(),
                get_best_cluster(&centroids.aggressive, &avg_genres),
            )
        } else {
            (
                "party".into(),
                get_best_cluster(&centroids.party, &avg_genres),
            )
        },
        if avg_mood.danceable >= avg_mood.relaxed {
            (
                "danceable".into(),
                get_best_cluster(&centroids.danceable, &avg_genres),
            )
        } else {
            (
                "relaxed".into(),
                get_best_cluster(&centroids.relaxed, &avg_genres),
            )
        },
        if avg_mood.happy >= avg_mood.sad {
            (
                "happy".into(),
                get_best_cluster(&centroids.happy, &avg_genres),
            )
        } else {
            ("sad".into(), get_best_cluster(&centroids.sad, &avg_genres))
        },
    ];

    Ok(moods)
}

fn get_best_cluster(clusters: &[CentroidMetadata], genres: &HashMap<String, f64>) -> u32 {
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
        .expect("At least one cluster for the given mood")
}
