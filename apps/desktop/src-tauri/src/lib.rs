//! Tauri v2 desktop application bootstrapper binding `VesselEngine` and `MpvWindowBackend`.

mod commands;
mod embedded_player;
mod events;
mod updater;

use std::sync::Arc;

use tauri::Manager;
use vessel_domain::{EventSink, PlayerBackend, VesselPaths};
use vessel_engine::VesselEngine;
use vessel_player_mpv::{MpvConfig, MpvWindowBackend};

use commands::AppState;
use embedded_player::EmbeddedPlayerHost;
use events::TauriEventSink;

/// Runs the Vessel Tauri desktop shell.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let paths = if std::env::var_os("FLATPAK_ID").is_some() {
                let data_dir = app.path().app_data_dir()?;
                VesselPaths::new(data_dir)
                    .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?
            } else {
                VesselPaths::from_home_env().or_else(|_| {
                    let data_dir = app.path().app_data_dir()?;
                    VesselPaths::new(data_dir)
                        .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)
                })?
            };

            let event_sink: Arc<dyn EventSink> =
                Arc::new(TauriEventSink::new(app.handle().clone()));
            let player: Arc<dyn PlayerBackend> = Arc::new(MpvWindowBackend::new(
                MpvConfig::default(),
                Some(Arc::clone(&event_sink)),
            ));

            let engine = tauri::async_runtime::block_on(async {
                let eng = VesselEngine::open(&paths, player, event_sink, false).await?;
                if let Ok(Some(always_proxy)) =
                    eng.storage().get_json_setting::<bool>("always_proxy").await
                {
                    eng.set_always_proxy(always_proxy);
                }
                if let Ok(Some(meta_enabled)) = eng
                    .storage()
                    .get_json_setting::<bool>("metadata_enabled")
                    .await
                {
                    eng.set_metadata_enabled(meta_enabled);
                }
                if let Ok(Some(custom_dl_dir)) = eng
                    .storage()
                    .get_json_setting::<String>("custom_download_dir")
                    .await
                {
                    let trimmed = custom_dl_dir.trim();
                    if !trimmed.is_empty() {
                        let _ = eng.set_downloads_dir(trimmed);
                    }
                }
                let _ = eng.bootstrap_plugins().await;
                Ok::<_, vessel_domain::VesselError>(Arc::new(eng))
            })?;

            let reaper_engine = Arc::clone(&engine);
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(45));
                interval.tick().await;
                loop {
                    interval.tick().await;
                    let _ = reaper_engine.evict_idle_plugins().await;
                }
            });

            let embedded_host = Arc::new(EmbeddedPlayerHost::new(app.handle().clone()));
            app.manage(AppState {
                engine,
                paths,
                embedded_host,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_engine_info,
            commands::set_always_proxy,
            commands::set_metadata_enabled,
            commands::set_tmdb_api_key,
            commands::set_simkl_client_id,
            commands::get_ui_preferences,
            commands::set_ui_preferences,
            commands::list_themes,
            commands::import_theme_json,
            commands::list_custom_locales,
            commands::load_custom_locale,
            commands::list_plugins,
            commands::resolve_source,
            commands::install_source,
            commands::set_plugin_enabled,
            commands::uninstall_plugin,
            commands::check_plugin_updates,
            commands::update_plugin,
            commands::update_all_plugins,
            commands::grant_plugin_domain,
            commands::get_plugin_settings,
            commands::set_plugin_settings,
            commands::get_home_feeds,
            commands::start_home_feeds,
            commands::search_all,
            commands::cancel_search,
            commands::get_details_with_enrichment,
            commands::get_streams,
            commands::get_streams_with_linked,
            commands::search_plugin_sync,
            commands::search_metadata,
            commands::get_linked_sources,
            commands::set_linked_sources,
            commands::set_metadata_override,
            commands::list_library,
            commands::upsert_library,
            commands::delete_library_item,
            commands::clear_runtime_caches,
            commands::fetch_subtitle_text,
            commands::prepare_playback_url,
            commands::play_candidate,
            commands::record_progress,
            commands::get_watch_progress,
            commands::list_media_watch_progress,
            commands::list_all_watch_progress,
            commands::list_continue_watching,
            commands::delete_watch_progress,
            commands::get_skip_segments,
            commands::player_play,
            commands::player_pause,
            commands::player_seek,
            commands::player_stop,
            commands::embedded_mpv_capabilities,
            commands::embedded_mpv_load,
            commands::embedded_mpv_play,
            commands::embedded_mpv_pause,
            commands::embedded_mpv_seek,
            commands::embedded_mpv_set_volume,
            commands::embedded_mpv_set_speed,
            commands::embedded_mpv_set_fit_mode,
            commands::embedded_mpv_select_audio_track,
            commands::embedded_mpv_select_subtitle_track,
            commands::embedded_mpv_stop,
            commands::add_subtitle,
            commands::set_subtitle_delay,
            commands::list_subtitle_plugins,
            commands::search_subtitles,
            commands::resolve_subtitle,
            commands::list_iptv_profiles,
            commands::upsert_iptv_profile,
            commands::delete_iptv_profile,
            commands::sync_iptv_profile,
            commands::sync_iptv_epg,
            commands::search_iptv_channels,
            commands::list_iptv_groups,
            commands::set_iptv_favorite,
            commands::get_epg_now_next,
            commands::play_iptv_channel,
            commands::list_downloads,
            commands::enqueue_download,
            commands::reorder_downloads,
            commands::prepare_local_download_playback,
            commands::pause_download,
            commands::resume_download,
            commands::delete_download,
            commands::set_download_directory,
            commands::pick_download_directory,
            commands::export_backup_json,
            commands::import_backup_json,
            commands::open_external_url,
            updater::get_update_preferences,
            updater::set_auto_check_updates,
            updater::skip_app_update_version,
            updater::check_app_update,
            updater::download_and_open_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Vessel desktop application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app_handle.state::<AppState>();
                state.embedded_host.teardown(app_handle);
                let engine = Arc::clone(&state.engine);
                tauri::async_runtime::block_on(engine.shutdown());
            }
        });
}
