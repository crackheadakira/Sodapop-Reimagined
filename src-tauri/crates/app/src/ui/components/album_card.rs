use common::Albums;
use gpui::{
    App, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce, Styled,
    StyledImage, Window, div, img, rems,
};

use crate::{
    events::{Route, UIUpdateEvent},
    ui::{AppStateContext, p, small},
};

#[derive(Clone, IntoElement)]
pub struct AlbumCard {
    pub album: Albums,
}

impl RenderOnce for AlbumCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.app_theme();
        let group_name = format!("card:album_{}", self.album.id);

        div()
            .group(&group_name)
            .on_mouse_down(MouseButton::Left, {
                let album_id = self.album.id.clone();
                move |_, _, cx| {
                    let state = cx.app_state();

                    state.ui_bus.emit(UIUpdateEvent::Navigate {
                        route: Route::Album { id: album_id },
                    });
                }
            })
            .cursor_pointer()
            .flex()
            .flex_col()
            .h(rems(17.5))
            .w_48()
            .pb_4()
            .gap_4()
            .child(
                img(self.album.cover_path)
                    .size_full()
                    .object_fit(gpui::ObjectFit::Cover)
                    .rounded_md()
                    .group_hover(&group_name, |this| this.opacity(0.9)),
            )
            .child(
                div()
                    .child(
                        p(self.album.name)
                            .id(format!("{group_name}:album_name"))
                            .truncate()
                            .text_color(theme.text.primary.default)
                            .group_hover(&group_name, |this| {
                                this.text_color(theme.text.primary.hovered)
                            }),
                    )
                    .child(
                        p(self.album.artist_name)
                            .id(format!("{group_name}:artist_name"))
                            .text_color(theme.text.secondary.default)
                            .truncate()
                            .group_hover(&group_name, |this| {
                                this.text_color(theme.text.secondary.hovered)
                            }),
                    )
                    .child(
                        small(self.album.album_type.to_string())
                            .id(format!("{group_name}:album_type"))
                            .text_color(theme.text.tertiary.default)
                            .group_hover(&group_name, |this| {
                                this.text_color(theme.text.tertiary.hovered)
                            }),
                    ),
            )
    }
}
