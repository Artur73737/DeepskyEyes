//! Native desktop window matching Rust_App/UI_Preview/preview.png:
//! deep-navy three-column dashboard, beige-white text and dividers,
//! 13 px corner radius on every card, fixed sidebars with a flexible center
//! (responsive: sidebars keep their width, the preview column absorbs resize).
use crate::bridge::*;
use gpui::{prelude::*, *};
use std::{
    sync::{
        mpsc::{Receiver, Sender, TryRecvError},
        Arc,
    },
    time::Duration,
};

/// Enter the native event loop on the main OS thread. The caller owns its worker
/// and both channel counterparts. Closing the window emits `Shutdown`.
pub fn run(initial: UiSnapshot, snapshots: Receiver<UiSnapshot>, actions: Sender<UiAction>) {
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let close = actions.clone();
        cx.on_window_closed(move |cx| {
            if cx.windows().is_empty() {
                let _ = close.send(UiAction::Shutdown);
                cx.quit();
            }
        })
        .detach();
        if let Err(error) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(960.), px(620.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("DeepskyEyes · Mission control".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |_, cx| cx.new(|cx| Desktop::new(initial, snapshots, actions, cx)),
        ) {
            eprintln!("Cannot open DeepskyEyes window: {error}");
            cx.quit();
        }
        // Native Windows caption stays in place: minimize / maximize / close
        // and window drag are the operating system's own controls.
        cx.activate(true);
    });
}

// ---------------------------------------------------------------------------
// Theme: deep navy + beige-white, 13 px radius everywhere (preview.png).
// ---------------------------------------------------------------------------
const RADIUS: f32 = 13.0;
const BG: u32 = 0x060c17;
const PANEL: u32 = 0x0a1322;
const OVERLAY: u32 = 0x09111e;
const PANEL_ACTIVE: u32 = 0x15294a;
const EDGE: u32 = 0x233049;
const TEXT: u32 = 0xf2e9d6;
const MUTED: u32 = 0x8e99ad;
const BEIGE: u32 = 0xf0dfbc;
const INK: u32 = 0x0a1322;
const GREEN: u32 = 0x35d07f;
const BLUE: u32 = 0x3f8cff;
const TOGGLE_ON: u32 = 0x2f7bff;
const TOGGLE_OFF: u32 = 0x3a4356;
const DANGER: u32 = 0xff9580;

fn rad() -> Pixels {
    px(RADIUS)
}

/// Base card: navy panel, 1 px steel edge, 13 px corners.
fn panel() -> Div {
    div()
        .flex()
        .flex_col()
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(EDGE))
        .rounded(rad())
}

fn divider() -> Div {
    div().h(px(1.)).w_full().bg(rgb(EDGE))
}

fn dot(color: u32) -> Div {
    div().text_color(rgb(color)).child("●")
}

fn fmt_exposure(ns: u64) -> String {
    if ns % 1_000_000_000 == 0 {
        format!("{} s", ns / 1_000_000_000)
    } else {
        format!("{:.3} s", ns as f64 / 1e9)
    }
}

fn fmt_integration(frames: u32, exposure_ns: u64) -> String {
    let seconds = u64::from(frames) * exposure_ns / 1_000_000_000;
    format!("{} s ({} min)", seconds, seconds / 60)
}

/// Next announced WB preset after the current one (wraps around).
fn next_wb_preset(presets: &[String], current: &str) -> String {
    if presets.is_empty() {
        return current.to_string();
    }
    let pos = presets
        .iter()
        .position(|m| m == current)
        .map(|i| (i + 1) % presets.len())
        .unwrap_or(0);
    presets[pos].clone()
}

// ---------------------------------------------------------------------------
// View state.
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq)]
enum Page {
    Dashboard,
    Camera,
    Preview,
    Sequence,
    Calibration,
    Sessions,
    Diagnostics,
    Settings,
}
impl Page {
    fn title(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Camera => "Acquisition",
            Self::Preview => "Preview",
            Self::Sequence => "Sequence",
            Self::Calibration => "Calibration",
            Self::Sessions => "Sessions",
            Self::Diagnostics => "Diagnostics",
            Self::Settings => "Settings",
        }
    }
}

struct Desktop {
    state: UiSnapshot,
    actions: Sender<UiAction>,
    page: Page,
    /// Active slider drag: which slider, grab x in window px, start fraction.
    slider_drag: Option<(SliderId, f32, f64)>,
    image: Option<Arc<Image>>,
    image_revision: Option<u64>,
    preview_cover: bool,
    error: String,
    _poll: Task<()>,
}

/// Slider target. One active drag at a time; ids keep sliders apart.
#[derive(Clone, Copy, PartialEq)]
enum SliderId {
    Exposure,
    Iso,
    Focus,
    Zoom,
    Frames,
}

