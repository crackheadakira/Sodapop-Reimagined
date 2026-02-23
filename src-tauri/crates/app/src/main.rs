// mod commands;
mod config;
mod discord;
mod error;
mod event_bus;
mod queue;
mod services;
mod state;
mod ui;

pub use config::VeilConfigChange;
pub(crate) use state::{AppState, VeilState};

pub mod events {
    pub use super::event_bus::{EventBus, EventSystemHandler};
    pub use super::queue::QueueEvent;
    pub use super::services::player::PlayerEvent;
    pub use super::services::ui::{Route, UIUpdateEvent};
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    logging::init();

    ui::app::run();
}
