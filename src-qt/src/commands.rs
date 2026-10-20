//! Routes commands invoked by the frontend to their implementations.
//!
//! Arguments arrive as a JSON object with camelCase keys, matching the conventions of the Tauri
//! commands this replaces, and results are returned as serialized JSON.

use std::num::NonZeroUsize;

use manderrow_types::mods::{ModId, ModMetadata, ModVersion};
use packed_semver::Version;
use serde::Deserialize;
use smol_str::SmolStr;
use uuid::Uuid;

use crate::app::{AppHandle, Channel, RawJson};
use crate::ipc::{ConnectionId, IpcState, S2CMessage};
use crate::launching::LaunchTarget;
use crate::mod_index::thunderstore::ModMarkdown;
use crate::profiles::Profile;
use crate::settings::{SettingsPatch, SettingsStateInner};
use crate::util::search::SortOption;
use crate::{
    bench_commands, games, i18n, importing, installing, ipc, launching, mod_index, profiles,
    settings, tasks, CommandError, Reqwest,
};

/// Deserializes the arguments of a command into an anonymous struct. Fields that borrow from the
/// arguments, other than `&str`, must be marked with `#[serde(borrow)]`.
macro_rules! args {
    ($args:expr, { $($(#[$meta:meta])* $field:ident: $ty:ty),* $(,)? }) => {{
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args<'a> {
            $($(#[$meta])* $field: $ty,)*
            #[serde(skip)]
            _lifetime: std::marker::PhantomData<&'a ()>,
        }
        Args::deserialize($args)
            .map_err(|e| CommandError::from(anyhow::anyhow!("Invalid arguments: {e}")))?
    }};
}

fn json<T: serde::Serialize>(result: Result<T, CommandError>) -> Result<String, CommandError> {
    Ok(serde_json::to_string(&result?).map_err(anyhow::Error::from)?)
}

fn raw(result: Result<RawJson, CommandError>) -> Result<String, CommandError> {
    Ok(result?.0)
}

pub async fn dispatch(
    app: &AppHandle,
    command: &str,
    args: &serde_json::Value,
) -> Result<String, CommandError> {
    let reqwest = || app.state::<Reqwest>();
    let settings = || app.state::<SettingsStateInner>();
    let ipc_state = || app.state::<IpcState>();

    match command {
        "bench_exit_interactive" => json(bench_commands::bench_exit_interactive().await),
        "bench_exit_splash" => json(bench_commands::bench_exit_splash().await),

        "get_games" => json(games::commands::get_games().await),
        "search_games" => {
            let a = args!(args, {
                query: String,
                sort: Vec<SortOption<games::commands::SortColumn>>,
            });
            json(games::commands::search_games(a.query, a.sort).await)
        }
        "get_games_popularity" => raw(games::commands::get_games_popularity().await),
        "get_game_mods_downloads" => raw(games::commands::get_game_mods_downloads().await),

        "get_preferred_locales" => json(Ok(i18n::get_preferred_locales())),

        "preview_import_modpack_from_thunderstore_code" => {
            let a = args!(args, {
                thunderstore_id: Uuid,
                game: &'a str,
                profile_id: Option<Uuid>,
                task_id: tasks::Id,
            });
            json(
                importing::commands::preview_import_modpack_from_thunderstore_code(
                    app,
                    &reqwest(),
                    a.thunderstore_id,
                    a.game,
                    a.profile_id,
                    a.task_id,
                )
                .await,
            )
        }
        "import_modpack_from_thunderstore_code" => {
            let a = args!(args, {
                thunderstore_id: Uuid,
                game: &'a str,
                profile_id: Option<Uuid>,
                mod_progress_channel: u32,
                task_id: tasks::Id,
            });
            json(
                importing::commands::import_modpack_from_thunderstore_code(
                    app,
                    &reqwest(),
                    a.thunderstore_id,
                    a.game,
                    a.profile_id,
                    Channel::new(app.clone(), a.mod_progress_channel),
                    a.task_id,
                )
                .await,
            )
        }

        "clear_cache" => json(installing::commands::clear_cache().await),

        "allocate_ipc_connection" => json(ipc::commands::allocate_ipc_connection(&ipc_state()).await),
        "get_ipc_connections" => json(ipc::commands::get_ipc_connections(&ipc_state()).await),
        "kill_ipc_client" => {
            let a = args!(args, { conn_id: ConnectionId });
            json(ipc::commands::kill_ipc_client(&ipc_state(), a.conn_id).await)
        }
        "send_s2c_message" => {
            let a = args!(args, { conn_id: ConnectionId, msg: S2CMessage });
            json(ipc::commands::send_s2c_message(&ipc_state(), a.conn_id, a.msg).await)
        }

        "launch_profile" => {
            let a = args!(args, {
                #[serde(borrow)]
                target: LaunchTarget<'a>,
                modded: bool,
                conn_id: ConnectionId,
            });
            json(
                launching::commands::launch_profile(
                    app.clone(),
                    &ipc_state(),
                    a.target,
                    a.modded,
                    a.conn_id,
                )
                .await,
            )
        }

        "fetch_mod_index" => {
            let a = args!(args, { game: &'a str, refresh: bool, task_id: tasks::Id });
            json(
                mod_index::commands::fetch_mod_index(app, &reqwest(), a.game, a.refresh, a.task_id)
                    .await,
            )
        }
        "count_mod_index" => {
            let a = args!(args, { game: &'a str, query: &'a str });
            json(mod_index::commands::count_mod_index(a.game, a.query).await)
        }
        "query_mod_index" => {
            let a = args!(args, {
                game: &'a str,
                query: &'a str,
                sort: Vec<SortOption<mod_index::SortColumn>>,
                skip: Option<usize>,
                limit: Option<NonZeroUsize>,
            });
            raw(mod_index::commands::query_mod_index(a.game, a.query, a.sort, a.skip, a.limit).await)
        }
        "get_from_mod_index" => {
            let a = args!(args, {
                game: &'a str,
                #[serde(borrow)]
                mod_ids: Vec<ModId<'a>>,
            });
            raw(mod_index::commands::get_from_mod_index(a.game, a.mod_ids).await)
        }
        "thunderstore_fetch_mod_markdown" => {
            let a = args!(args, {
                owner: &'a str,
                name: &'a str,
                version: Version,
                endpoint: ModMarkdown,
                task_id: tasks::Id,
            });
            raw(
                mod_index::thunderstore::commands::thunderstore_fetch_mod_markdown(
                    app,
                    &reqwest(),
                    a.owner,
                    a.name,
                    a.version,
                    a.endpoint,
                    a.task_id,
                )
                .await,
            )
        }

        "get_profiles" => json(profiles::commands::get_profiles().await),
        "create_profile" => {
            let a = args!(args, { game: SmolStr, name: SmolStr });
            json(profiles::commands::create_profile(a.game, a.name).await)
        }
        "overwrite_profile_metadata" => {
            let a = args!(args, { id: Uuid, metadata: Profile });
            json(profiles::commands::overwrite_profile_metadata(a.id, a.metadata).await)
        }
        "delete_profile" => {
            let a = args!(args, { id: Uuid });
            json(profiles::commands::delete_profile(a.id).await)
        }
        "get_profile_path" => {
            let a = args!(args, { id: Uuid });
            json(profiles::commands::get_profile_path(a.id))
        }
        "get_profile_mods" => {
            let a = args!(args, { id: Uuid });
            raw(profiles::commands::get_profile_mods(a.id).await)
        }
        "install_profile_mod" => {
            let a = args!(args, {
                id: Uuid,
                #[serde(borrow)]
                r#mod: ModMetadata<'a>,
                #[serde(borrow)]
                version: ModVersion<'a>,
                task_id: tasks::Id,
            });
            json(
                profiles::commands::install_profile_mod(
                    app,
                    &reqwest(),
                    a.id,
                    a.r#mod,
                    a.version,
                    a.task_id,
                )
                .await,
            )
        }
        "uninstall_profile_mod" => {
            let a = args!(args, { id: Uuid, owner: &'a str, name: &'a str });
            json(profiles::commands::uninstall_profile_mod(a.id, a.owner, a.name).await)
        }

        "get_settings" => raw(settings::commands::get_settings(&settings()).await),
        "get_settings_ui" => raw(settings::commands::get_settings_ui().await),
        "update_settings" => {
            let a = args!(args, { patch: SettingsPatch });
            json(settings::commands::update_settings(app, &settings(), a.patch).await)
        }

        "allocate_task" => json(tasks::commands::allocate_task().await),
        "cancel_task" => {
            let a = args!(args, { id: tasks::Id });
            json(tasks::commands::cancel_task(a.id).await)
        }

        _ => Err(anyhow::anyhow!("Unknown command {command:?}").into()),
    }
}
