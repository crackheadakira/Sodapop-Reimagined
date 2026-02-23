use gpui::{App, Global};
use logging::{lock_or_log, try_with_log};
use std::{
    env,
    fs::create_dir,
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::Notify;

use crate::{
    VeilConfigChange,
    config::VeilConfig,
    discord::DiscordState,
    error::VeilError,
    events::{EventBus, EventSystemHandler, PlayerEvent, QueueEvent, UIUpdateEvent},
    queue::{QueueOrigin, QueueSystem},
    services::{
        player::{next_track_status, try_preloading_next_sound_handle},
        utils::data_path,
    },
    ui::AppStateContext,
};

pub struct VeilState {
    pub player: Arc<RwLock<media_controls::DefaultPlayer>>,
    pub queue: Arc<Mutex<QueueSystem>>,
    pub db: Arc<db::Database>,
    pub discord: Mutex<DiscordState>,
    pub config: Arc<RwLock<VeilConfig>>,
    pub lastfm: Arc<tokio::sync::Mutex<lastfm::LastFM>>,
    pub resume_notify: Arc<Notify>,

    pub player_bus: EventBus<PlayerEvent>,
    pub ui_bus: EventBus<UIUpdateEvent>,
    pub config_bus: EventBus<VeilConfigChange>,
    pub queue_bus: EventBus<QueueEvent>,
}

pub struct AppState(pub Arc<VeilState>);

impl Global for AppState {}

pub fn initialize_state(_cx: &mut App) -> Result<VeilState, VeilError> {
    #[cfg(not(target_os = "windows"))]
    let hwnd = None;

    #[cfg(target_os = "windows")]
    let hwnd = {
        // TODO: untested, will probably fail
        if let Some(handle) = _cx.windows().get(0) {
            handle.get_raw_handle()
        } else {
            None
        }
    };

    let path = data_path();

    if !path.exists() {
        create_dir(&path).expect("Error creating main data directory");
    }

    let platform_config = media_controls::PlatformConfig {
        dbus_name: "com.veil.dbus",
        display_name: "VEIL",
        hwnd,
    };

    let veil_config = try_with_log!("VEIL Config", VeilConfig::new)?;

    #[cfg(debug_assertions)]
    let api_key = env::var("LASTFM_API_KEY").expect("Missing LASTFM_API_KEY environment variable");

    #[cfg(not(debug_assertions))]
    let api_key = env!("LASTFM_API_KEY");

    #[cfg(debug_assertions)]
    let api_secret =
        env::var("LASTFM_API_SECRET").expect("Missing LASTFM_API_SECRET environment variable");

    #[cfg(not(debug_assertions))]
    let api_secret = env!("LASTFM_API_SECRET");

    let mut lastfm = try_with_log!("LastFM API", || lastfm::LastFM::builder()
        .api_key(&api_key)
        .api_secret(&api_secret)
        .build())?;

    #[cfg(debug_assertions)]
    let discord_client_id =
        env::var("DISCORD_CLIENT_ID").expect("Missing DISCORD_CLIENT_ID environment variable");

    #[cfg(not(debug_assertions))]
    let discord_client_id = env!("DISCORD_CLIENT_ID");

    let mut discord = DiscordState::new(&discord_client_id);

    lastfm.enable(veil_config.integrations.last_fm_enabled);
    discord.enable(veil_config.integrations.discord_enabled);

    if let Some(session_key) = veil_config.integrations.last_fm_session_key.clone() {
        lastfm.set_session_key(session_key);
    }

    let player = try_with_log!("Music Player", || {
        media_controls::Player::new(platform_config)
    })?;

    Ok(VeilState {
        player: Arc::new(RwLock::new(player)),
        queue: Arc::new(Mutex::new(QueueSystem::new(
            veil_config.playback.queue_origin,
            veil_config.playback.repeat_mode,
        ))),
        db: Arc::new(db::Database::new(path.clone())),
        lastfm: Arc::new(tokio::sync::Mutex::new(lastfm)),
        config: Arc::new(RwLock::new(veil_config)),
        discord: Mutex::new(discord),
        resume_notify: Arc::new(Notify::new()),

        player_bus: EventBus::new(128),
        ui_bus: EventBus::new(16),
        config_bus: EventBus::new(16),
        queue_bus: EventBus::new(16),
    })
}

pub fn attach_media_controls_to_player(state: Arc<VeilState>) -> Result<(), anyhow::Error> {
    let mut player = lock_or_log(state.player.write(), "Player Write Lock")?;

    let state = state.clone();
    player.controls.attach(move |event| {
        use media_controls::MediaControlEvent;

        match event {
            MediaControlEvent::Play => state.player_bus.emit(PlayerEvent::Resume),
            MediaControlEvent::Pause => state.player_bus.emit(PlayerEvent::Pause),
            MediaControlEvent::Next => state.player_bus.emit(PlayerEvent::NextTrackInQueue),
            MediaControlEvent::Previous => state.player_bus.emit(PlayerEvent::PreviousTrackInQueue),
            MediaControlEvent::SetVolume(volume) => state.player_bus.emit(PlayerEvent::SetVolume {
                volume: volume as f32,
            }),
            MediaControlEvent::SeekBy(direction, duration) => {
                let sign = match direction {
                    media_controls::SeekDirection::Forward => 1.0,
                    media_controls::SeekDirection::Backward => -1.0,
                };
                let secs = duration.as_secs_f64() * sign;
                state.player_bus.emit(PlayerEvent::Seek {
                    position: secs,
                    resume: true,
                });
            }
            MediaControlEvent::SetPosition(position) => state.player_bus.emit(PlayerEvent::Seek {
                position: position.0.as_secs_f64(),
                resume: false,
            }),
            _ => (),
        };
    })?;

    Ok(())
}

pub fn handle_state_setup(cx: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let veil_state = initialize_state(cx)?;
    let app_state = Arc::new(veil_state);
    cx.set_global(AppState(app_state));

    let state = cx.app_state();

    {
        let config = lock_or_log(state.config.read(), "Config RwLock")?;
        if config.integrations.discord_enabled {
            let mut discord = lock_or_log(state.discord.lock(), "Discord Mutex")?;
            discord.connect();
        }
    }

    attach_media_controls_to_player(state.clone())?;

    tokio::spawn({
        let state = state.clone();
        let mut rx = state.config_bus.subscribe();

        async move {
            while let Ok(event) = rx.recv().await {
                let state = state.clone();

                match event {
                    VeilConfigChange::SetDiscordStatus(status) => {
                        let mut discord =
                            lock_or_log(state.discord.lock(), "Discord Mutex").unwrap();
                        let player = lock_or_log(state.player.read(), "Player Read Lock").unwrap();

                        if status {
                            if discord.connect() {
                                let progress = player.get_progress();
                                discord.update_activity_progress(progress);
                            }
                        } else {
                            discord.close();
                        }
                    }
                    VeilConfigChange::SetLastFmStatus(status) => {
                        let mut lastfm = state.lastfm.lock().await;
                        lastfm.enable(status);
                    }
                    _ => (),
                };

                let mut config = lock_or_log(state.config.write(), "Config Write").unwrap();
                config.update_config_and_write(event).unwrap();
            }
        }
    });

    PlayerEvent::attach_listener(state.player_bus.clone(), state.clone());
    QueueEvent::attach_listener(state.queue_bus.clone(), state.clone());

    let path = data_path();
    let covers = path.join("covers");
    if !covers.exists() {
        std::fs::create_dir_all(&covers).expect("Error creating covers directory");
        let pc = include_bytes!("../../../../assets/images/placeholder.png");

        std::fs::write(covers.join("placeholder.png"), pc)?;
    }

    // Populate the queue if a queue origin exists
    let mut queue = lock_or_log(state.queue.lock(), "Queue Mutex").unwrap();
    if let Some(queue_origin) = queue.origin() {
        let config = lock_or_log(state.config.read(), "Config Read").unwrap();
        match queue_origin {
            QueueOrigin::Album { id } => {
                let result = state.db.album_with_tracks(&id)?;
                let track_ids: Vec<u32> = result.tracks.iter().map(|track| track.id).collect();

                queue.set_global(track_ids);
            }
            QueueOrigin::Playlist { id } => {
                let result = state.db.get_playlist_with_tracks(&id)?;
                let track_ids: Vec<u32> = result.tracks.iter().map(|track| track.id).collect();

                queue.set_global(track_ids);
            }
        }

        let saved_progress = config.playback.progress;
        let volume = config.playback.volume;
        queue.set_current_index(config.playback.queue_idx);

        if let Some(track_id) = queue.current() {
            drop(queue);
            drop(config);

            if let Ok(track) = state.db.by_id::<common::Tracks>(&track_id) {
                state.player_bus.emit(PlayerEvent::SetVolume {
                    volume: volume as f32,
                });

                state.player_bus.emit(PlayerEvent::Initialize {
                    track,
                    progress: saved_progress,
                });
            }
        }
    }

    initiate_track_ended_thread(state.clone());
    initiate_player_progress_thread(state.clone());

    Ok(())
}

fn initiate_track_ended_thread(state: Arc<VeilState>) {
    tokio::spawn(async move {
        let check_interval = std::time::Duration::from_millis(25);

        loop {
            tokio::time::sleep(check_interval).await;

            let ended = {
                let player = lock_or_log(state.player.read(), "Player Read Lock").unwrap();
                player.has_ended()
            };

            if !ended {
                continue;
            }

            {
                let mut player = lock_or_log(state.player.write(), "Player Write Lock").unwrap();
                try_preloading_next_sound_handle(&state, &mut player);

                if let Some(track) = next_track_status(&state, &player) {
                    state.player_bus.emit(PlayerEvent::NewTrack { track });
                }
            }

            let queue_has_ended = {
                let queue = lock_or_log(state.queue.lock(), "Queue Mutex").unwrap();
                queue.reached_end
            };

            if queue_has_ended {
                state.player_bus.emit(PlayerEvent::Stop);
                state.resume_notify.notified().await;
            }
        }
    });
}

fn initiate_player_progress_thread(state: Arc<VeilState>) {
    tokio::spawn(async move {
        let check_interval = std::time::Duration::from_millis(400);

        loop {
            tokio::time::sleep(check_interval).await;

            let progress = {
                let player = lock_or_log(state.player.read(), "Player Read Lock").unwrap();

                if matches!(player.state, media_controls::PlayerState::Playing) {
                    Some(player.get_progress())
                } else {
                    None
                }
            };

            if let Some(progress) = progress {
                state
                    .ui_bus
                    .emit(UIUpdateEvent::ProgressUpdate { progress });
            };
        }
    });
}
