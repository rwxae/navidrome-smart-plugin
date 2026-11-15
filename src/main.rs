use std::error::Error;

use itertools::Itertools;
use listenbrainz::raw::Client;

fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let listens: Vec<_> = client
        .user_listens("rwxae", None, None, Some(1000))?
        .payload
        .listens
        .into_iter()
        .into_group_map_by(|l| l.recording_msid.clone())
        .into_iter()
        .sorted_by(|a, b| b.1.len().cmp(&a.1.len()))
        .take(5)
        .collect();

    for (_, v) in &listens {
        let rec = v.first().unwrap();
        println!(
            "{} - {}",
            rec.track_metadata.artist_name, rec.track_metadata.track_name
        );
    }

    Ok(())
}
