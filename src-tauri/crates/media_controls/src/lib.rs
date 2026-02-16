use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common::Tracks;
pub use kira::sound::PlaybackState;
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, StartTime, Tween,
    backend::mock::MockBackend,
    clock::{ClockHandle, ClockSpeed},
    sound::FromFileError,
};
pub use souvlaki::{MediaControlEvent, PlatformConfig, SeekDirection};
use souvlaki::{MediaControls, MediaMetadata, MediaPlayback};

mod seams;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Kira(#[from] kira::PlaySoundError<FromFileError>),

    #[error(transparent)]
    FromFile(#[from] FromFileError),

    #[error(transparent)]
    Souvlaki(#[from] souvlaki::Error),

    #[error(transparent)]
    Cpal(#[from] kira::backend::cpal::Error),

    #[error(transparent)]
    ResourceLimitReached(#[from] kira::ResourceLimitReached),
}

pub(crate) type Result<T, U = Error> = std::result::Result<T, U>;

#[cfg(feature = "serialization")]
use serde::Serialize;

use crate::seams::{
    FakeMediaControls, FakeSoundFactory, KiraSoundFactory, Sound, SoundFactory, SouvlakiControls,
};

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize))]
pub enum PlayerState {
    Playing,
    #[default]
    Paused,
}

pub struct PlayerTrack<S: Sound> {
    /// Handle for the tracks [`StreamingSoundData`]
    sound_handle: S,

    /// Database ID of the track
    pub id: u32,

    /// How far along the track the player has progressed
    pub progress: f64,

    /// Duration of the track
    pub duration: f64,

    /// At what time did the user start listening to the track
    pub timestamp: u64,

    /// If the track has already been scrobbled
    pub scrobbled: bool,

    /// How many seconds does the user need to listen to to be able to scrobble the track
    scrobble_condition: Option<f64>,
}

impl<S: Sound> PlayerTrack<S> {
    pub fn new(track_id: u32, duration: f64, sound_handle: S) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        Self {
            id: track_id,
            progress: sound_handle.position(),
            sound_handle,
            duration,
            timestamp,
            scrobbled: false,
            scrobble_condition: Self::get_scrobble_condition(duration),
        }
    }

    // https://www.last.fm/api/scrobbling#when-is-a-scrobble-a-scrobble
    /// If duration is greater than 4 mins, set condition to 4 mins,
    /// otherwise half of the duration as long as it's above 30 seconds.
    fn get_scrobble_condition(duration: f64) -> Option<f64> {
        if duration > 240.0 {
            Some(240.0)
        } else if duration > 30.0 {
            Some(duration / 2.0)
        } else {
            None
        }
    }
}

pub struct Player<F: SoundFactory, S: SouvlakiControls> {
    sound_factory: F,

    /// To use for playback & volume
    tween: Tween,

    /// Clock to keep track of user's progress, can't use [`Player::progress`] as the user can manipulate that
    clock: ClockHandle,

    /// Volume that ranges from -60 to 1.0
    pub volume: f32,

    /// Player State
    pub state: PlayerState,

    /// Souvlaki Controls
    pub controls: S,

    pub track: Option<PlayerTrack<F::Sound>>,
    preloaded_track: Option<PlayerTrack<F::Sound>>,
}

pub type DefaultPlayer = Player<KiraSoundFactory, MediaControls>;

impl Player<KiraSoundFactory, MediaControls> {
    pub fn new(c: souvlaki::PlatformConfig) -> Result<Self> {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;

        // 1 tick per second
        let clock = manager.add_clock(ClockSpeed::TicksPerMinute(60.0))?;

        let sound_factory = KiraSoundFactory { manager };

        Ok(Player {
            sound_factory,
            clock,
            tween: Tween {
                start_time: StartTime::Immediate,
                duration: Duration::from_millis(0),
                easing: kira::Easing::Linear,
            },
            track: None,
            preloaded_track: None,
            volume: -6.0, // externally 0.0 - 1.0
            state: PlayerState::Paused,
            controls: MediaControls::new(c)?,
        })
    }
}

