use std::{fs, path::PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{
    error::VeilError,
    queue::{QueueOrigin, RepeatMode},
    services::utils::data_path,
};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct VeilConfig {
    /// User interface–related preferences
    pub ui: UiConfig,

    /// Integration & connectivity settings
    pub integrations: IntegrationsConfig,

    /// Music library settings
    pub library: LibraryConfig,

    /// Playback behavior and queue info
    pub playback: PlaybackConfig,
}

/// UI configuration such as theme or other future endeavors
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct UiConfig {
    /// What theme the user has selected
    pub theme: ThemeMode,
}

/// Integrations like Discord RPC or Last.FM
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct IntegrationsConfig {
    /// If Discord RPC should be enabled
    pub discord_enabled: bool,

    /// If Last.FM should be enabled
    pub last_fm_enabled: bool,

    /// The session key from Last.FM, used for API communication
    pub last_fm_session_key: Option<String>,
}

/// Music library settings
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LibraryConfig {
    /// The directory where all the music files are
    pub music_dir: Option<String>,
}

/// Playback behavior and queue state
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PlaybackConfig {
    /// Where the queue originated from
    pub queue_origin: Option<QueueOrigin>,

    /// What index the queue is at
    pub queue_idx: usize,

    /// What repeat mode the queue should be at
    pub repeat_mode: RepeatMode,

    /// How far the track has progressed
    pub progress: f64,

    /// What volume the playback should be at
    pub volume: f64,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug, Default)]
pub enum ThemeMode {
    #[default]
    Dark,

    Light,

    System,
}

#[derive(Clone)]
pub enum VeilConfigChange {
    SetTheme(ThemeMode),
    SetDiscordStatus(bool),

    SetMusicDirectory(String),

    SetLastFmStatus(bool),
    SetLastFmSessionKey(String),

    SetQueueOrigin(QueueOrigin),
    SetQueueIdx(usize),
    SetRepeatMode(RepeatMode),

    SetProgress(f64),
    SetVolume(f64),
}

impl VeilConfig {
    pub fn new() -> Result<Self, VeilError> {
        let path = Self::config_file_path();
        if path.exists() {
            let json_reader = fs::File::open(path).expect("error opening config.json reader");
            Ok(serde_json::from_reader(json_reader)?)
        } else {
            let config = Self {
                ui: UiConfig {
                    theme: ThemeMode::Dark,
                },
                library: LibraryConfig { music_dir: None },
                integrations: IntegrationsConfig {
                    discord_enabled: false,
                    last_fm_enabled: false,
                    last_fm_session_key: None,
                },
                playback: PlaybackConfig {
                    queue_origin: None,
                    queue_idx: 0,
                    repeat_mode: RepeatMode::None,
                    progress: 0.0,
                    volume: 0.5,
                },
            };
            config.write_config()?;
            Ok(config)
        }
    }

    /// Update config field values
    fn update_config(&mut self, change: VeilConfigChange) {
        match change {
            VeilConfigChange::SetTheme(theme_mode) => self.ui.theme = theme_mode,
            VeilConfigChange::SetDiscordStatus(status) => {
                self.integrations.discord_enabled = status;
            }
            VeilConfigChange::SetMusicDirectory(music_dir) => {
                self.library.music_dir = Some(music_dir);
            }
            VeilConfigChange::SetLastFmStatus(status) => self.integrations.last_fm_enabled = status,
            VeilConfigChange::SetLastFmSessionKey(session_key) => {
                self.integrations.last_fm_session_key = Some(session_key);
            }
            VeilConfigChange::SetQueueOrigin(queue_origin) => {
                self.playback.queue_origin = Some(queue_origin);
            }
            VeilConfigChange::SetQueueIdx(queue_idx) => self.playback.queue_idx = queue_idx,
            VeilConfigChange::SetRepeatMode(repeat_mode) => self.playback.repeat_mode = repeat_mode,
            VeilConfigChange::SetProgress(progress) => self.playback.progress = progress,
            VeilConfigChange::SetVolume(volume) => self.playback.volume = volume,
        };
    }