/// Value slider over an announced numeric range. Relative drag: ~200 px
/// covers the full span (logarithmic when `log`, for wide ranges like
/// exposure or ISO). Emits only on real change; the runtime validates
/// every request against the announced range.
#[allow(clippy::too_many_arguments)]
fn slider(
    dom_id: &'static str,
    id: SliderId,
    range: Option<(f64, f64)>,
    value: f64,
    log: bool,
    enabled: bool,
    cx: &mut Context<Desktop>,
) -> Stateful<Div> {
    fn map(lo: f64, hi: f64, log: bool, frac: f64) -> f64 {
        let t = frac.clamp(0.0, 1.0);
        if log && lo > 0.0 && hi > lo {
            lo * (hi / lo).powf(t)
        } else {
            lo + t * (hi - lo).max(f64::EPSILON)
        }
    }
    fn unmap(lo: f64, hi: f64, log: bool, value: f64) -> f64 {
        if log && lo > 0.0 && hi > lo && value > 0.0 {
            ((value / lo).ln() / (hi / lo).ln()).clamp(0.0, 1.0)
        } else {
            ((value - lo) / (hi - lo).max(f64::EPSILON)).clamp(0.0, 1.0)
        }
    }
    let (lo, hi) = range.unwrap_or((0.0, 1.0));
    let span = (hi - lo).max(f64::EPSILON);
    let enabled = enabled && range.is_some();
    let frac = unmap(lo, hi, log, value) as f32;
    let mk = move |v: f64| match id {
        SliderId::Exposure => UiAction::SetExposure(v as u64),
        SliderId::Iso => UiAction::SetIso(v as u32),
        SliderId::Focus => UiAction::SetFocus(v as f32),
        SliderId::Zoom => UiAction::SetZoom(v as f32),
        SliderId::Frames => UiAction::SetFrameCount(v.max(1.0) as u32),
    };
    div()
        .id(dom_id)
        .flex()
        .flex_row()
        .items_center()
        .h(px(22.))
        .cursor_pointer()
        .child(
            div()
                .flex_1()
                .h(px(6.))
                .rounded(px(999.))
                .bg(rgb(PANEL_ACTIVE))
                .child(
                    div()
                        .h(px(6.))
                        .rounded(px(999.))
                        .bg(rgb(BLUE))
                        .w(relative(frac)),
                ),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view, event: &MouseDownEvent, _, _| {
                if enabled {
                    view.slider_drag =
                        Some((id, f32::from(event.position.x), unmap(lo, hi, log, value)));
                }
            }),
        )
        .on_mouse_move(cx.listener(move |view, event: &MouseMoveEvent, _, cx| {
            match event.pressed_button {
                Some(MouseButton::Left) => {
                    if !enabled {
                        return;
                    }
                    if let Some((drag_id, start_x, start_frac)) = view.slider_drag {
                        if drag_id != id {
                            return;
                        }
                        let here = start_frac
                            + (f64::from(f32::from(event.position.x)) - start_x as f64) / 200.0;
                        let next = map(lo, hi, log, (here * 200.0).round() / 200.0);
                        if (next - value).abs() > span / 2000.0 {
                            view.send(mk(next), cx);
                        }
                    }
                }
                _ => {
                    if view.slider_drag.is_some_and(|(drag_id, _, _)| drag_id == id) {
                        view.slider_drag = None;
                    }
                }
            }
        }))
}