impl Player<FakeSoundFactory, FakeMediaControls> {
    pub fn new_mock() -> Result<Self> {
        let mut manager =
            AudioManager::<MockBackend>::new(AudioManagerSettings::default()).unwrap();

        // 1 tick per second
        let clock = manager.add_clock(ClockSpeed::TicksPerMinute(60.0))?;

        Ok(Player {
            sound_factory: FakeSoundFactory {},
            clock,
            tween: Tween {
                start_time: StartTime::Immediate,
                duration: Duration::from_millis(0),
                easing: kira::Easing::Linear,
            },
            track: None,
            preloaded_track: None,
            volume: -6.0, // externally 0.0 - 1.0
            state: PlayerState::Paused,
            controls: FakeMediaControls,
        })
    }
}

impl<F: SoundFactory, S: SouvlakiControls> Player<F, S> {
    /// Takes a value from 0.0 to 1.0 and passes to player. Range gets converted to -60.0 to 1.0
    pub fn set_volume(&mut self, volume: f32) -> Result<()> {
        // https://www.desmos.com/calculator/cj1nmmamzb
        let converted_volume = -60.0 + 61.0 * volume.powf(0.44);

        if let Some(ref mut player_track) = self.track {
            player_track
                .sound_handle
                .set_volume(converted_volume, self.tween);
            self.controls.set_volume(volume as f64)?;
        }

        self.volume = converted_volume;

        Ok(())
    }

    /// Returns a value between 0.0 and 1.0
    pub fn get_volume(&self) -> f32 {
        ((self.volume + 60.0) / 61.0).powf(1.0 / 0.44)
    }

    pub fn play(&mut self, track: &Tracks, start_position: Option<f64>) -> Result<(f64, f64)> {
        logging::debug!("Trying to play track {}", track.name);

        let (progress, track_duration) = {
            if let Some(mut player_track) = self.preloaded_track.take() {
                logging::debug!("Found existing player track for track {}", track.name);
                player_track
                    .sound_handle
                    .set_volume(self.volume, self.tween);

                player_track.sound_handle.resume(self.tween);

                let duration = player_track.duration;
                let progress = player_track.progress;
                player_track.scrobbled = false;

                self.track = Some(player_track);

                (progress, duration)
            } else if self.track.as_ref().is_none_or(|t| t.id != track.id) {
                logging::debug!("Creating new player track for {}", track.name);
                let mut player_track = self.create_player_track(track, start_position)?;
                player_track.sound_handle.resume(self.tween);

                let duration = player_track.duration;
                let progress = player_track.progress;

                self.track = Some(player_track);

                (progress, duration)
            } else {
                logging::debug!("Reusing existing track {}", track.name);
                let player_track = self.track.as_mut().unwrap();
                player_track
                    .sound_handle
                    .set_volume(self.volume, self.tween);

                player_track.sound_handle.resume(self.tween);

                let duration = player_track.duration;
                let progress = player_track.progress;
                player_track.scrobbled = false;

                (progress, duration)
            }
        };

        self.state = PlayerState::Playing;
        self.clock.start();

        self.controls.set_metadata(MediaMetadata {
            title: Some(&track.name),
            album: Some(&track.album_name),
            artist: Some(&track.artist_name),
            cover_url: Some(&track.cover_path),
            duration: Some(Duration::from_secs_f64(track_duration)),
        })?;
        self.set_media_controls_playing(progress)?;

        logging::debug!("Succesfully playing track {}", track.name);

        Ok((progress, track_duration))
    }

    /// Initialize the player with a track and a progress
    pub fn initialize_player(&mut self, track: Tracks, progress: f64) -> Result<()> {
        logging::debug!(
            "Initializing player with track {} and progress {progress}",
            track.name
        );

        let player_track = self.create_player_track(&track, Some(progress))?;
        self.track = Some(player_track);

        Ok(())
    }

