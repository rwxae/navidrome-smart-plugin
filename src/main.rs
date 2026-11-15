use std::{
    error::Error,
    time::{Duration, UNIX_EPOCH},
};

use itertools::Itertools;
use listenbrainz::raw::{Client, response::UserListensListen};

const LISTENS_COUNT: u64 = 500;

fn calc_score(v: &[UserListensListen]) -> f64 {
    v.iter()
        .map(|listen| {
            let hours = (UNIX_EPOCH + Duration::from_secs(listen.listened_at as u64))
                .elapsed()
                .unwrap()
                .as_secs_f64()
                / 3600.0;
            1.0 / (hours + 24.0)
        })
        .sum()
}

fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let listens: Vec<_> = client
        .user_listens("rwxae", None, None, Some(LISTENS_COUNT))?
        .payload
        .listens
        .into_iter()
        .into_group_map_by(|l| l.recording_msid.clone())
        .into_values()
        .map(|group| (calc_score(&group), group))
        .sorted_by(|a, b| b.0.total_cmp(&a.0))
        .take(10)
        .collect();

    for (score, v) in &listens {
        let rec = v.first().unwrap();
        println!(
            "score: {} | count: {}\nsong: {} - {}\n",
            score,
            v.len(),
            rec.track_metadata.artist_name,
            rec.track_metadata.track_name
        );
    }

    Ok(())
}
