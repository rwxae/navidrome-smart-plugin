mod analysis;
mod audiomuse;
mod scrobble;
mod subsonic;

use analysis::guess_mood;
use anyhow::Error;
use audiomuse::{AlchemyItem, AlchemyReqBody};
use nd_pdk::{
    host::{
        config, scheduler,
        scrobbleretriever::{self, ScrobbleOptions},
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
        scheduler::schedule_recurring("*/1 * * * *", SCHEDULE_UPDATE_ID, SCHEDULE_UPDATE_ID)
            .map_err(|err| nd_pdk::lifecycle::Error::new(err.to_string()))?;
        Ok(())
    }
}

impl CallbackProvider for SmartPlugin {
    fn on_callback(&self, _req: SchedulerCallbackRequest) -> Result<(), nd_pdk::scheduler::Error> {
        generate_playlists().map_err(|err| nd_pdk::scheduler::Error::new(err.to_string()))
    }
}

nd_pdk::register_lifecycle_init!(SmartPlugin);
nd_pdk::register_scheduler_callback!(SmartPlugin);

fn generate_playlists() -> Result<(), Error> {
    let am_url =
        config::get("audiomuse_url")?.ok_or_else(|| Error::msg("audiomuse_url is required"))?;
    let am_token =
        config::get("audiomuse_token")?.ok_or_else(|| Error::msg("audiomuse_token is required"))?;
    let nm_user = "rwxae".to_owned();

    let am_client = audiomuse::Client::new(&am_url, &am_token);
    let sb_client = subsonic::Client::new(&nm_user);

    let now = scrobbleretriever::get_last_timestamp(&nm_user)?
        .map(scrobble::to_system_time)
        .ok_or_else(|| Error::msg("no listening history"))?;

    let scrobble_options = ScrobbleOptions {
        descending: true,
        max_items: LISTENS_COUNT,
        ..Default::default()
    };

    let (listens, _) = scrobbleretriever::get_scrobbles(&nm_user, scrobble_options)?;

    let items: Vec<AlchemyItem> = guess_mood(&listens, &am_client, now)?
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