    pub fn update_many(&mut self, changes: impl IntoIterator<Item = VeilConfigChange>) {
        for c in changes {
            self.update_config(c);
        }
    }

    /// Update config field values and writes it to disk
    pub fn update_config_and_write(&mut self, change: VeilConfigChange) -> Result<(), VeilError> {
        self.update_config(change);
        self.write_config()?;

        Ok(())
    }

    pub fn write_config(&self) -> Result<(), VeilError> {
        fs::write(
            Self::config_file_path(),
            serde_json::to_string_pretty(&self)?,
        )
        .context("Failed to write to config.json")?;
        Ok(())
    }

    pub fn config_file_path() -> PathBuf {
        data_path().join("config.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_many() {
        let mut config = VeilConfig::default();

        assert_eq!(config.ui.theme, ThemeMode::Dark);
        assert_eq!(config.integrations.last_fm_enabled, false);

        config.update_many([
            VeilConfigChange::SetTheme(ThemeMode::Dark),
            VeilConfigChange::SetLastFmStatus(true),
        ]);

        assert_eq!(config.ui.theme, ThemeMode::Dark);
        assert_eq!(config.integrations.last_fm_enabled, true);
    }

    #[test]
    fn update_theme() {
        let mut config = VeilConfig::default();

        assert_eq!(config.ui.theme, ThemeMode::Dark);

        config.update_config(VeilConfigChange::SetTheme(ThemeMode::Light));

        assert_eq!(config.ui.theme, ThemeMode::Light);
    }

    #[test]
    fn update_music_dir() {
        let mut config = VeilConfig::default();

        assert_eq!(config.library.music_dir, None);

        config.update_config(VeilConfigChange::SetMusicDirectory("hello".to_owned()));

        assert_eq!(config.library.music_dir, Some("hello".to_owned()));
    }

    #[test]
    fn update_discord_enabled() {
        let mut config = VeilConfig::default();

        assert_eq!(config.integrations.discord_enabled, false);

        config.update_config(VeilConfigChange::SetDiscordStatus(true));

        assert_eq!(config.integrations.discord_enabled, true);
    }

    #[test]
    fn update_last_fm_enabled() {
        let mut config = VeilConfig::default();

        assert_eq!(config.integrations.last_fm_enabled, false);

        config.update_config(VeilConfigChange::SetLastFmStatus(true));

        assert_eq!(config.integrations.last_fm_enabled, true);
    }

    #[test]
    fn update_last_fm_key() {
        let mut config = VeilConfig::default();

        assert_eq!(config.integrations.last_fm_session_key, None);

        config.update_config(VeilConfigChange::SetLastFmSessionKey("hello".to_owned()));

        assert_eq!(
            config.integrations.last_fm_session_key,
            Some("hello".to_owned())
        );
    }

    #[test]
    fn update_queue_origin() {
        let mut config = VeilConfig::default();
        let origin = QueueOrigin::Album { id: 0 };

        assert_eq!(config.playback.queue_origin, None);

        config.update_config(VeilConfigChange::SetQueueOrigin(origin.clone()));

        assert_eq!(config.playback.queue_origin, Some(origin));
    }

    #[test]
    fn update_queue_idx() {
        let mut config = VeilConfig::default();

        assert_eq!(config.playback.queue_idx, usize::MIN);

        config.update_config(VeilConfigChange::SetQueueIdx(usize::MAX));

        assert_eq!(config.playback.queue_idx, usize::MAX);
    }

    #[test]
    fn update_repeat_mode() {
        let mut config = VeilConfig::default();

        assert_eq!(config.playback.repeat_mode, RepeatMode::None);

        config.update_config(VeilConfigChange::SetRepeatMode(RepeatMode::Track));

        assert_eq!(config.playback.repeat_mode, RepeatMode::Track);
    }
}
