use common::Tracks;

use crate::queue::RepeatMode;

#[derive(Clone, Copy)]
pub enum Route {
    Home,
    AllAlbums,
    Album { id: u32 },
    Playlist { id: u32 },
    Settings,
}

#[derive(Clone)]
pub enum PlayButtonState {
    Playing,
    Paused,
}

#[derive(Clone)]
pub enum UIUpdateEvent {
    /// Updates the state of the shuffle button
    ShuffleButton {
        enabled: bool,
    },

    LoopButton {
        mode: RepeatMode,
    },

    PlayButton {
        state: PlayButtonState,
    },

    TrackChange {
        track: Tracks,
    },

    ProgressUpdate {
        progress: f64,
    },

    Navigate {
        route: Route,
    },
}
