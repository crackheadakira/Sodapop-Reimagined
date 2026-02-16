use common::Albums;
use gpui::{App, Context, IntoElement, ParentElement, Render, Styled, Window, div, rems};

use crate::ui::{
    AlbumCard, AlbumCoverCacheProvider, AppStateContext, Theme, UniformGridScrollHandle, h6,
    uniform_grid,
};

#[derive(Clone)]
pub struct AllAlbumsView {
    albums: Vec<Albums>,
    scroll_handle: UniformGridScrollHandle,
}

impl AllAlbumsView {
    pub fn new(cx: &mut App) -> Self {
        let state = cx.app_state();
        let albums = state
            .db
            .all::<Albums>()
            .expect("failed to fetch all albums");

        Self {
            albums,
            scroll_handle: UniformGridScrollHandle::new(),
        }
    }
}

impl Render for AllAlbumsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .image_cache(AlbumCoverCacheProvider::new("cache:all_albums", 48))
            .size_full()
            .bg(theme.background.primary.default)
            .flex()
            .flex_col()
            .items_center()
            .gap_4()
            .child(
                h6(format!("{} albums", self.albums.len()))
                    .w_full()
                    .text_color(theme.text.primary.default),
            )
            .child(
                uniform_grid(
                    "all_albums_list",
                    self.albums.len(),
                    cx.processor(|this, range: std::ops::Range<usize>, _window, _cx| {
                        range
                            .map(|idx| AlbumCard {
                                album: this.albums[idx].clone(),
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .size_full()
                .gap(rems(1.0))
                .track_scroll(&self.scroll_handle)
                .preload_rows(2),
            )
    }
}
