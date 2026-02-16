use std::{cell::Cell, rc::Rc};

use gpui::{
    App, Bounds, ElementId, Entity, FocusHandle, InteractiveElement, IntoElement, KeyBinding,
    MouseButton, MouseEvent, ParentElement, Pixels, Point, Refineable, RenderOnce, StyleRefinement,
    Styled, Window, actions, canvas, div, relative, rems,
};

use crate::ui::AppStateContext;

actions!(slider, [Decrease, Increase, JumpStart, JumpEnd]);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", Decrease, None),
        KeyBinding::new("right", Increase, None),
        KeyBinding::new("home", JumpStart, None),
        KeyBinding::new("end", JumpEnd, None),
    ]);
}

#[derive(Clone, Copy)]
pub struct SliderState {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
}

impl Default for SliderState {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
        }
    }
}

impl SliderState {
    pub fn snap_to_step(&self, value: f64) -> f64 {
        let clamped = value.clamp(self.min, self.max);
        let steps = ((clamped - self.min) / self.step).round();

        self.min + steps * self.step
    }

    pub fn mouse_to_value(&self, mouse: Point<Pixels>, bounds: Bounds<Pixels>) -> f64 {
        let mouse_x = mouse.x - bounds.origin.x;
        let track_width = bounds.size.width;

        let normalized = (mouse_x / track_width).clamp(0.0, 1.0) as f64;
        self.snap_to_step(self.min + normalized * (self.max - self.min))
    }
}

type ChangeCallback = dyn Fn(f64, &mut App);

#[derive(Clone, IntoElement)]
pub struct Slider {
    id: ElementId,
    focus_handle: FocusHandle,
    state: SliderState,
    style: StyleRefinement,

    on_commit: Option<Rc<ChangeCallback>>,
    on_input: Option<Rc<ChangeCallback>>,
}

impl Slider {
    pub fn new(id: impl Into<ElementId> + Clone, focus_handle: FocusHandle) -> Self {
        Self {
            id: id.clone().into(),
            focus_handle,
            state: SliderState {
                ..SliderState::default()
            },
            style: StyleRefinement::default(),
            on_commit: None,
            on_input: None,
        }
    }

    pub fn on_commit(mut self, callback: impl Fn(f64, &mut App) + 'static) -> Self {
        self.on_commit = Some(Rc::new(callback));
        self
    }

    pub fn on_input(mut self, callback: impl Fn(f64, &mut App) + 'static) -> Self {
        self.on_input = Some(Rc::new(callback));
        self
    }

    pub fn min(mut self, min: f64) -> Self {
        self.state.min = min;
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.state.max = max;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.state.step = step;
        self
    }

    pub fn value(mut self, value: f64) -> Self {
        self.state.value = value;
        self
    }

    fn action<T>(
        &self,
        f: impl Fn(f64, SliderState) -> f64 + 'static,
    ) -> impl Fn(&T, &mut Window, &mut App) + 'static {
        let on_commit = self.on_commit.clone();
        let state = self.state;

        move |_event, _window, cx| {
            let new_value = f(state.value, state);

            if let Some(ref callback) = on_commit {
                callback(new_value, cx);
            }
        }
    }

    fn stop_drag_handler<T>(
        &self,
        is_dragging: &Entity<bool>,
        check_drag: bool,
    ) -> impl Fn(&T, &mut Window, &mut App) + 'static
    where
        T: MouseEvent,
    {
        let is_dragging = is_dragging.clone();
        let on_commit = self.on_commit.clone();
        let state = self.state;

        move |_event, _window, cx| {
            let should_stop = !check_drag || *is_dragging.read(cx);

            if should_stop {
                is_dragging.write(cx, false);
                if let Some(ref cb) = on_commit {
                    cb(state.value, cx);
                }
            }
        }
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_dragging = window.use_keyed_state(format!("{}-dragging", self.id), cx, |_, _| false);

        let theme = cx.app_theme();

        let track_bounds = Rc::new(Cell::new(Bounds::default()));

        let normalized_value =
            (self.state.value - self.state.min) / (self.state.max - self.state.min);

        let group_name = format!("{}:group", &self.id);
        let mut root = div()
            .id(self.id.clone())
            .track_focus(&self.focus_handle)
            .child(canvas(
                {
                    let track_bounds = track_bounds.clone();
                    move |bounds, _window, _cx| {
                        track_bounds.set(bounds);
                    }
                },
                |_bounds, _state, _window, _cx| {},
            ))
            .on_action::<Decrease>(self.action(|v, s| s.snap_to_step(v - s.step)))
            .on_action::<Increase>(self.action(|v, s| s.snap_to_step(v + s.step)))
            .on_action::<JumpStart>(self.action(|_, s| s.min))
            .on_action::<JumpEnd>(self.action(|_, s| s.max))
            .on_mouse_down(MouseButton::Left, {
                let track_bounds = track_bounds.clone();
                let is_dragging = is_dragging.clone();
                let on_input = self.on_input.clone();

                move |event, _window, cx| {
                    is_dragging.write(cx, true);
                    let new_value = self
                        .state
                        .mouse_to_value(event.position, track_bounds.get());

                    if let Some(ref cb) = on_input {
                        cb(new_value, cx);
                    }
                }
            })
            .on_mouse_up(
                MouseButton::Left,
                self.stop_drag_handler(&is_dragging, false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                self.stop_drag_handler(&is_dragging, true),
            )
            .on_mouse_move({
                move |event, _window, cx| {
                    if event.dragging() {
                        let new_value = self
                            .state
                            .mouse_to_value(event.position, track_bounds.get());

                        if let Some(ref cb) = self.on_input {
                            cb(new_value, cx);
                        }
                    }
                }
            })
            .group(&group_name)
            .min_w(rems(9.0))
            .h_2()
            .relative()
            .child(
                div()
                    .size_full()
                    .bg(theme.border.secondary.default)
                    .rounded_full()
                    .absolute(),
            )
            .child(
                div()
                    .w(relative(normalized_value as f32))
                    .h_full()
                    .bg(theme.text.secondary.default)
                    .rounded_full()
                    .absolute()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .id(format!("{}:thumb", &self.id))
                            .absolute()
                            .right(-rems(0.5))
                            .size_4()
                            .rounded_full()
                            .bg(theme.text.primary.default)
                            .border_1()
                            .border_color(theme.border.primary.default)
                            .opacity(0.0)
                            .group_hover(&group_name, |this| this.opacity(1.0))
                            .in_focus(|this| this.border_color(theme.text.primary.default)),
                    ),
            );

        root.style().refine(&self.style);

        root
    }
}
