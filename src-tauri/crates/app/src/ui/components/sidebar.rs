use std::rc::Rc;

use gpui::{
    App, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::events::Route;
use crate::ui::{AppStateContext, StyleFromColorSet, Theme, small};

type NavigateHandler = Rc<dyn Fn(&Route, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Sidebar {
    on_navigate: Option<NavigateHandler>,
}

impl Sidebar {
    pub fn new() -> Self {
        Self { on_navigate: None }
    }

    pub fn on_navigate(
        mut self,
        handler: impl Fn(&Route, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_navigate = Some(Rc::new(handler));
        self
    }

    fn route_button(
        theme: &Theme,
        label: impl Into<SharedString> + IntoElement + Clone,
        route: Route,
        navigate: Option<NavigateHandler>,
    ) -> impl IntoElement + InteractiveElement {
        div()
            .id(format!("sidebar:{}", label.clone().into()))
            .text_from(&theme.text.tertiary)
            .on_click({
                let navigate = navigate.clone();
                move |_, window, app| {
                    if let Some(handler) = &navigate {
                        handler(&route, window, app);
                    }
                }
            })
            .cursor_pointer()
            .child(label)
    }
}

impl RenderOnce for Sidebar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.app_theme();
        let navigate = self.on_navigate.clone();

        div()
            .id("sidebar")
            .overflow_y_scroll()
            .flex_shrink_0()
            .h_full()
            .w_72()
            .p_8()
            .gap_8()
            .bg(theme.background.primary.default)
            .border_r_1()
            .border_color(theme.border.secondary.default)
            .text_color(theme.text.primary.default)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(small("General").text_color(theme.text.tertiary.default))
                    .child(div().flex().flex_col().gap_4().px_2().children(vec![
                        Self::route_button(theme, "Home", Route::Home, navigate.clone()),
                        Self::route_button(theme, "Albums", Route::AllAlbums, navigate.clone()),
                    ])),
            )
    }
}