impl Desktop {
    fn new(
        state: UiSnapshot,
        snapshots: Receiver<UiSnapshot>,
        actions: Sender<UiAction>,
        cx: &mut Context<Self>,
    ) -> Self {
        let poll = cx.spawn(async move |view, cx| {
            loop {
                Timer::after(Duration::from_millis(33)).await;
                let mut latest = None;
                let mut ended = false;
                // Bound each drain so a fast producer cannot starve the UI.
                for _ in 0..64 {
                    match snapshots.try_recv() {
                        Ok(s) => latest = Some(s),
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            ended = true;
                            break;
                        }
                    }
                }
                if view
                    .update(cx, |view, cx| {
                        if let Some(mut s) = latest {
                            // Camera I/O can outlive several pointer events. Do not
                            // roll the active drag back to an older worker value.
                            if let Some((id, _, _)) = view.slider_drag {
                                match id {
                                    SliderId::Exposure => s.exposure_ns = view.state.exposure_ns,
                                    SliderId::Iso => s.iso = view.state.iso,
                                    SliderId::Focus => s.focus_diopters = view.state.focus_diopters,
                                    SliderId::Zoom => s.zoom = view.state.zoom,
                                    SliderId::Frames => s.frames_total = view.state.frames_total,
                                }
                            }
                            view.state = s;
                            view.update_image();
                            cx.notify();
                        }
                        if ended {
                            view.error = "Application worker disconnected".into();
                            cx.notify();
                        }
                    })
                    .is_err()
                    || ended
                {
                    break;
                }
            }
        });
        let mut view = Self {
            state,
            actions,
            page: Page::Dashboard,
            slider_drag: None,
            image: None,
            image_revision: None,
            preview_cover: true,
            error: String::new(),
            _poll: poll,
        };
        view.update_image();
        view
    }

    fn update_image(&mut self) {
        if let Some(preview) = &self.state.preview {
            if self.image_revision != Some(preview.revision) {
                let format = if preview.bytes.starts_with(b"\x89PNG") {
                    ImageFormat::Png
                } else {
                    ImageFormat::Jpeg
                };
                self.image = Some(Arc::new(Image::from_bytes(format, preview.bytes.to_vec())));
                self.image_revision = Some(preview.revision);
            }
        } else {
            self.image = None;
            self.image_revision = None;
        }
    }

    fn send(&mut self, action: UiAction, cx: &mut Context<Self>) {
        // Pending requests render immediately; applied values remain exclusively
        // authoritative and are only populated by the backend snapshot.
        match &action {
            UiAction::SetExposure(v) => self.state.exposure_ns = *v,
            UiAction::SetIso(v) => self.state.iso = *v,
            UiAction::SetFocus(v) => self.state.focus_diopters = *v,
            UiAction::SetZoom(v) => self.state.zoom = *v,
            UiAction::SetFrameCount(v) => self.state.frames_total = *v,
            _ => {}
        }
        if action == UiAction::ChooseDestination {
            let picker = cx.prompt_for_paths(PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some("Select acquisition destination".into()),
            });
            cx.spawn(async move |view, cx| {
                let result = picker.await;
                let _ = view.update(cx, |view, cx| {
                    match result {
                        Ok(Ok(Some(paths))) => {
                            if let Some(path) = paths.first() {
                                if view
                                    .actions
                                    .send(UiAction::SetDestination(
                                        path.to_string_lossy().into_owned(),
                                    ))
                                    .is_err()
                                {
                                    view.error = "Application worker unavailable".into();
                                }
                            }
                        }
                        Ok(Ok(None)) => {}
                        _ => view.error = "Unable to open the folder picker".into(),
                    }
                    cx.notify();
                });
            })
            .detach();
            return;
        }
        if self.actions.send(action).is_err() {
            self.error = "Application worker unavailable".into();
        } else {
            self.error.clear();
        }
        cx.notify();
    }

    /// Outlined pill button: beige text on navy, 13 px corners.
    fn button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<String>,
        action: UiAction,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id.into())
            .px_2()
            .py_1()
            .rounded(rad())
            .border_1()
            .border_color(rgb(EDGE))
            .bg(rgb(PANEL))
            .text_color(rgb(if enabled { TEXT } else { MUTED }))
            .when(enabled, |d| {
                d.cursor_pointer().hover(|d| d.bg(rgb(PANEL_ACTIVE)))
            })
            .child(label.into())
            .on_click(cx.listener(move |view, _, _, cx| {
                if enabled {
                    view.send(action.clone(), cx);
                }
            }))
    }

    /// Solid beige primary button (Start sequence), 13 px corners.
    fn primary_button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<String>,
        action: UiAction,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id.into())
            .px_2()
            .py_2()
            .rounded(rad())
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .bg(rgb(if enabled { BEIGE } else { TOGGLE_OFF }))
            .text_color(rgb(if enabled { INK } else { MUTED }))
            .when(enabled, |d| d.cursor_pointer())
            .child(label.into())
            .on_click(cx.listener(move |view, _, _, cx| {
                if enabled {
                    view.send(action.clone(), cx);
                }
            }))
    }

    /// iOS-style toggle: blue track when on, slate when off, beige knob.
    fn toggle(
        &self,
        id: impl Into<SharedString>,
        on: bool,
        action: UiAction,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let id = id.into();
        // Track 40x21, knob 15: slides 3 <-> 22 px with a 150 ms ease.
        // The animation id embeds the state so every flip replays from rest.
        let knob = div()
            .size(px(15.))
            .rounded(px(999.))
            .bg(rgb(TEXT))
            .absolute()
            .top(px(3.))
            .with_animation(
                SharedString::from(format!("{id}-knob-{on}")),
                Animation::new(Duration::from_millis(150)).with_easing(ease_in_out),
                move |el, delta| {
                    let from = if on { 3.0 } else { 22.0 };
                    let to = if on { 22.0 } else { 3.0 };
                    el.left(px(from + (to - from) * delta))
                },
            );
        div()
            .id(id)
            .w(px(40.))
            .h(px(21.))
            .relative()
            .rounded(px(999.))
            .bg(rgb(if on { TOGGLE_ON } else { TOGGLE_OFF }))
            .when(enabled, |d| d.cursor_pointer())
            .child(knob)
            .on_click(cx.listener(move |view, _, _, cx| {
                if enabled {
                    view.send(action.clone(), cx);
                }
            }))
    }

    fn row(label: impl Into<String>, value: impl Into<String>) -> Div {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_2()
            .child(div().text_color(rgb(MUTED)).child(label.into()))
            .child(div().text_color(rgb(TEXT)).child(value.into()))
    }

    // ------------------------------------------------------------------
    // Left sidebar cards.
    // ------------------------------------------------------------------
    fn device_card(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let s = &self.state;
        let connected = s.connected;
        let action = if connected {
            UiAction::Disconnect
        } else {
            UiAction::Connect
        };
        let source = s.source;
        let source_button = |id: &'static str, label: &'static str, kind: SourceKind| {
            let active = source == kind;
            div()
                .id(id)
                .flex_1()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .px_2()
                .py_1()
                .rounded(px(8.))
                .cursor_pointer()
                .bg(rgb(if active { PANEL_ACTIVE } else { PANEL }))
                .text_color(rgb(if active { TEXT } else { MUTED }))
                .border_1()
                .border_color(rgb(EDGE))
                .child(label)
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.send(UiAction::SetSource(kind), cx);
                }))
        };
        div()
            .id("device-card")
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .bg(rgb(PANEL))
            .border_1()
            .border_color(rgb(EDGE))
            .rounded(rad())
            .child(
                div()
                    .id("device-connect")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .child(div().text_lg().text_color(rgb(BLUE)).child("✆"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(div().text_color(rgb(TEXT)).child(s.device.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(if connected { GREEN } else { MUTED }))
                                    .child(if connected { "● Connected" } else { "○ Offline" }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child(format!("USB · {:.0} MB/s", s.rx_mbps.max(0.0))),
                            ),
                    )
                    .child(div().text_color(rgb(MUTED)).child("›"))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.send(action.clone(), cx);
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("Src"),
                    )
                    .child(source_button("src-phone", "Phone", SourceKind::Phone))
                    .child(source_button("src-sim", "Sim", SourceKind::Simulator)),
            )
    }

    fn camera_card(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let s = &self.state;
        let title = if s.camera_id.is_empty() {
            "No camera".to_string()
        } else {
            s.cameras
                .iter()
                .find(|c| c.id == s.camera_id)
                .map(|c| c.label.clone())
                .unwrap_or_else(|| s.camera_id.clone())
        };
        let subtitle = if s.resolution == (0, 0) {
            "—".to_string()
        } else {
            format!(
                "{} × {} ({})",
                s.resolution.0,
                s.resolution.1,
                if s.raw_enabled { "RAW" } else { "preview" }
            )
        };
        div()
            .id("camera-card")
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .p_2()
            .bg(rgb(PANEL))
            .border_1()
            .border_color(rgb(EDGE))
            .rounded(rad())
            .cursor_pointer()
            .child(div().text_lg().text_color(rgb(TEXT)).child("◉"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(div().text_color(rgb(TEXT)).child(title))
                    .child(div().text_xs().text_color(rgb(MUTED)).child(subtitle)),
            )
            .child(div().text_color(rgb(MUTED)).child("∨"))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.page = Page::Camera;
                cx.notify();
            }))
    }

    fn nav_card(&self, cx: &mut Context<Self>) -> Div {
        let mut nav = panel().p_2().gap_1();
        for (page, icon) in [
            (Page::Dashboard, "⌂"),
            (Page::Preview, "▣"),
            (Page::Camera, "◎"),
            (Page::Sequence, "☰"),
            (Page::Calibration, "⌖"),
            (Page::Sessions, "▤"),
            (Page::Diagnostics, "∿"),
            (Page::Settings, "⚙"),
        ] {
            let active = self.page == page;
            let title = page.title();
            nav = nav.child(
                div()
                    .id(title)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded(rad())
                    .cursor_pointer()
                    .bg(rgb(if active { PANEL_ACTIVE } else { PANEL }))
                    .text_color(rgb(if active { TEXT } else { MUTED }))
                    .hover(|d| d.bg(rgb(PANEL_ACTIVE)))
                    .child(div().w(px(22.)).child(icon))
                    .child(title)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.page = page;
                        cx.notify();
                    })),
            );
        }
        nav.child(
            div()
                .id("nav-quit")
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .px_2()
                .py_1()
                .rounded(rad())
                .cursor_pointer()
                .bg(rgb(PANEL))
                .text_color(rgb(MUTED))
                .hover(|d| d.bg(rgb(0xc42b1c)).text_color(rgb(0xffffff)))
                .child(div().w(px(22.)).child("×"))
                .child("Quit")
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.send(UiAction::Shutdown, cx);
                    window.remove_window();
                })),
        )
    }

    fn session_card(&self) -> Div {
        let s = &self.state;
        let total = s.frames_total.max(1);
        let pct = (s.frames_done as f32 / total as f32 * 100.0).clamp(0.0, 100.0);
        let project = s
            .sessions
            .first()
            .map(|session| session.name.clone())
            .unwrap_or_else(|| "No active session".to_string());
        let session_id = s
            .sessions
            .first()
            .map(|session| format!("Session {}", session.id))
            .unwrap_or_else(|| s.destination.clone());
        panel()
            .p_2()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("⌖")
                            .child("CURRENT SESSION"),
                    )
                    .child(div().text_color(rgb(MUTED)).child("›")),
            )
            .child(div().text_color(rgb(TEXT)).child(project))
            .child(div().text_xs().text_color(rgb(MUTED)).child(session_id))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .child(div().text_color(rgb(TEXT)).child(format!(
                        "{:03} / {} frames",
                        s.frames_done, s.frames_total
                    )))
                    .child(div().text_color(rgb(MUTED)).child(format!("{pct:.1}%"))),
            )
            .child(
                div()
                    .h(px(4.))
                    .w_full()
                    .rounded(px(999.))
                    .bg(rgb(PANEL_ACTIVE))
                    .child(div().h(px(4.)).rounded(px(999.)).bg(rgb(BLUE)).w(relative(
                        (s.frames_done as f32 / total as f32).clamp(0.0, 1.0),
                    ))),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(rgb(EDGE))
                            .text_color(rgb(MUTED))
                            .child(format!("◷ {:.3} s", s.exposure_ns as f64 / 1e9)),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(rgb(EDGE))
                            .text_color(rgb(MUTED))
                            .child(format!("ISO {}", s.iso)),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(rgb(EDGE))
                            .text_color(rgb(MUTED))
                            .child(if s.raw_enabled { "RAW" } else { "—" }),
                    ),
            )
    }

    fn system_card(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let s = &self.state;
        let status_row = |color: u32, label: String| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .text_xs()
                .child(dot(color))
                .child(div().text_color(rgb(TEXT)).child(label))
        };
        div()
            .id("system-card")
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .bg(rgb(PANEL))
            .border_1()
            .border_color(rgb(EDGE))
            .rounded(rad())
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("SYSTEM STATUS")
                    .child("›"),
            )
            .child(status_row(
                if s.connected { GREEN } else { MUTED },
                format!("Camera {}", if s.connected { "Ready" } else { "Offline" }),
            ))
            .child(status_row(
                if s.connected { GREEN } else { MUTED },
                format!("Transport {}", if s.connected { "OK" } else { "Down" }),
            ))
            .child(status_row(GREEN, format!("Storage {}", s.storage_free.clone())))
            .child(status_row(BLUE, format!("Thermal {}", s.thermal.clone())))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.page = Page::Diagnostics;
                cx.notify();
            }))
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("sidebar")
            .w(px(180.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_2()
            .min_h_0()
            .overflow_y_scroll()
            .child(self.device_card(cx))
            .child(self.camera_card(cx))
            .child(self.nav_card(cx))
            .child(self.session_card())
            .child(self.system_card(cx))
    }

    // ------------------------------------------------------------------
    // Center preview panel with overlays.
    // ------------------------------------------------------------------
    fn pill(text: impl Into<String>) -> Div {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(px(999.))
            .bg(rgb(OVERLAY))
            .border_1()
            .border_color(rgb(EDGE))
            .text_xs()
            .text_color(rgb(TEXT))
            .child(text.into())
    }

    fn preview_panel(&self, cx: &mut Context<Self>) -> Div {
        let s = &self.state;
        let backdrop: AnyElement = if let Some(image) = &self.image {
            img(image.clone())
                .size_full()
                .object_fit(if self.preview_cover {
                    ObjectFit::Cover
                } else {
                    ObjectFit::Contain
                })
                .into_any_element()
        } else {
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .bg(rgb(0x02060d))
                .text_color(rgb(MUTED))
                .child(div().text_lg().child("✦"))
                .child("Waiting for preview")
                .child(
                    div()
                        .text_xs()
                        .child("Frames are supplied by the application preview pipeline"),
                )
                .into_any_element()
        };
        // Luminance bars sampled from the (R+G+B concatenated) histogram.
        let mut bars = div().flex().flex_row().items_end().gap(px(1.)).h(px(28.)).w(px(88.));
        if s.histogram.len() >= 768 {
            let bins: Vec<u32> = (0..256).map(|i| s.histogram[i] + s.histogram[256+i] + s.histogram[512+i]).collect();
            let max = bins.iter().copied()
                .max()
                .unwrap_or(1)
                .max(1) as f32;
            for bin in bins.iter().step_by(3) {
                let v = *bin as f32 / max;
                bars = bars.child(div().flex_1().h(px(2.0 + v * 26.0)).bg(rgb(BEIGE)));
            }
        } else {
            bars = bars.child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("no histogram"),
            );
        }
        let alert = if !self.error.is_empty() {
            Some((self.error.clone(), DANGER))
        } else if !s.message.is_empty() {
            Some((s.message.clone(), MUTED))
        } else {
            None
        };
        let caption_title = s
            .sessions
            .first()
            .map(|session| session.name.clone())
            .unwrap_or_else(|| "No target".to_string());
        let caption_sub = if s.resolution == (0, 0) {
            "—".to_string()
        } else {
            format!(
                "{} × {} · {}",
                s.resolution.0,
                s.resolution.1,
                if s.raw_enabled { "RAW" } else { "preview" }
            )
        };
        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .rounded(rad())
                    .border_1()
                    .border_color(rgb(EDGE))
                    .bg(rgb(0x02060d))
                    .overflow_hidden()
                    .child(backdrop)
                    .child(
                        div()
                            .absolute()
                            .top(px(12.))
                            .left(px(12.))
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(Self::pill(if matches!(s.sequence, SequenceStatus::Running | SequenceStatus::Paused) {
                                "RAW acquisition · previous preview".to_string()
                            } else if s.preview.is_some() {
                                "● Live preview".to_string()
                            } else {
                                "○ Preview idle".to_string()
                            }))
                            .child(Self::pill(format!(
                                "{}  {} × {}",
                                if s.raw_enabled { "RAW" } else { "PREVIEW" },
                                s.resolution.0,
                                s.resolution.1
                            ))),
                    )
                    .child(match alert {
                        Some((text, color)) => div()
                            .absolute()
                            .top(px(12.))
                            .left(px(0.))
                            .right(px(0.))
                            .flex()
                            .flex_row()
                            .justify_center()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(999.))
                                    .bg(rgb(OVERLAY))
                                    .border_1()
                                    .border_color(rgb(EDGE))
                                    .text_xs()
                                    .text_color(rgb(color))
                                    .child(text),
                            )
                            .into_any_element(),
                        None => div().into_any_element(),
                    })
                    .child(
                        div()
                            .absolute()
                            .top(px(12.))
                            .right(px(12.))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_2()
                            .rounded(rad())
                            .bg(rgb(OVERLAY))
                            .border_1()
                            .border_color(rgb(EDGE))
                            .child(bars)
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .text_xs()
                                    .text_color(rgb(TEXT))
                                    .child(format!("ISO {}", s.iso))
                                    .child(fmt_exposure(s.exposure_ns)),
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom(px(12.))
                            .left(px(12.))
                            .flex()
                            .flex_col()
                            .px_2()
                            .py_1()
                            .rounded(rad())
                            .bg(rgb(OVERLAY))
                            .border_1()
                            .border_color(rgb(EDGE))
                            .child(div().text_color(rgb(TEXT)).child(caption_title))
                            .child(div().text_xs().text_color(rgb(MUTED)).child(caption_sub)),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom(px(12.))
                            .right(px(12.))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .rounded(rad())
                            .bg(rgb(OVERLAY))
                            .border_1()
                            .border_color(rgb(EDGE))
                            .text_color(rgb(TEXT))
                            .child(self.icon_button("preview-grid", "▦", Page::Preview, cx))
                            .child(self.icon_button_lock(cx))
                            .child(self.scale_button(cx))
                            .child(div().text_xs().child(if self.preview_cover { "Fill" } else { "Fit" })),
                    ),
            )
    }

    fn icon_button(
        &self,
        id: impl Into<SharedString>,
        glyph: impl Into<String>,
        page: Page,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id.into())
            .px_2()
            .py_1()
            .rounded(px(8.))
            .cursor_pointer()
            .hover(|d| d.bg(rgb(PANEL_ACTIVE)))
            .child(glyph.into())
            .on_click(cx.listener(move |view, _, _, cx| {
                view.page = page;
                cx.notify();
            }))
    }

    fn icon_button_lock(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let locked = self.state.locked;
        let action = UiAction::SetLocked(!locked);
        div()
            .id("preview-lock")
            .px_2()
            .py_1()
            .rounded(px(8.))
            .cursor_pointer()
            .text_color(rgb(if locked { BEIGE } else { MUTED }))
            .hover(|d| d.bg(rgb(PANEL_ACTIVE)))
            .child(if locked { "◉" } else { "◎" })
            .on_click(cx.listener(move |view, _, _, cx| {
                view.send(action.clone(), cx);
            }))
    }

    fn scale_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("preview-fit")
            .px_2()
            .py_1()
            .rounded(px(8.))
            .cursor_pointer()
            .hover(|d| d.bg(rgb(PANEL_ACTIVE)))
            .child(if self.preview_cover { "⛶" } else { "⊡" })
            .on_click(cx.listener(move |view, _, _, cx| {
                view.preview_cover = !view.preview_cover;
                cx.notify();
            }))
    }

    // ------------------------------------------------------------------
    // Right acquisition panel.
    // ------------------------------------------------------------------
    fn acquisition_panel(&self, cx: &mut Context<Self>) -> Div {
        let s = &self.state;
        let active = matches!(s.sequence, SequenceStatus::Running | SequenceStatus::Paused);
        let mut panel = panel().p_2().gap_2().child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .text_lg()
                .text_color(rgb(TEXT))
                .child("◎")
                .child("Camera"),
        );
        for block in self.camera_control_blocks(cx) {
            panel = panel.child(block);
        }
        panel
            .child(divider())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .text_lg()
                    .text_color(rgb(TEXT))
                    .child("◉")
                    .child("Acquisition"),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(22.)).text_color(rgb(MUTED)).child("▦"))
                    .child(
                        div()
                            .flex_1()
                            .text_color(rgb(MUTED))
                            .child(format!("Frames ({})", s.frames_total)),
                    ),
            )
            .child(
                slider(
                    "frames-slider",
                    SliderId::Frames,
                    Some((1.0, 1000.0)),
                    f64::from(s.frames_total),
                    false,
                    !active,
                    cx,
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(22.)).text_color(rgb(MUTED)).child("◷"))
                    .child(
                        div()
                            .flex_1()
                            .text_color(rgb(MUTED))
                            .child(format!("Exposure ({})", fmt_exposure(s.exposure_ns))),
                    ),
            )
            .child(
                slider(
                    "acq-exposure-slider",
                    SliderId::Exposure,
                    s.exposure_range_ns.map(|(lo, hi)| (lo as f64, hi as f64)),
                    s.exposure_ns as f64,
                    true,
                    !active,
                    cx,
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(22.)).text_color(rgb(MUTED)).child("∑"))
                    .child(
                        div()
                            .flex_1()
                            .text_color(rgb(MUTED))
                            .child("Total integration"),
                    )
                    .child(
                        div()
                            .text_color(rgb(TEXT))
                            .child(fmt_integration(s.frames_total, s.exposure_ns)),
                    ),
            )
            .child(if active {
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(
                        self.primary_button(
                            "pause-resume",
                            if s.sequence == SequenceStatus::Paused {
                                "▶  Resume"
                            } else {
                                "❚❚  Pause"
                            },
                            if s.sequence == SequenceStatus::Paused {
                                UiAction::ResumeSequence
                            } else {
                                UiAction::PauseSequence
                            },
                            true,
                            cx,
                        ),
                    )
                    .child(self.button("stop", "■ Stop", UiAction::StopSequence, true, cx))
                    .into_any_element()
            } else {
                self.primary_button(
                    "start",
                    "▶  Start sequence",
                    UiAction::StartSequence,
                    s.connected && s.frames_total > 0,
                    cx,
                )
                .into_any_element()
            })
            .child(divider())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .text_color(rgb(TEXT))
                    .child("⚙")
                    .child("Capture options"),
            )
            .child(self.toggle_row("↓", "Save RAW/DNG", s.raw_enabled, UiAction::SetRaw(!s.raw_enabled), s.raw_supported && !active, cx))
            .child(self.toggle_row("↻", "Auto save", s.auto_save, UiAction::SetAutoSave(!s.auto_save), !active, cx))
            .child(self.toggle_row("◉", "Lock focus", s.locked, UiAction::SetLocked(!s.locked), s.connected, cx))
            .child(divider())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .text_color(rgb(TEXT))
                    .child("▣")
                    .child("Sequence mode"),
            )
            .child(self.button(
                "frame-type",
                format!("{:?}s ∨", s.frame_type),
                UiAction::SetFrameType(match s.frame_type {
                    FrameType::Light => FrameType::Dark,
                    FrameType::Dark => FrameType::Flat,
                    FrameType::Flat => FrameType::Bias,
                    FrameType::Bias => FrameType::Light,
                }),
                !active,
                cx,
            ))
    }

    fn toggle_row(
        &self,
        icon: impl Into<String>,
        label: impl Into<String>,
        on: bool,
        action: UiAction,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let label: String = label.into();
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(div().w(px(22.)).text_color(rgb(MUTED)).child(icon.into()))
            .child(div().flex_1().text_color(rgb(TEXT)).child(label.clone()))
            .child(self.toggle(format!("toggle-{label}"), on, action, enabled, cx))
    }

    // ------------------------------------------------------------------
    // Bottom bar + pages.
    // ------------------------------------------------------------------
    fn bottom_bar(&self, cx: &mut Context<Self>) -> Div {
        let s = &self.state;
        let status = |icon: &str, top: String, bottom: String| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .flex_shrink_0()
                .child(div().text_lg().text_color(rgb(MUTED)).child(icon.to_string()))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().text_xs().text_color(rgb(MUTED)).child(top))
                        .child(div().text_xs().text_color(rgb(TEXT)).child(bottom)),
                )
        };
        let tab = |id: &'static str, icon: &'static str, label: &'static str, page: Page, active: bool| {
            div()
                .id(id)
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .flex_shrink_0()
                .px_2()
                .py_1()
                .rounded(rad())
                .cursor_pointer()
                .bg(rgb(if active { BEIGE } else { PANEL }))
                .text_color(rgb(if active { INK } else { TEXT }))
                .border_1()
                .border_color(rgb(EDGE))
                .child(icon.to_string())
                .child(label.to_string())
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.page = page;
                    cx.notify();
                }))
        };
        panel()
            .flex_shrink_0()
            .p_2()
            .child(
                div()
                    .id("bottom-scroll")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_4()
                    .overflow_x_scroll()
                    .child(status("▤", "Storage".into(), s.storage_free.clone()))
                    .child(divider_v())
                    .child(status(
                        "≣",
                        "Metadata".into(),
                        if s.raw_enabled {
                            "DNG + sidecar".into()
                        } else {
                            "Preview only".into()
                        },
                    ))
                    .child(divider_v())
                    .child(status("∿", "Diagnostics".into(), s.thermal.clone()))
                    .child(divider_v())
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .flex_shrink_0()
                            .child(div().text_xs().text_color(rgb(MUTED)).child("Frame"))
                            .child(
                                div()
                                    .w(px(120.))
                                    .h(px(6.))
                                    .rounded(px(999.))
                                    .bg(rgb(PANEL_ACTIVE))
                                    .child(div().h(px(6.)).rounded(px(999.)).bg(rgb(BLUE)).w(relative(
                                        s.frame_progress.unwrap_or(0.0).clamp(0.0, 1.0),
                                    ))),
                            )
                            .child(
                                div().text_sm().text_color(rgb(TEXT)).child(match s.frame_progress {
                                    Some(p) => {
                                        let total_s = s.frame_exposure_s.unwrap_or(s.exposure_ns as f32 / 1e9);
                                        let elapsed = s.frame_elapsed_s.unwrap_or(p * total_s);
                                        format!("{elapsed:.2} / {total_s:.3} s{}", if elapsed > total_s { " · readout / transfer" } else { " · elapsed (host)" })
                                    }
                                    None => "—".into(),
                                }),
                            ),
                    )
                    .child(divider_v())
                    .child(div().flex_1().min_w(px(8.)))
                    .child(tab("tab-preview", "◉", "Preview", Page::Preview, self.page == Page::Preview))
                    .child(tab("tab-acquire", "◎", "Acquisition", Page::Camera, self.page == Page::Camera))
                    .child(tab("tab-sequence", "☰", "Sequence", Page::Sequence, self.page == Page::Sequence))
                    .child(tab("tab-storage", "▤", "Storage", Page::Sessions, self.page == Page::Sessions))
                    .child(tab("tab-settings", "⚙", "Settings", Page::Settings, self.page == Page::Settings))
                    .child(divider_v())
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(rgb(if self.error.is_empty() { MUTED } else { DANGER }))
                            .child(if self.error.is_empty() {
                                s.message.clone()
                            } else {
                                self.error.clone()
                            }),
                    ),
            )
    }

    fn dashboard(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .flex_row()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(self.sidebar(cx))
            .child(self.preview_panel(cx))
            .child(
                div()
                    .id("acquire-col")
                    .w(px(224.))
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(self.acquisition_panel(cx)),
            )
    }


    /// Every camera control as elements, shared by the Camera page and the
    /// right side panel. Button ids match in both trees on purpose: only one
    /// tree renders at a time, so ids never collide.
    fn camera_control_blocks(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let s = &self.state;
        let edit = s.connected
            && !matches!(s.sequence, SequenceStatus::Running | SequenceStatus::Paused);
        let mut items: Vec<AnyElement> = Vec::new();
        for camera in &s.cameras {
            items.push(
                self.button(
                    format!("camera-{}", camera.id),
                    format!(
                        "{} {}",
                        if s.camera_id == camera.id { "●" } else { "○" },
                        camera.label
                    ),
                    UiAction::SelectCamera(camera.id.clone()),
                    edit,
                    cx,
                )
                .into_any_element(),
            );
        }
        for &(w, h) in &s.resolutions {
            items.push(
                self.button(
                    format!("res-{w}-{h}"),
                    format!("{w} × {h}"),
                    UiAction::SetResolution(w, h),
                    edit,
                    cx,
                )
                .into_any_element(),
            );
        }
        items.push(Self::row("Sensor", s.sensor.clone()).into_any_element());
        items.push(Self::row("Hardware", s.hardware_level.clone()).into_any_element());
        items.push(Self::row("Exposure", fmt_exposure(s.exposure_ns)).into_any_element());
        items.push(
            slider(
                "exp-slider",
                SliderId::Exposure,
                s.exposure_range_ns.map(|(lo, hi)| (lo as f64, hi as f64)),
                s.exposure_ns as f64,
                true,
                edit,
                cx,
            )
            .into_any_element(),
        );
        items.push(Self::row("Sensitivity", format!("ISO {}", s.iso)).into_any_element());
        items.push(
            slider(
                "iso-slider",
                SliderId::Iso,
                s.iso_range.map(|(lo, hi)| (lo as f64, hi as f64)),
                f64::from(s.iso),
                true,
                edit,
                cx,
            )
            .into_any_element(),
        );
        items.push(Self::row("Focus", format!("{:.2} D", s.focus_diopters)).into_any_element());
        items.push(self.button("center-af", "Autofocus centro + blocco", UiAction::AutofocusCenter,
            edit && s.focus_range.is_some(), cx).into_any_element());
        items.push(
            slider(
                "focus-slider",
                SliderId::Focus,
                s.focus_range.map(|(lo, hi)| (f64::from(lo), f64::from(hi))),
                f64::from(s.focus_diopters),
                false,
                edit,
                cx,
            )
            .into_any_element(),
        );
        items.push(
            Self::row(
                "White balance",
                if s.wb_preset.is_empty() { "auto".into() } else { s.wb_preset.clone() },
            )
            .into_any_element(),
        );
        items.push(
            self.button(
                "wb-cycle",
                format!(
                    "{} ∨",
                    if s.wb_preset.is_empty() { "auto" } else { s.wb_preset.as_str() }
                ),
                UiAction::SetWbPreset(next_wb_preset(&s.wb_presets, &s.wb_preset)),
                edit && !s.wb_presets.is_empty(),
                cx,
            )
            .into_any_element(),
        );
        items.push(Self::row("Zoom", format!("{:.1}×", s.zoom)).into_any_element());
        items.push(
            slider(
                "zoom-slider",
                SliderId::Zoom,
                s.zoom_range.map(|(lo, hi)| (f64::from(lo), f64::from(hi))),
                f64::from(s.zoom),
                true,
                edit,
                cx,
            )
            .into_any_element(),
        );
        items.push(
            self.button(
                "lock",
                if s.locked { "Unlock focus" } else { "Lock focus" },
                UiAction::SetLocked(!s.locked),
                s.connected,
                cx,
            )
            .into_any_element(),
        );
        items
    }

    fn camera_page(&self, cx: &mut Context<Self>) -> Div {
        let mut content = panel().p_2().gap_2().child(
            div()
                .text_lg()
                .text_color(rgb(TEXT))
                .child("Camera controls"),
        );
        for item in self.camera_control_blocks(cx) {
            content = content.child(item);
        }
        div()
            .flex()
            .flex_row()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(self.sidebar(cx))
            .child(scroll_col("page-camera").child(content))
    }

    fn sequence_page(&self, cx: &mut Context<Self>) -> Div {
        let s = &self.state;
        let active = matches!(s.sequence, SequenceStatus::Running | SequenceStatus::Paused);
        div()
            .flex()
            .flex_row()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(self.sidebar(cx))
            .child(
                scroll_col("page-sequence")
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        panel().p_2().gap_2()
                            .child(div().text_lg().text_color(rgb(TEXT)).child("Sequence"))
                            .child(Self::row("Frames", format!("{:03} / {}", s.frames_done, s.frames_total)))
                            .child(Self::row("Integration", fmt_integration(s.frames_total, s.exposure_ns)))
                            .child(Self::row("State", format!("{:?}", s.sequence)))
                            .child(
                                slider(
                                    "seq-frames-slider",
                                    SliderId::Frames,
                                    Some((1.0, 1000.0)),
                                    f64::from(s.frames_total),
                                    false,
                                    !active,
                                    cx,
                                ),
                            )
                            .child(if active {
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(self.button(
                                        "pause2",
                                        if s.sequence == SequenceStatus::Paused { "Resume" } else { "Pause" },
                                        if s.sequence == SequenceStatus::Paused {
                                            UiAction::ResumeSequence
                                        } else {
                                            UiAction::PauseSequence
                                        },
                                        true,
                                        cx,
                                    ))
                                    .child(self.button("stop2", "Stop", UiAction::StopSequence, true, cx))
                                    .into_any_element()
                            } else {
                                self.primary_button(
                                    "start2",
                                    "▶  Start sequence",
                                    UiAction::StartSequence,
                                    s.connected && s.frames_total > 0,
                                    cx,
                                )
                                .into_any_element()
                            })
                            .child(Self::row("Frame type", format!("{:?}", s.frame_type)))
                            .child(self.button(
                                "capture-one",
                                "Capture one RAW",
                                UiAction::CaptureOne,
                                s.connected && s.raw_supported && !active,
                                cx,
                            )),
                    )
                    .child(
                        panel().p_2().gap_2()
                            .child(div().text_lg().text_color(rgb(TEXT)).child("Output"))
                            .child(Self::row("Destination", s.destination.clone()))
                            .child(self.button(
                                "choose-seq",
                                "Choose destination",
                                UiAction::ChooseDestination,
                                true,
                                cx,
                            )),
                    ),
            )
    }

    fn diagnostics_page(&self, cx: &mut Context<Self>) -> Div {
        let s = &self.state;
        let mut content = panel()
            .p_2()
            .gap_2()
            .child(div().text_lg().text_color(rgb(TEXT)).child("Diagnostics · raw values"))
            .child(Self::row("Requested exposure (ns)", s.exposure_ns.to_string()))
            .child(Self::row("Applied exposure (ns)", format!("{:?}", s.applied_exposure_ns)))
            .child(Self::row("Requested ISO", s.iso.to_string()))
            .child(Self::row("Applied ISO", format!("{:?}", s.applied_iso)))
            .child(Self::row("RAW supported", s.raw_supported.to_string()))
            .child(Self::row("RX / TX (MB/s)", format!("{:.3} / {:.3}", s.rx_mbps, s.tx_mbps)))
            .child(Self::row("Dropped RAW / preview", format!("{} / {}", s.dropped_raw, s.dropped_preview)));
        for (key, value) in &s.diagnostics {
            content = content.child(Self::row(key.clone(), value.clone()));
        }
        content = content.child(self.button(
            "refresh-diag",
            "Refresh diagnostics",
            UiAction::RefreshDiagnostics,
            true,
            cx,
        ));
        div()
            .flex()
            .flex_row()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(self.sidebar(cx))
            .child(scroll_col("page-diagnostics").child(content))
    }

    fn simple_page(&self, cx: &mut Context<Self>, title: &str, body: Vec<AnyElement>) -> Div {
        let mut content = panel().p_2().gap_2().child(
            div()
                .text_lg()
                .text_color(rgb(TEXT))
                .child(title.to_string()),
        );
        for element in body {
            content = content.child(element);
        }
        div()
            .flex()
            .flex_row()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(self.sidebar(cx))
            .child(scroll_col("page-simple").child(content))
    }
}