    fn create_player_track(
        &mut self,
        track: &Tracks,
        start_position: Option<f64>,
    ) -> Result<PlayerTrack<F::Sound>> {
        let progress = start_position.unwrap_or(0.0);

        let (sound, duration) =
            self.sound_factory
                .create(&track.path, progress, self.volume, self.tween)?;

        Ok(PlayerTrack::new(track.id, duration, sound))
    }

    pub fn maybe_queue_next(&mut self, track: &Tracks) -> Result<()> {
        logging::debug!("Preloading next track for gapless playback.");

        let player_track = self.create_player_track(track, None)?;
        self.preloaded_track = Some(player_track);

        Ok(())
    }

    /// Check if the track has ended
    pub fn has_ended(&self) -> bool {
        if let Some(ref player_track) = self.track {
            player_track.sound_handle.state() == PlaybackState::Stopped
        } else {
            true
        }
    }

    /// Pause track if has `sound_handle`
    pub fn pause(&mut self) -> Result<()> {
        if let Some(ref mut player_track) = self.track {
            logging::debug!("Pausing track");

            let progress = player_track.sound_handle.position();
            player_track.sound_handle.pause(self.tween);
            player_track.progress = progress;

            self.state = PlayerState::Paused;

            self.clock.pause();
            self.set_media_controls_paused(progress)?;
        }

        Ok(())
    }

    /// Resume track if has `sound_handle`
    pub fn resume(&mut self) -> Result<()> {
        if let Some(ref mut player_track) = self.track {
            logging::debug!("Resuming track");

            player_track.sound_handle.resume(self.tween);
            self.state = PlayerState::Playing;

            let progress = player_track.progress;
            self.set_media_controls_playing(progress)?;

            self.clock.start();
        }

        Ok(())
    }

    /// Seek to a specific position in the track and resume playing if the player is paused and resume is true
    pub fn seek(&mut self, position: f64, resume: bool) -> Result<()> {
        if let Some(ref mut player_track) = self.track {
            logging::debug!("Seeking track to {position}");

            if let PlayerState::Playing = self.state {
                player_track.sound_handle.seek_to(position);
                player_track.progress = position;
                self.set_media_controls_playing(position)?;
            } else {
                player_track.sound_handle.seek_to(position);
                player_track.progress = position;

                if resume {
                    self.resume()?;
                } else {
                    self.set_media_controls_paused(position)?;
                }
            }
        }

        Ok(())
    }

    /// Stop track if has `sound_handle`
    pub fn stop(&mut self) -> Result<()> {
        logging::debug!("Stopping track");

        if let Some(mut player_track) = self.track.take() {
            player_track.sound_handle.stop(self.tween);
        }

        self.state = PlayerState::Paused;

        self.clock.stop();
        self.controls.set_playback(MediaPlayback::Stopped)?;

        Ok(())
    }

    /// Gets player progress from `sound_handle`
    pub fn get_progress(&self) -> f64 {
        if let Some(ref player_track) = self.track {
            player_track.sound_handle.position()
        } else {
            -1.0
        }
    }

    /// Gets track duration from `track`
    pub fn get_duration(&self) -> f64 {
        if let Some(ref player_track) = self.track {
            player_track.duration
        } else {
            -1.0
        }
    }

    /// Whether or not the clock progress has passsed `scrobble_condition`
    fn scrobble(&self) -> bool {
        self.track
            .as_ref()
            .and_then(|track| {
                track.scrobble_condition.map(|cond| {
                    let ticks = self.clock.time().ticks as f64;
                    ticks >= cond
                })
            })
            .unwrap_or(false)
    }

