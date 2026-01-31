//! Simple notifications library for egui.

#![warn(missing_docs)]

mod toast;
pub use toast::*;
mod anchor;
pub use anchor::*;

#[doc(hidden)]
pub use egui::__run_test_ctx;
use egui::text::TextWrapping;
use egui::{
    vec2, Align, Color32, Context, CornerRadius, FontId, FontSelection, Id, LayerId, Order,
    Shadow, Stroke, TextWrapMode, Vec2, WidgetText,
};

pub(crate) const TOAST_WIDTH: f32 = 180.;
pub(crate) const TOAST_HEIGHT: f32 = 34.;

const WARNING_COLOR: Color32 = Color32::from_rgb(230, 220, 140);

/// Main notifications collector.
/// # Usage
/// You need to create [`Toasts`] once and call `.show(ctx)` in every frame.
/// ```
/// # use std::time::Duration;
/// use egui_notify::Toasts;
///
/// # egui_notify::__run_test_ctx(|ctx| {
/// let mut t = Toasts::default();
/// t.info("Hello, World!").duration(Some(Duration::from_secs(5))).closable(true);
/// // More app code
/// t.show(ctx);
/// # });
/// ```
pub struct Toasts {
    toasts: Vec<Toast>,
    anchor: Anchor,
    margin: Vec2,
    spacing: f32,
    padding: Vec2,
    reverse: bool,
    speed: f32,
    font: Option<FontId>,
    shadow: Option<Shadow>,
    held: bool,
}

impl Toasts {
    /// Creates new [`Toasts`] instance.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            anchor: Anchor::TopRight,
            margin: vec2(8., 8.),
            toasts: vec![],
            spacing: 8.,
            padding: vec2(10., 10.),
            held: false,
            speed: 4.,
            reverse: false,
            font: None,
            shadow: None,
        }
    }

    /// Adds new toast to the collection.
    /// By default adds toast at the end of the list, can be changed with `self.reverse`.
    #[allow(clippy::unwrap_used)] // We know that the index is valid
    pub fn add(&mut self, toast: Toast) -> &mut Toast {
        if self.reverse {
            self.toasts.insert(0, toast);
            return self.toasts.get_mut(0).unwrap();
        }
        self.toasts.push(toast);
        let l = self.toasts.len() - 1;
        self.toasts.get_mut(l).unwrap()
    }

    /// Dismisses the oldest toast
    pub fn dismiss_oldest_toast(&mut self) {
        if let Some(toast) = self.toasts.get_mut(0) {
            toast.dismiss();
        }
    }

    /// Dismisses the most recent toast
    pub fn dismiss_latest_toast(&mut self) {
        if let Some(toast) = self.toasts.last_mut() {
            toast.dismiss();
        }
    }

    /// Dismisses all toasts
    pub fn dismiss_all_toasts(&mut self) {
        for toast in &mut self.toasts {
            toast.dismiss();
        }
    }

    /// Returns the number of toast items.
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    /// Returns `true` if there are no toast items.
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// Shortcut for adding a toast with info `success`.
    pub fn success(&mut self, caption: impl Into<WidgetText>) -> &mut Toast {
        self.add(Toast::success(caption))
    }

    /// Shortcut for adding a toast with info `level`.
    pub fn info(&mut self, caption: impl Into<WidgetText>) -> &mut Toast {
        self.add(Toast::info(caption))
    }

    /// Shortcut for adding a toast with warning `level`.
    pub fn warning(&mut self, caption: impl Into<WidgetText>) -> &mut Toast {
        self.add(Toast::warning(caption))
    }

    /// Shortcut for adding a toast with error `level`.
    pub fn error(&mut self, caption: impl Into<WidgetText>) -> &mut Toast {
        self.add(Toast::error(caption))
    }

    /// Shortcut for adding a toast with no level.
    pub fn basic(&mut self, caption: impl Into<WidgetText>) -> &mut Toast {
        self.add(Toast::basic(caption))
    }

    /// Shortcut for adding a toast with custom `level`.
    pub fn custom(
        &mut self,
        caption: impl Into<WidgetText>,
        level_string: String,
        level_color: egui::Color32,
    ) -> &mut Toast {
        self.add(Toast::custom(
            caption,
            ToastLevel::Custom(level_string, level_color),
        ))
    }

    /// Should toasts be added in reverse order?
    pub const fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Where toasts should appear.
    pub const fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Sets spacing between adjacent toasts.
    pub const fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Margin or distance from screen to toasts' bounding boxes
    pub const fn with_margin(mut self, margin: Vec2) -> Self {
        self.margin = margin;
        self
    }

    /// Enables the use of a shadow for toasts.
    pub const fn with_shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    /// Padding or distance from toasts' bounding boxes to inner contents.
    pub const fn with_padding(mut self, padding: Vec2) -> Self {
        self.padding = padding;
        self
    }

    /// Changes the default font used for all toasts.
    pub fn with_default_font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
}

