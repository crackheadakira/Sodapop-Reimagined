use gpui::{IntoElement, RenderOnce, SharedString, StyleRefinement, Styled, Svg, svg};

#[derive(Clone, Copy)]
pub enum IconVariants {
    Add,
    AddSquareMultiple,
    ArrowRepeat1,
    ArrowRepeatAll,
    ArrowShuffle,
    Checkmark,
    ChevronRight,
    Clock,
    Delete,
    Dismiss,
    Edit,
    ErrorCircle,
    Folder,
    Info,
    Key,
    LayoutRowTwo,
    Maximize,
    Next,
    Pause,
    Play,
    Previous,
    Search,
    SearchAlt,
    Speaker,
    SpeakerMute,
    StarAdd,
    Subtract,
    Warning,
    WindowNew,
}

impl From<IconVariants> for SharedString {
    fn from(val: IconVariants) -> Self {
        let file_name = match val {
            IconVariants::Add => "FluentAdd24Regular",
            IconVariants::AddSquareMultiple => "FluentAddSquareMultiple24Regular",
            IconVariants::ArrowRepeat1 => "FluentArrowRepeat116Filled",
            IconVariants::ArrowRepeatAll => "FluentArrowRepeatAll16Filled",
            IconVariants::ArrowShuffle => "FluentArrowShuffle16Filled",
            IconVariants::Checkmark => "FluentCheckmark20Filled",
            IconVariants::ChevronRight => "FluentChevronRight24Regular",
            IconVariants::Clock => "FluentClock12Regular",
            IconVariants::Delete => "FluentDelete24Regular",
            IconVariants::Dismiss => "FluentDismiss20Filled",
            IconVariants::Edit => "FluentEdit24Filled",
            IconVariants::ErrorCircle => "FluentErrorCircle20Filled",
            IconVariants::Folder => "FluentFolder24Filled",
            IconVariants::Info => "FluentInfo20Filled",
            IconVariants::Key => "FluentKey24Filled",
            IconVariants::LayoutRowTwo => "FluentLayoutRowTwo16Filled",
            IconVariants::Maximize => "FluentMaximize20Filled",
            IconVariants::Next => "FluentNext16Filled",
            IconVariants::Pause => "FluentPause16Filled",
            IconVariants::Play => "FluentPlay16Filled",
            IconVariants::Previous => "FluentPrevious16Filled",
            IconVariants::Search => "FluentSearch12Filled",
            IconVariants::SearchAlt => "FluentSearch20Filled",
            IconVariants::Speaker => "FluentSpeaker16Regular",
            IconVariants::SpeakerMute => "FluentSpeakerMute16Regular",
            IconVariants::StarAdd => "FluentStarAdd24Regular",
            IconVariants::Subtract => "FluentSubtract20Filled",
            IconVariants::Warning => "FluentWarning20Filled",
            IconVariants::WindowNew => "FluentWindowNew24Filled",
        };

        format!("embedded://icons/{file_name}.svg").into()
    }
}

#[derive(IntoElement)]
pub struct Icon {
    svg: Svg,
    icon: IconVariants,
}

impl Icon {
    pub fn new(icon: IconVariants) -> Self {
        Self { svg: svg(), icon }
    }
}

impl Styled for Icon {
    fn style(&mut self) -> &mut StyleRefinement {
        self.svg.style()
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        self.svg.path(self.icon)
    }
}