    /// Whether or not the track should be scrobbled depending on three factors.
    ///
    /// 1. The clock progress has passed `scrobble_condition`.
    /// 2. The track hasn't already been scrobbled.
    /// 3. `track` contains Some(id)
    pub fn should_scrobble(&mut self) -> Option<(u32, i64)> {
        let can_scrobble = self.scrobble();

        if let Some(track) = self.track.as_mut() {
            if !can_scrobble || track.scrobbled {
                return None;
            }

            track.scrobbled = true;
            Some((track.id, track.timestamp as i64))
        } else {
            None
        }
    }

    /// Gets players state from sound handle if exists.
    pub fn get_player_state(&self) -> Option<PlaybackState> {
        self.track
            .as_ref()
            .map(|player_track| player_track.sound_handle.state())
    }

    pub fn has_preloaded_track(&self) -> bool {
        self.preloaded_track.is_some()
    }

    fn set_media_controls_playing(&mut self, progress: f64) -> Result<(), souvlaki::Error> {
        let progress = Self::progress_as_position(progress);
        self.controls
            .set_playback(MediaPlayback::Playing { progress })
    }

    fn set_media_controls_paused(&mut self, progress: f64) -> Result<(), souvlaki::Error> {
        let progress = Self::progress_as_position(progress);
        self.controls
            .set_playback(MediaPlayback::Paused { progress })
    }

    fn progress_as_position(progress: f64) -> Option<souvlaki::MediaPosition> {
        Some(souvlaki::MediaPosition(std::time::Duration::from_secs_f64(
            progress,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_track(id: u32) -> Tracks {
        Tracks {
            id,
            album_id: 0,
            artist_id: 0,
            album_name: "a".into(),
            artist_name: "a".into(),
            name: "a".into(),
            number: 0,
            duration: 120,
            cover_path: "a".into(),
            path: "a".into(),
            hash: "a".into(),
        }
    }

    #[test]
    fn set_volume_range() -> Result<()> {
        let mut player = Player::new_mock()?;

        player.set_volume(0.0)?;
        assert_eq!(player.volume, -60.0);

        player.set_volume(1.0)?;
        assert_eq!(player.volume, 1.0);
        Ok(())
    }

    #[test]
    fn play_creates_track_when_none_preloaded() -> Result<()> {
        let mut player = Player::new_mock()?;

        assert_eq!(player.state, PlayerState::Paused);
        assert!(!player.track.is_some());
        assert!(!player.preloaded_track.is_some());

        player.play(&get_track(0), None)?;

        assert!(player.track.is_some());
        assert!(!player.preloaded_track.is_some());

        assert_eq!(player.state, PlayerState::Playing);
        assert_eq!(player.clock.time().ticks, 0);
        assert_eq!(player.track.as_ref().unwrap().duration, 120.0);
        assert_eq!(player.track.as_ref().unwrap().progress, 0.0);

        Ok(())
    }

    #[test]
    fn maybe_queue_next_sets_preloaded_track() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        assert!(player.preloaded_track.is_none());
        player.maybe_queue_next(&track)?;
        assert!(player.preloaded_track.is_some());
        Ok(())
    }

    #[test]
    fn play_uses_preloaded_track_if_available() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        player.maybe_queue_next(&track)?;

        assert_eq!(player.state, PlayerState::Paused);
        assert!(!player.track.is_some());
        assert!(player.preloaded_track.is_some());

        player.play(&track, None)?;

        assert!(player.track.is_some());
        assert!(!player.preloaded_track.is_some());

        assert_eq!(player.state, PlayerState::Playing);
        assert_eq!(player.clock.time().ticks, 0);
        assert_eq!(player.track.as_ref().unwrap().duration, 120.0);
        assert_eq!(player.track.as_ref().unwrap().progress, 0.0);

        Ok(())
    }

    #[test]
    fn playing_new_track_replaces_current_track() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track1 = get_track(0);
        let track2 = get_track(1);

        player.play(&track1, None)?;
        let first_id = player.track.as_ref().unwrap().id;

        player.play(&track2, None)?;
        let second_id = player.track.as_ref().unwrap().id;

        assert_ne!(first_id, second_id);
        Ok(())
    }

    #[test]
    fn playing_same_track_resets_fields() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track1 = get_track(0);

        player.play(&track1, None)?;
        assert!(!player.track.as_ref().unwrap().scrobbled);

        player.track.as_mut().unwrap().scrobbled = true;

        player.play(&track1, None)?;

        assert!(!player.track.as_ref().unwrap().scrobbled);
        Ok(())
    }