impl Toasts {
    /// Displays toast queue
    pub fn show(&mut self, ctx: &Context) {
        let Self {
            anchor,
            margin,
            spacing,
            padding,
            toasts,
            held,
            speed,
            ..
        } = self;

        let mut pos = anchor.screen_corner(ctx.input(|i| i.content_rect().max), *margin);
        let p = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("toasts")));

        // `held` used to prevent sticky removal
        if ctx.input(|i| i.pointer.primary_released()) {
            *held = false;
        }

        let visuals = ctx.style().visuals.widgets.noninteractive;
        let mut update = false;

        toasts.retain_mut(|toast| {
            // Start disappearing expired toasts
            if let Some((_initial_d, current_d)) = toast.duration {
                if current_d <= 0. {
                    toast.state = ToastState::Disappear;
                }
            }

            let anim_offset = toast.width * (1. - ease_in_cubic(toast.value));
            pos.x += anim_offset * anchor.anim_side();

            // Calculate caption galley first to determine dimensions
            let caption_galley = toast.caption.clone().into_galley_impl(
                ctx,
                ctx.style().as_ref(),
                TextWrapping::from_wrap_mode_and_width(TextWrapMode::Extend, f32::INFINITY),
                FontSelection::Default,
                Align::LEFT,
            );

            let (caption_width, caption_height) =
                (caption_galley.rect.width(), caption_galley.rect.height());

            let rounding = CornerRadius::same(4);

            // Update toast dimensions BEFORE calculating rect
            toast.width = padding.x.mul_add(2., caption_width);
            let progress_bar_height = if toast.show_progress_bar { 6.0 } else { 0.0 };
            let content_height = padding.y.mul_add(2., caption_height);
            toast.height = content_height + progress_bar_height;

            // Now calculate rect with correct dimensions
            let rect = toast.calc_anchored_rect(pos, *anchor);

            if let Some((_, d)) = toast.duration.as_mut() {
                // Check if we hover over the toast and if true don't decrease the duration
                let hover_pos = ctx.input(|i| i.pointer.hover_pos());
                let is_outside_rect = hover_pos.is_none_or(|pos| !rect.contains(pos));

                if is_outside_rect && toast.state.idling() {
                    *d -= ctx.input(|i| i.stable_dt);
                    update = true;
                }
            }

            // Required due to positioning of the next toast
            pos.x -= anim_offset * anchor.anim_side();

            // Draw shadow
            if let Some(shadow) = self.shadow {
                let s = shadow.as_shape(rect, rounding);
                p.add(s);
            }

            // Draw background + border
            p.rect_filled(rect, rounding, visuals.bg_fill);
            {
                let stroke = Stroke { width: 1.0, color: visuals.bg_stroke.color };
                // Top
                p.line_segment([
                    rect.min,
                    egui::pos2(rect.max.x, rect.min.y)
                ], stroke);
                // Bottom
                p.line_segment([
                    egui::pos2(rect.min.x, rect.max.y),
                    rect.max
                ], stroke);
                // Left
                p.line_segment([
                    rect.min,
                    egui::pos2(rect.min.x, rect.max.y)
                ], stroke);
                // Right
                p.line_segment([
                    egui::pos2(rect.max.x, rect.min.y),
                    rect.max
                ], stroke);
            }

            // Calculate vertical offset for content centering (same for all elements)
            let content_oy = content_height / 2. - caption_height / 2.;

            // Paint caption (centered in content area, above progress bar)
            let ox = toast.width / 2. - caption_width / 2.;
            p.galley(
                rect.min + vec2(ox, content_oy),
                caption_galley,
                visuals.fg_stroke.color,
            );

            // Click anywhere to dismiss + pointer cursor
            if let Some(hover_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                if rect.contains(hover_pos) {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    if ctx.input(|i| i.pointer.primary_clicked()) && !*held {
                        toast.dismiss();
                        *held = true;
                    }
                }
            }

            // Draw duration
            if toast.show_progress_bar {
                if let Some((initial, current)) = toast.duration {
                    if !toast.state.disappearing() {
                        let bar_height = 6.0;
                        p.line_segment(
                            [
                                rect.min + vec2(0., toast.height - bar_height / 2.),
                                rect.max - vec2((1. - (current / initial)) * toast.width, bar_height / 2.),
                            ],
                            Stroke::new(bar_height, WARNING_COLOR),
                        );
                    }
                }
            }

            toast.adjust_next_pos(&mut pos, *anchor, *spacing);

            // Animations
            if toast.state.appearing() {
                update = true;
                toast.value += ctx.input(|i| i.stable_dt) * (*speed);

                if toast.value >= 1. {
                    toast.value = 1.;
                    toast.state = ToastState::Idle;
                }
            } else if toast.state.disappearing() {
                update = true;
                toast.value -= ctx.input(|i| i.stable_dt) * (*speed);

                if toast.value <= 0. {
                    toast.state = ToastState::Disappeared;
                }
            }

            // Remove disappeared toasts
            !toast.state.disappeared()
        });

        if update {
            ctx.request_repaint();
        }
    }
}

impl Default for Toasts {
    fn default() -> Self {
        Self::new()
    }
}

fn ease_in_cubic(x: f32) -> f32 {
    1. - (1. - x).powi(3)
}
