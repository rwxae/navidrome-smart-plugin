mod analysis;
mod audiomuse;
mod playlist;
mod subsonic;

use extism_pdk::{info, warn};
use nd_pdk::{
    host::{config, scheduler, users},
    lifecycle::InitProvider,
    scheduler::{CallbackProvider, SchedulerCallbackRequest},
};
use playlist::{SongFilters, generate_my_vibe};

use crate::playlist::generate_daily_mixes;

const SCHEDULE_UPDATE_ID: &str = "update";

#[derive(Default)]
struct SmartPlugin;

impl InitProvider for SmartPlugin {
    fn on_init(&self) -> Result<(), nd_pdk::lifecycle::Error> {
        scheduler::schedule_recurring("*/10 * * * *", SCHEDULE_UPDATE_ID, SCHEDULE_UPDATE_ID)
            .map_err(|err| nd_pdk::lifecycle::Error::new(err.to_string()))?;
        Ok(())
    }
}

impl CallbackProvider for SmartPlugin {
    fn on_callback(&self, _req: SchedulerCallbackRequest) -> Result<(), nd_pdk::scheduler::Error> {
        let audiomuse_url = config::get("audiomuse_url")
            .map_err(|err| nd_pdk::scheduler::Error::new(err.to_string()))?
            .ok_or_else(|| nd_pdk::scheduler::Error::new("config: audiomuse/url is required"))?;
        let audiomuse_token = config::get("audiomuse_token")
            .map_err(|err| nd_pdk::scheduler::Error::new(err.to_string()))?;
        let am_client = audiomuse::Client::new(audiomuse_url, audiomuse_token);

        let exclude_libraries: Vec<_> = config::get("exclude_libraries")
            .map_err(|err| nd_pdk::scheduler::Error::new(err.to_string()))?
            .iter()
            .flat_map(|v| v.split(","))
            .filter_map(|v| v.parse::<i32>().ok())
            .collect();

        let song_filters = SongFilters::default().exclude_libraries(exclude_libraries);

        let users =
            users::get_users().map_err(|err| nd_pdk::scheduler::Error::new(err.to_string()))?;

        for user in users {
            if let Err(err) = generate_my_vibe(&am_client, &user.user_name, &song_filters) {
                warn!(
                    "Failed to create My Vibe mix for {}: {}",
                    &user.user_name, err
                );
            } else {
                info!("Saved My Vibe mix for {}", &user.user_name);
            }
            // if let Err(err) = generate_daily_mixes(&am_client, &user.user_name, &song_filters) {
            //     warn!(
            //         "Failed to create Daily Mixes for {}: {}",
            //         &user.user_name, err
            //     );
            // } else {
            //     info!("Saved Daily Mixes for {}", &user.user_name);
            // }
        }

        Ok(())
    }
}

nd_pdk::register_lifecycle_init!(SmartPlugin);
nd_pdk::register_scheduler_callback!(SmartPlugin);
