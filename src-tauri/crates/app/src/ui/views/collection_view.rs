use common::{AlbumWithTracks, PlaylistWithTracks, Tracks};
use gpui::{
    App, Context, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    UniformListScrollHandle, Window, div, img, prelude::FluentBuilder, uniform_list,
};

use crate::ui::{
    AppStateContext, Icon, IconVariants, h2, h6, p, small, views::player::format_time,
};

pub struct CollectionView {
    pub data: CollectionData,
    scroll_handle: UniformListScrollHandle,
}

impl CollectionView {
    pub fn new(cx: &mut App, id: u32, kind: CollectionKind) -> Self {
        let state = cx.app_state();

        let data = match kind {
            CollectionKind::Album => {
                let album = state
                    .db
                    .album_with_tracks(&id)
                    .expect("failed to fetch album with tracks");
                album.into()
            }
            CollectionKind::Playlist => {
                let playlist = state
                    .db
                    .get_playlist_with_tracks(&id)
                    .expect("failed to fetch playlist with tracks");
                playlist.into()
            }
        };

        Self {
            data,
            scroll_handle: UniformListScrollHandle::new(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum CollectionKind {
    Album,
    Playlist,
}

impl Into<String> for CollectionKind {
    fn into(self) -> String {
        match self {
            CollectionKind::Album => "Album",
            CollectionKind::Playlist => "Playlist",
        }
        .to_owned()
    }
}

pub struct CollectionData {
    pub id: u32,
    title: String,

    /// Description or album type
    subtitle: Option<String>,

    cover_path: String,
    tracks: Vec<Tracks>,
    year: Option<u16>,
    duration: Option<u32>,
    kind: CollectionKind,
}

impl From<AlbumWithTracks> for CollectionData {
    fn from(a: AlbumWithTracks) -> Self {
        Self {
            id: a.album.id,
            title: a.album.name,
            subtitle: Some(a.album.artist_name),
            cover_path: a.album.cover_path,
            tracks: a.tracks,
            year: Some(a.album.year),
            duration: Some(a.album.duration),
            kind: CollectionKind::Album,
        }
    }
}

impl From<PlaylistWithTracks> for CollectionData {
    fn from(p: PlaylistWithTracks) -> Self {
        let subtitle = if p.playlist.description.is_empty() {
            None
        } else {
            Some(p.playlist.description)
        };

        Self {
            id: p.playlist.id,
            title: p.playlist.name,
            subtitle,
            cover_path: p.playlist.cover_path,
            tracks: p.tracks,
            year: None,
            duration: None,
            kind: CollectionKind::Playlist,
        }
    }
}

impl Render for CollectionView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.app_theme();

        let song_or_songs = if self.data.tracks.len() > 1 {
            "songs"
        } else {
            "song"
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_8()
            .child(
                div()
                    .bg(theme.background.secondary.default)
                    .border_color(theme.border.primary.default)
                    .border_1()
                    .flex()
                    .items_center()
                    .gap_8()
                    .rounded_md()
                    .p_8()
                    .child(
                        img(self.data.cover_path.clone())
                            .border_1()
                            .border_color(theme.border.secondary.default)
                            .size_48()
                            .rounded_md(),
                    )
                    .child(
                        div().flex().flex_col().gap_4().child(
                            div().flex().cursor_default().flex_col().gap_1().child(
                                div()
                                    .child(
                                        h6(self.data.kind.clone())
                                            .text_color(theme.text.tertiary.default),
                                    )
                                    .child(
                                        h2(self.data.title.clone())
                                            .text_color(theme.text.primary.default),
                                    )
                                    .when_some(self.data.subtitle.clone(), |this, subtitle| {
                                        this.child(
                                            p(subtitle).text_color(theme.text.secondary.default),
                                        )
                                    })
                                    .when_some(self.data.duration.clone(), |this, duration| {
                                        this.child(
                                            small(format!(
                                                "{}, {} {song_or_songs}",
                                                format_time(duration as f64),
                                                self.data.tracks.len()
                                            ))
                                            .text_color(theme.text.tertiary.default),
                                        )
                                    })
                                    .when_none(&self.data.duration, |this| {
                                        this.child(
                                            small(format!(
                                                "{} {song_or_songs}",
                                                self.data.tracks.len()
                                            ))
                                            .text_color(theme.text.tertiary.default),
                                        )
                                    })
                                    .when_some(self.data.year.clone(), |this, year| {
                                        this.child(
                                            small(year.to_string())
                                                .text_color(theme.text.tertiary.default),
                                        )
                                    }),
                            ),
                        ),
                    ),
            )
            .child(
                div()
                    .bg(theme.background.secondary.default)
                    .border_1()
                    .border_color(theme.border.primary.default)
                    .flex()
                    .flex_col()
                    .rounded_md()
                    .p_4()
                    .h_full()
                    .child(
                        div()
                            .text_color(theme.text.tertiary.default)
                            .border_b_1()
                            .border_color(theme.border.secondary.default)
                            .mb_4()
                            .flex()
                            .gap_4()
                            .p_3()
                            .px_4()
                            .children([
                                small("#"),
                                small("Title").flex_grow(),
                                small("Album").flex_grow(),
                            ])
                            .child(
                                Icon::new(IconVariants::Clock)
                                    .size_4()
                                    .col_end(-1)
                                    .text_color(theme.text.secondary.default),
                            ),
                    )
                    // TODO: fix issue with height being full, not conforming to the tracks if theres not
                    // enough to fill out screen.
                    .child(
                        uniform_list(
                            format!("track_list:album_{}", self.data.id),
                            self.data.tracks.len(),
                            cx.processor(|this, range, _, cx| {
                                let mut track_divs = Vec::new();
                                let theme = cx.app_theme();

                                for idx in range {
                                    let track: &Tracks = &this.data.tracks[idx];
                                    let group_name = format!("track_list:group_{}", track.id);

                                    let track_item = div()
                                        .group(group_name)
                                        .hover(|this| this.bg(theme.background.secondary.hovered))
                                        .p_3()
                                        .px_4()
                                        .flex()
                                        .w_full()
                                        .h_16()
                                        .child(
                                            p(track.name.clone())
                                                .text_color(theme.text.primary.default),
                                        );

                                    track_divs.push(track_item);
                                }

                                track_divs
                            }),
                        )
                        .w_full()
                        .h_full()
                        .track_scroll(&self.scroll_handle),
                    ),
            )
    }
}
