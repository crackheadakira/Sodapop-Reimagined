use std::sync::Arc;

use crate::{
    VeilState,
    events::VeilConfigEvent,
    state::{AppState, handle_state_setup},
    ui::{
        Sidebar, Theme,
        assets::VeilAssetSource,
        slider,
        views::{
            AllAlbumsView, Home, ModalLayer, PlayerView,
            modal_layer::{self, GlobalModalLayer},
        },
    },
};
use gpui::{
    App, AppContext, Application, Bounds, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement, Point, Render, SharedString, Styled, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, actions, div, px, rems,
    size,
};
use logging::lock_or_log;

actions!(app, [Quit]);

pub fn run() {
    Application::new()
        .with_assets(VeilAssetSource)
        .run(|cx: &mut App| {
            cx.on_action(|_: &Quit, cx| cx.quit());

            handle_state_setup(cx).expect("Failed setting up the state");

            let theme = Theme::default();
            cx.set_global::<Theme>(theme);

            let modal_layer = cx.new(ModalLayer::new);
            cx.set_global(GlobalModalLayer(modal_layer));

            slider::bind_keys(cx);
            modal_layer::bind_keys(cx);

            cx.on_app_quit(|cx: &mut App| {
                let state = cx.app_state().clone();

                async move {
                    let (progress, volume) = {
                        let player = lock_or_log(state.player.read(), "Player Read Lock").unwrap();
                        (
                            player.track.as_ref().map(|t| t.progress),
                            player.get_volume(),
                        )
                    };

                    let mut config =
                        lock_or_log(state.config.write(), "Config Write Lock").unwrap();
                    let mut discord = lock_or_log(state.discord.lock(), "Discord Mutex").unwrap();

                    if config.integrations.discord_enabled {
                        discord.close();
                    };

                    state.db.shutdown().unwrap();

                    config
                        .update_config_and_write(VeilConfigEvent {
                            progress,
                            volume: Some(volume as f64),
                            ..VeilConfigEvent::default()
                        })
                        .expect("error writing config on app exit");
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1280.0), px(720.0)), cx);

            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_min_size: Some(size(px(1280.0), px(720.0))),
                    titlebar: Some(TitlebarOptions {
                        title: Some(SharedString::from("VEIL")),
                        appears_transparent: true,
                        traffic_light_position: Some(Point {
                            x: px(12.0),
                            y: px(11.0),
                        }),
                    }),
                    app_id: Some("org.crackheadakira.veil".to_owned()),
                    kind: WindowKind::Normal,
                    ..Default::default()
                },
                |window, cx| {
                    window.set_window_title("VEIL");

                    cx.new(AppWindow::new)
                },
            )
            .unwrap();
        });
}

#[derive(Clone, Copy)]
pub enum Route {
    Home,
    AllAlbums,
    Album { id: u32 },
    Playlist { id: u32 },
    Settings,
}

struct AppWindow {
    focus_handle: FocusHandle,
    all_albums_view: Option<Entity<AllAlbumsView>>,
    home: Option<Entity<Home>>,
    player_view: Entity<PlayerView>,
    modal_layer: Entity<ModalLayer>,
    route: Route,
}

impl AppWindow {
    fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let modal_layer = cx.global::<GlobalModalLayer>().0.clone();

        Self {
            focus_handle,
            all_albums_view: None,
            home: None,
            route: Route::Home,
            player_view: cx.new(PlayerView::new),
            modal_layer,
        }
    }

    fn navigate(&mut self, route: Route, cx: &mut Context<Self>) {
        self.route = route;
        cx.notify();
    }

    fn render_route(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.route {
            Route::Home => {
                let view = self
                    .home
                    .get_or_insert_with(|| cx.new(|_| Home::new(self.focus_handle.clone())));
                view.clone().into_any_element()
            }
            Route::AllAlbums => {
                let view = self
                    .all_albums_view
                    .get_or_insert_with(|| cx.new(|cx| AllAlbumsView::new(cx)));

                view.clone().into_any_element()
            }
            _ => div().into_any_element(),
        }
    }
}

impl Focusable for AppWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AppWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .size_full()
            .relative()
            .bg(theme.background.primary.default)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .size_full()
                    .child(Sidebar::new().on_navigate(cx.listener(
                        |this: &mut AppWindow, route: &Route, _, cx| {
                            this.navigate(*route, cx);
                        },
                    )))
                    .child(
                        div()
                            .flex_grow()
                            .p_8()
                            .max_w(rems(100.0))
                            .mx_auto()
                            .child(self.render_route(cx)),
                    ),
            )
            .child(self.player_view.clone())
            .child(self.modal_layer.clone())
    }
}

pub trait AppStateContext {
    fn app_state(&self) -> &Arc<VeilState>;
    fn app_theme(&self) -> &Theme;

    // TODO: maybe add more convenience methods
}

impl<V> AppStateContext for Context<'_, V> {
    fn app_state(&self) -> &Arc<VeilState> {
        &self.global::<AppState>().0
    }

    fn app_theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

impl AppStateContext for App {
    fn app_state(&self) -> &Arc<VeilState> {
        &self.global::<AppState>().0
    }

    fn app_theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}