    #[test]
    fn initialize_player_sets_track_with_progress() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        player.initialize_player(track.clone(), 50.0)?;
        assert_eq!(player.track.as_ref().unwrap().progress, 50.0);
        Ok(())
    }

    #[test]
    fn player_play_sets_state_playing() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        assert!(player.get_player_state().is_none());
        player.play(&track, None)?;

        assert_eq!(player.get_player_state().unwrap(), PlaybackState::Playing);
        assert_eq!(player.state, PlayerState::Playing);

        Ok(())
    }

    #[test]
    fn player_pause_sets_state_paused() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        assert!(player.get_player_state().is_none());
        player.play(&track, None)?;

        player.pause()?;

        assert_eq!(player.get_player_state().unwrap(), PlaybackState::Paused);
        assert_eq!(player.state, PlayerState::Paused);

        Ok(())
    }

    #[test]
    fn player_resume_sets_state_playing() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        assert!(player.get_player_state().is_none());
        player.play(&track, None)?;
        player.pause()?;

        player.resume()?;

        assert_eq!(player.get_player_state().unwrap(), PlaybackState::Playing);
        assert_eq!(player.state, PlayerState::Playing);

        Ok(())
    }

    #[test]
    fn player_stop_sets_state_paused_and_clears_track() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        assert!(player.get_player_state().is_none());
        player.play(&track, None)?;

        player.stop()?;

        assert!(player.get_player_state().is_none());
        assert_eq!(player.state, PlayerState::Paused);
        assert!(player.track.is_none());

        Ok(())
    }

    #[test]
    fn scrobble_not_ready_returns_false() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        player.play(&track, None)?;

        assert!(!player.scrobble());
        assert_eq!(
            player.track.as_ref().unwrap().scrobble_condition,
            Some(60.0)
        );

        Ok(())
    }

    #[test]
    fn scrobble_ready_returns_true_once() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        player.play(&track, None)?;

        player.track.as_mut().unwrap().scrobble_condition = Some(0.0);

        assert!(player.scrobble());
        assert!(player.should_scrobble().is_some());

        Ok(())
    }

    #[test]
    fn scrobble_after_already_scrobbled_returns_none() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        player.play(&track, None)?;

        player.track.as_mut().unwrap().scrobble_condition = Some(0.0);

        player.track.as_mut().unwrap().scrobbled = true;
        assert!(player.should_scrobble().is_none());

        Ok(())
    }

    #[test]
    fn seek_with_resume_true_updates_progress_and_resumes() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        player.play(&track, None)?;

        player.pause()?;

        player.seek(20.0, true)?;

        assert_eq!(player.track.as_ref().unwrap().sound_handle.pos, 20.0);
        assert_eq!(player.track.as_ref().unwrap().progress, 20.0);
        assert_eq!(player.state, PlayerState::Playing);

        Ok(())
    }

    #[test]
    fn seek_with_resume_false_updates_progress_but_pauses() -> Result<()> {
        let mut player = Player::new_mock()?;
        let track = get_track(0);

        player.play(&track, None)?;

        player.pause()?;

        player.seek(60.0, false)?;

        assert_eq!(player.track.as_ref().unwrap().sound_handle.pos, 60.0);
        assert_eq!(player.track.as_ref().unwrap().progress, 60.0);
        assert_eq!(player.state, PlayerState::Paused);

        Ok(())
    }
}