fn divider_v() -> Div {
    div().w(px(1.)).h(px(24.)).bg(rgb(EDGE)).flex_shrink_0()
}

/// Scrollable content column with a stable id (GPUI scroll needs state).
fn scroll_col(id: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id.into())
        .flex_1()
        .min_w_0()
        .min_h_0()
        .overflow_y_scroll()
}

impl Render for Desktop {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match self.page {
            Page::Dashboard | Page::Preview => self.dashboard(cx).into_any_element(),
            Page::Camera => self.camera_page(cx).into_any_element(),
            Page::Sequence => self.sequence_page(cx).into_any_element(),
            Page::Diagnostics => self.diagnostics_page(cx).into_any_element(),
            Page::Calibration => {
                let enabled = !matches!(
                    self.state.sequence,
                    SequenceStatus::Running | SequenceStatus::Paused
                );
                let mut kinds: Vec<AnyElement> = Vec::new();
                for kind in [FrameType::Light, FrameType::Dark, FrameType::Flat, FrameType::Bias] {
                    kinds.push(
                        self.button(
                            format!("kind-{kind:?}"),
                            format!(
                                "{} {:?}",
                                if self.state.frame_type == kind { "●" } else { "○" },
                                kind
                            ),
                            UiAction::SetFrameType(kind),
                            enabled,
                            cx,
                        )
                        .into_any_element(),
                    );
                }
                self.simple_page(
                    cx,
                    "Calibration frames",
                    vec![
                        div()
                            .text_color(rgb(MUTED))
                            .child("Cover the lens for dark/bias frames. Use an evenly illuminated field for flats.")
                            .into_any_element(),
                    ]
                    .into_iter()
                    .chain(kinds)
                    .collect(),
                )
                .into_any_element()
            }
            Page::Sessions => {
                let mut items: Vec<AnyElement> = vec![self
                    .button(
                        "refresh-sessions",
                        "Refresh sessions",
                        UiAction::RefreshSessions,
                        true,
                        cx,
                    )
                    .into_any_element()];
                if self.state.sessions.is_empty() {
                    items.push(
                        div()
                            .text_color(rgb(MUTED))
                            .child("No sessions reported by the runtime")
                            .into_any_element(),
                    );
                }
                for session in &self.state.sessions {
                    items.push(
                        self.button(
                            format!("session-{}", session.id),
                            format!("{} · {} frames · {}", session.name, session.frames, session.status),
                            UiAction::OpenSession(session.id.clone()),
                            true,
                            cx,
                        )
                        .into_any_element(),
                    );
                }
                self.simple_page(cx, "Sessions", items).into_any_element()
            }
            Page::Settings => {
                let settings_body = vec![
                    Self::row("Destination", self.state.destination.clone()).into_any_element(),
                    self.button(
                        "destination",
                        "Choose destination",
                        UiAction::ChooseDestination,
                        true,
                        cx,
                    )
                    .into_any_element(),
                    self.toggle_row(
                        "↻",
                        "Auto save",
                        self.state.auto_save,
                        UiAction::SetAutoSave(!self.state.auto_save),
                        true,
                        cx,
                    )
                    .into_any_element(),
                    Self::row(
                        "About",
                        format!("DeepskyEyes {} · GPUI desktop", env!("CARGO_PKG_VERSION")),
                    )
                    .into_any_element(),
                ];
                self.simple_page(cx, "Settings", settings_body)
                    .into_any_element()
            }
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .text_xs()
            .font_family("Segoe UI")
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_1()
                            .min_h_0()
                            .gap_2()
                            .child(content),
                    )
                    .child(self.bottom_bar(cx)),
            )
    }
}
