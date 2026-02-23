use gpui::{Global, InteractiveElement, Rgba, StatefulInteractiveElement, Styled};
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Default,
    Disabled,
    Hovered,
    Active,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Primary,
    Secondary,
    Tertiary,
}

#[derive(Clone, Deserialize)]
pub struct ColorSet {
    pub default: Rgba,
    pub disabled: Rgba,
    pub hovered: Rgba,
    pub active: Rgba,
}

impl ColorSet {
    /// Get a color dynamically by state
    pub fn get(&self, state: State) -> Rgba {
        match state {
            State::Default => self.default,
            State::Disabled => self.disabled,
            State::Hovered => self.hovered,
            State::Active => self.active,
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct ColorRole {
    pub primary: ColorSet,
    pub secondary: ColorSet,
    pub tertiary: ColorSet,
}

impl ColorRole {
    /// Get a color dynamically by state and level
    pub fn get(&self, level: Level, state: State) -> Rgba {
        match level {
            Level::Primary => self.primary.get(state),
            Level::Secondary => self.secondary.get(state),
            Level::Tertiary => self.tertiary.get(state),
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct Theme {
    pub text: ColorRole,

    pub background: ColorRole,

    pub border: ColorRole,
}

impl Global for Theme {}

impl Default for Theme {
    fn default() -> Self {
        Self {
            text: ColorRole {
                primary: ColorSet {
                    default: rgb(160, 159, 164),
                    disabled: rgb(123, 122, 129),
                    hovered: rgb(179, 178, 182),
                    active: rgb(198, 198, 201),
                },

                secondary: ColorSet {
                    default: rgb(104, 105, 112),
                    disabled: rgb(76, 77, 83),
                    hovered: rgb(133, 134, 140),
                    active: rgb(164, 165, 169),
                },

                tertiary: ColorSet {
                    default: rgb(71, 71, 73),
                    disabled: rgb(48, 48, 50),
                    hovered: rgb(95, 95, 97),
                    active: rgb(120, 120, 122),
                },
            },

            background: ColorRole {
                primary: ColorSet {
                    default: rgb(10, 10, 13),
                    disabled: rgb(5, 5, 7),
                    hovered: rgb(13, 14, 17),
                    active: rgb(18, 18, 23),
                },

                secondary: ColorSet {
                    default: rgb(11, 11, 14),
                    disabled: rgb(6, 6, 8),
                    hovered: rgb(17, 18, 22),
                    active: rgb(24, 24, 30),
                },

                tertiary: ColorSet {
                    default: rgb(13, 13, 16),
                    disabled: rgb(7, 7, 9),
                    hovered: rgb(19, 19, 24),
                    active: rgb(26, 26, 33),
                },
            },

            border: ColorRole {
                primary: ColorSet {
                    default: rgb(33, 31, 39),
                    disabled: rgb(20, 20, 26),
                    hovered: rgb(45, 43, 52),
                    active: rgb(57, 54, 66),
                },

                secondary: ColorSet {
                    default: rgb(38, 35, 46),
                    disabled: rgb(25, 23, 31),
                    hovered: rgb(52, 48, 62),
                    active: rgb(65, 61, 78),
                },

                tertiary: ColorSet {
                    default: rgb(45, 42, 53),
                    disabled: rgb(30, 28, 36),
                    hovered: rgb(61, 57, 72),
                    active: rgb(79, 71, 100),
                },
            },
        }
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

pub trait StyleFromColorSet {
    fn border_from(self, colors: &ColorSet) -> Self;
    fn text_from(self, colors: &ColorSet) -> Self;
}

impl<T> StyleFromColorSet for T
where
    T: Styled + InteractiveElement + StatefulInteractiveElement,
{
    fn border_from(self, colors: &ColorSet) -> Self {
        self.border_color(colors.default)
            .hover(|s| s.border_color(colors.hovered))
            .active(|s| s.border_color(colors.active))
    }

    fn text_from(self, colors: &ColorSet) -> Self {
        self.text_color(colors.default)
            .hover(|s| s.text_color(colors.hovered))
            .focus_visible(|s| s.text_color(colors.hovered))
            .active(|s| s.text_color(colors.active))
    }
}

pub mod text_elements {
    use gpui::{Div, FontWeight, ParentElement, Styled, div, rems};

    pub fn h1(text: impl Into<String>) -> Div {
        div()
            .font_weight(FontWeight::BOLD)
            .text_size(rems(2.15))
            .child(text.into())
    }

    pub fn h2(text: impl Into<String>) -> Div {
        div()
            .font_weight(FontWeight::BOLD)
            .text_size(rems(1.925))
            .child(text.into())
    }

    pub fn h3(text: impl Into<String>) -> Div {
        div()
            .font_weight(FontWeight::BOLD)
            .text_size(rems(1.725))
            .child(text.into())
    }

    pub fn h4(text: impl Into<String>) -> Div {
        div()
            .font_weight(FontWeight::SEMIBOLD)
            .text_size(rems(1.55))
            .child(text.into())
    }

    pub fn h5(text: impl Into<String>) -> Div {
        div()
            .font_weight(FontWeight::SEMIBOLD)
            .text_size(rems(1.39375))
            .child(text.into())
    }

    pub fn h6(text: impl Into<String>) -> Div {
        div()
            .font_weight(FontWeight::MEDIUM)
            .text_size(rems(1.25))
            .child(text.into())
    }

    pub fn p(text: impl Into<String>) -> Div {
        div()
            .font_weight(FontWeight::MEDIUM)
            .text_size(rems(0.8875))
            .child(text.into())
    }

    pub fn small(text: impl Into<String>) -> Div {
        div()
            .font_weight(FontWeight::MEDIUM)
            .text_size(rems(0.8125))
            .child(text.into())
    }
}
