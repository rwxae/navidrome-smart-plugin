mod analysis;
mod audiomuse;
mod subsonic;

use analysis::guess_mood;
use anyhow::{Error, anyhow};
use audiomuse::{AlchemyItem, AlchemyReqBody};
use extism_pdk::info;
use nd_pdk::{
    host::{
        config, scheduler,
        scrobbleretriever::{self, ScrobbleOptions},
        users::{User, get_users},
    },
    lifecycle::InitProvider,
    scheduler::{CallbackProvider, SchedulerCallbackRequest},
};

const SCROBBLES_COUNT: i32 = 15;

const PLAYLIST_MAX_SIZE: u32 = 100;
const ALCHEMY_SUBSTRACT_DISTANCE: f32 = 0.2;
const ALCHEMY_TEMPERATURE: f32 = 0.4;

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
        let audiomuse_url = config::get("audiomuse_url")
            .map_err(|err| nd_pdk::scheduler::Error::new(err.to_string()))?
            .ok_or_else(|| nd_pdk::scheduler::Error::new("config: audiomuse_url is required"))?;
        let audiomuse_token = config::get("audiomuse_token")
            .map_err(|err| nd_pdk::scheduler::Error::new(err.to_string()))?
            .ok_or_else(|| nd_pdk::scheduler::Error::new("config: audiomuse_token is required"))?;

        let users = get_users().map_err(|err| nd_pdk::scheduler::Error::new(err.to_string()))?;

        let am_client = audiomuse::Client::new(&audiomuse_url, &audiomuse_token);

        generate_playlists(&am_client, &users)
            .map_err(|err| nd_pdk::scheduler::Error::new(err.to_string()))
    }
}

nd_pdk::register_lifecycle_init!(SmartPlugin);
nd_pdk::register_scheduler_callback!(SmartPlugin);

fn generate_playlists(am_client: &audiomuse::Client, users: &[User]) -> Result<(), Error> {
    let mut errors = Vec::new();

    for user in users {
        if let Err(err) = generate_user_playlists(am_client, user) {
            errors.push(format!("user={}: {}", user.user_name, err));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "Failed to generate playlists for users:\n{}",
            errors.join("\n")
        ))
    }
}

fn generate_user_playlists(am_client: &audiomuse::Client, user: &User) -> Result<(), Error> {
    let scrobble_options = ScrobbleOptions {
        descending: true,
        max_items: SCROBBLES_COUNT,
        ..Default::default()
    };

    let (scrobbles, _) = scrobbleretriever::get_scrobbles(&user.user_name, scrobble_options)?;

    let items: Vec<_> = guess_mood(&scrobbles, am_client)?
        .into_iter()
        .inspect(|(mood, cluster)| info!("Seeding: mood:{mood}, cluster:{cluster}"))
        .map(|(mood, cluster)| AlchemyItem {
            id: format!("{mood}:{cluster}"),
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

    subsonic::create_playlist(
        &user.user_name,
        "My Vibe",
        alchemy_result.results.iter().map(|i| i.item_id.clone()),
        Some("5rv8I3rbnM8OKRRcHAqnWs"), // TODO: take from storage
    )?;

    Ok(())
}
