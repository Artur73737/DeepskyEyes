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
        let bounds = Bounds::centered(None, size(px(1440.), px(920.)), cx);
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
                window_min_size: Some(size(px(1050.), px(700.))),
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
        cx.activate(true);
    });
}

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
            Self::Camera => "Camera",
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
    image: Option<Arc<Image>>,
    image_revision: Option<u64>,
    preview_scale: f32,
    error: String,
    _poll: Task<()>,
}
const BG: u32 = 0x020f1e;
const PANEL: u32 = 0x061b2d;
const EDGE: u32 = 0x23455f;
const MUTED: u32 = 0x95adc5;
fn card(title: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(EDGE))
        .rounded_lg()
        .child(div().text_color(rgb(MUTED)).child(title.into()))
}
fn row(label: impl Into<String>, value: impl Into<String>) -> Div {
    div()
        .flex()
        .justify_between()
        .gap_3()
        .child(div().text_color(rgb(MUTED)).child(label.into()))
        .child(value.into())
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
                        if let Some(s) = latest {
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
            image: None,
            image_revision: None,
            preview_scale: 1.0,
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
    fn button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<String>,
        action: UiAction,
        enabled: bool,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id.into())
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(EDGE))
            .bg(rgb(if enabled { 0x103253 } else { 0x10202e }))
            .text_color(rgb(if enabled { 0xe9f0f7 } else { 0x536779 }))
            .when(enabled, |d| {
                d.cursor_pointer().hover(|d| d.bg(rgb(0x20496e)))
            })
            .child(label.into())
            .on_click(cx.listener(move |view, _, _, cx| {
                if enabled {
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
                                                view.error =
                                                    "Application worker unavailable".into();
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
                    if view.actions.send(action.clone()).is_err() {
                        view.error = "Application worker unavailable".into();
                    } else {
                        view.error.clear();
                    }
                    cx.notify();
                }
            }))
    }
    fn camera(&self, cx: &Context<Self>) -> Div {
        let s = &self.state;
        let edit = s.connected
            && !s.locked
            && !matches!(s.sequence, SequenceStatus::Running | SequenceStatus::Paused);
        let mut panel = card("CAMERA CONTROLS")
            .child(row("Sensor", s.sensor.clone()))
            .child(row("Hardware", s.hardware_level.clone()));
        for camera in &s.cameras {
            panel = panel.child(self.button(
                format!("camera-{}", camera.id),
                format!(
                    "{} {}",
                    if s.camera_id == camera.id {
                        "●"
                    } else {
                        "○"
                    },
                    camera.label
                ),
                UiAction::SelectCamera(camera.id.clone()),
                edit,
                cx,
            ));
        }
        for &(w, h) in &s.resolutions {
            panel = panel.child(self.button(
                format!("res-{w}-{h}"),
                format!("{w} × {h}"),
                UiAction::SetResolution(w, h),
                edit,
                cx,
            ));
        }
        panel = panel.child(self.button(
            "raw",
            format!("RAW / DNG   {}", s.raw_enabled),
            UiAction::SetRaw(!s.raw_enabled),
            edit && s.raw_supported,
            cx,
        ));
        let exposure = s.exposure_range_ns;
        panel = panel
            .child(row(
                "Exposure",
                format!("{:.3} s", s.exposure_ns as f64 / 1e9),
            ))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.button(
                        "exp-down",
                        "− 1 s",
                        UiAction::SetExposure(exposure.map_or(s.exposure_ns, |(lo, hi)| {
                            s.exposure_ns
                                .saturating_sub(1_000_000_000)
                                .clamp(lo, hi.max(lo))
                        })),
                        edit && exposure.is_some(),
                        cx,
                    ))
                    .child(self.button(
                        "exp-up",
                        "+ 1 s",
                        UiAction::SetExposure(exposure.map_or(s.exposure_ns, |(lo, hi)| {
                            s.exposure_ns
                                .saturating_add(1_000_000_000)
                                .clamp(lo, hi.max(lo))
                        })),
                        edit && exposure.is_some(),
                        cx,
                    )),
            );
        panel = panel
            .child(row("Sensitivity", format!("ISO {}", s.iso)))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.button(
                        "iso-down",
                        "− 100",
                        UiAction::SetIso(s.iso_range.map_or(s.iso, |(lo, hi)| {
                            s.iso.saturating_sub(100).clamp(lo, hi.max(lo))
                        })),
                        edit && s.iso_range.is_some(),
                        cx,
                    ))
                    .child(self.button(
                        "iso-up",
                        "+ 100",
                        UiAction::SetIso(s.iso_range.map_or(s.iso, |(lo, hi)| {
                            s.iso.saturating_add(100).clamp(lo, hi.max(lo))
                        })),
                        edit && s.iso_range.is_some(),
                        cx,
                    )),
            );
        panel = panel
            .child(row("Focus", format!("{:.2} D", s.focus_diopters)))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.button(
                        "focus-down",
                        "− 0.1 D",
                        UiAction::SetFocus(s.focus_range.map_or(s.focus_diopters, |(lo, hi)| {
                            (s.focus_diopters - 0.1).clamp(lo, hi.max(lo))
                        })),
                        edit && s.focus_range.is_some(),
                        cx,
                    ))
                    .child(self.button(
                        "focus-up",
                        "+ 0.1 D",
                        UiAction::SetFocus(s.focus_range.map_or(s.focus_diopters, |(lo, hi)| {
                            (s.focus_diopters + 0.1).clamp(lo, hi.max(lo))
                        })),
                        edit && s.focus_range.is_some(),
                        cx,
                    )),
            );
        panel = panel
            .child(row(
                "White balance",
                format!("{} K", s.white_balance_kelvin),
            ))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.button(
                        "wb-down",
                        "− 250 K",
                        UiAction::SetWhiteBalance(
                            s.white_balance_kelvin.saturating_sub(250).max(1000),
                        ),
                        edit && s.manual_white_balance,
                        cx,
                    ))
                    .child(self.button(
                        "wb-up",
                        "+ 250 K",
                        UiAction::SetWhiteBalance(
                            s.white_balance_kelvin.saturating_add(250).min(15000),
                        ),
                        edit && s.manual_white_balance,
                        cx,
                    )),
            );
        panel
            .child(row("Zoom", format!("{:.1}×", s.zoom)))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        self.button(
                            "zoom-down",
                            "− 0.5×",
                            UiAction::SetZoom(
                                s.zoom_range.map_or(s.zoom, |(lo, hi)| {
                                    (s.zoom - 0.5).clamp(lo, hi.max(lo))
                                }),
                            ),
                            edit && s.zoom_range.is_some(),
                            cx,
                        ),
                    )
                    .child(
                        self.button(
                            "zoom-up",
                            "+ 0.5×",
                            UiAction::SetZoom(
                                s.zoom_range.map_or(s.zoom, |(lo, hi)| {
                                    (s.zoom + 0.5).clamp(lo, hi.max(lo))
                                }),
                            ),
                            edit && s.zoom_range.is_some(),
                            cx,
                        ),
                    ),
            )
            .child(self.button(
                "lock",
                if s.locked {
                    "Unlock controls"
                } else {
                    "Lock controls"
                },
                UiAction::SetLocked(!s.locked),
                s.connected,
                cx,
            ))
    }
    fn sequence(&self, cx: &Context<Self>) -> Div {
        let s = &self.state;
        let active = matches!(s.sequence, SequenceStatus::Running | SequenceStatus::Paused);
        card("ACQUISITION")
            .child(self.button(
                "capture-one",
                "Capture one RAW",
                UiAction::CaptureOne,
                s.connected && s.raw_supported && !active,
                cx,
            ))
            .child(row(
                "Frames",
                format!("{:03} / {}", s.frames_done, s.frames_total),
            ))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.button(
                        "frames-less",
                        "− 10",
                        UiAction::SetFrameCount(s.frames_total.saturating_sub(10).max(1)),
                        !active,
                        cx,
                    ))
                    .child(self.button(
                        "frames-more",
                        "+ 10",
                        UiAction::SetFrameCount(s.frames_total.saturating_add(10)),
                        !active,
                        cx,
                    )),
            )
            .child(row(
                "Integration",
                format!(
                    "{:.1} min",
                    s.frames_total as f64 * s.exposure_ns as f64 / 60e9
                ),
            ))
            .child(row("State", format!("{:?}", s.sequence)))
            .child(div().h_2().w_full().bg(rgb(EDGE)).rounded_md().child(
                div().h_2().rounded_md().bg(rgb(0x398cff)).w(relative(
                    (s.frames_done as f32 / s.frames_total.max(1) as f32).clamp(0., 1.),
                )),
            ))
            .child(
                self.button(
                    "start",
                    "▶  Start sequence",
                    UiAction::StartSequence,
                    s.connected && !active && s.frames_total > 0,
                    cx,
                )
                .when(s.connected && !active, |d| {
                    d.bg(rgb(0xf1dec0)).text_color(rgb(BG))
                }),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.button(
                        "pause",
                        if s.sequence == SequenceStatus::Paused {
                            "Resume"
                        } else {
                            "Pause"
                        },
                        if s.sequence == SequenceStatus::Paused {
                            UiAction::ResumeSequence
                        } else {
                            UiAction::PauseSequence
                        },
                        active,
                        cx,
                    ))
                    .child(self.button("stop", "Stop", UiAction::StopSequence, active, cx)),
            )
            .child(row("Frame type", format!("{:?}", s.frame_type)))
    }
    fn preview(&self, cx: &Context<Self>) -> Div {
        let s = &self.state;
        let image = if let Some(image) = &self.image {
            img(image.clone())
                .w(relative(self.preview_scale))
                .h(relative(self.preview_scale))
                .object_fit(ObjectFit::Contain)
                .into_any_element()
        } else {
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .text_color(rgb(MUTED))
                .child("✦")
                .child("Waiting for preview")
                .child("Frames are supplied by the application preview pipeline")
                .into_any_element()
        };
        card("LIVE PREVIEW")
            .flex_1()
            .min_h(px(320.))
            .child(row(
                format!("{} × {}", s.resolution.0, s.resolution.1),
                if s.raw_enabled { "RAW" } else { "Preview" },
            ))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .children([1.0_f32, 2.0, 4.0].into_iter().map(|scale| {
                        div()
                            .id(SharedString::from(format!("preview-scale-{scale}")))
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x103253))
                            .cursor_pointer()
                            .child(if scale == 1.0 {
                                "Fit".to_string()
                            } else {
                                format!("{scale}×")
                            })
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.preview_scale = scale;
                                cx.notify();
                            }))
                    })),
            )
            .child(
                div()
                    .id("preview-scroll")
                    .flex_1()
                    .min_h_0()
                    .bg(rgb(0x010810))
                    .rounded_md()
                    .overflow_scroll()
                    .child(image),
            )
    }
    fn diagnostics(&self, cx: &Context<Self>) -> Div {
        let s = &self.state;
        let mut d = card("DIAGNOSTICS · RAW VALUES")
            .child(row("Requested exposure (ns)", s.exposure_ns.to_string()))
            .child(row(
                "Applied exposure (ns)",
                format!("{:?}", s.applied_exposure_ns),
            ))
            .child(row("Requested ISO", s.iso.to_string()))
            .child(row("Applied ISO", format!("{:?}", s.applied_iso)))
            .child(row("RAW supported", s.raw_supported.to_string()))
            .child(row(
                "RX / TX (MB/s)",
                format!("{:.3} / {:.3}", s.rx_mbps, s.tx_mbps),
            ))
            .child(row(
                "Dropped RAW / preview",
                format!("{} / {}", s.dropped_raw, s.dropped_preview),
            ));
        for (key, value) in &s.diagnostics {
            d = d.child(row(key.clone(), value.clone()));
        }
        d.child(self.button(
            "refresh-diag",
            "Refresh diagnostics",
            UiAction::RefreshDiagnostics,
            true,
            cx,
        ))
    }
}
impl Render for Desktop {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let s = &self.state;
        let mut nav = card("✦  DeepskyEyes")
            .w(px(220.))
            .flex_shrink_0()
            .child(div().child(s.device.clone()))
            .child(row(
                "Connection",
                if s.connected {
                    "● Connected"
                } else {
                    "Disconnected"
                },
            ))
            .child(self.button(
                "connect",
                if s.connected { "Disconnect" } else { "Connect" },
                if s.connected {
                    UiAction::Disconnect
                } else {
                    UiAction::Connect
                },
                true,
                cx,
            ));
        for page in [
            Page::Dashboard,
            Page::Camera,
            Page::Preview,
            Page::Sequence,
            Page::Calibration,
            Page::Sessions,
            Page::Diagnostics,
            Page::Settings,
        ] {
            nav = nav.child(
                div()
                    .id(page.title())
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(rgb(if self.page == page { 0x103253 } else { PANEL }))
                    .hover(|d| d.bg(rgb(0x183c59)))
                    .child(page.title())
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.page = page;
                        cx.notify();
                    })),
            );
        }
        nav = nav
            .child(row("Storage", s.storage_free.clone()))
            .child(row("Thermal", s.thermal.clone()));
        let content = match self.page {
            Page::Dashboard => div()
                .flex()
                .gap_4()
                .flex_1()
                .min_h_0()
                .child(self.preview(cx))
                .child(
                    div()
                        .id("dashboard-controls")
                        .w(px(310.))
                        .flex_shrink_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(self.sequence(cx))
                        .child(self.camera(cx)),
                ),
            Page::Preview => div().flex().flex_1().min_h_0().child(self.preview(cx)),
            Page::Camera => div().child(self.camera(cx)),
            Page::Sequence => div().child(self.sequence(cx)).child(
                card("OUTPUT")
                    .child(row("Destination", s.destination.clone()))
                    .child(self.button(
                        "choose-seq",
                        "Choose destination",
                        UiAction::ChooseDestination,
                        true,
                        cx,
                    )),
            ),
            Page::Diagnostics => div().child(self.diagnostics(cx)),
            Page::Calibration => {
                let mut d=card("CALIBRATION FRAMES").child("Cover the lens for dark/bias frames. Use an evenly illuminated field for flats.");
                for kind in [
                    FrameType::Light,
                    FrameType::Dark,
                    FrameType::Flat,
                    FrameType::Bias,
                ] {
                    d = d.child(self.button(
                        format!("kind-{kind:?}"),
                        format!("{kind:?}"),
                        UiAction::SetFrameType(kind),
                        !matches!(s.sequence, SequenceStatus::Running | SequenceStatus::Paused),
                        cx,
                    ));
                }
                div().child(d).child(self.sequence(cx))
            }
            Page::Sessions => {
                let mut d = card("SESSIONS").child(self.button(
                    "refresh-sessions",
                    "Refresh sessions",
                    UiAction::RefreshSessions,
                    true,
                    cx,
                ));
                if s.sessions.is_empty() {
                    d = d.child("No sessions reported by the runtime");
                }
                for session in &s.sessions {
                    d = d.child(self.button(
                        format!("session-{}", session.id),
                        format!(
                            "{} · {} frames · {}",
                            session.name, session.frames, session.status
                        ),
                        UiAction::OpenSession(session.id.clone()),
                        true,
                        cx,
                    ));
                }
                div().child(d)
            }
            Page::Settings => div().child(
                card("SETTINGS")
                    .child(row("Destination", s.destination.clone()))
                    .child(self.button(
                        "destination",
                        "Choose destination",
                        UiAction::ChooseDestination,
                        true,
                        cx,
                    ))
                    .child(self.button(
                        "autosave",
                        format!("Auto save: {}", s.auto_save),
                        UiAction::SetAutoSave(!s.auto_save),
                        true,
                        cx,
                    )),
            ),
        };
        let mut histogram = div().flex().items_end().gap_1().h(px(52.));
        let max = s.histogram.iter().copied().max().unwrap_or(1).max(1) as f32;
        for bin in s.histogram.iter().take(256) {
            histogram = histogram.child(
                div()
                    .flex_1()
                    .h(px(*bin as f32 / max * 48.))
                    .bg(rgb(0xe5d2b6)),
            );
        }
        let mut footer = card("HISTOGRAM / STATISTICS").child(histogram);
        if s.histogram.is_empty() {
            footer = footer.child("No histogram available");
        }
        for (key, value) in &s.statistics {
            footer = footer.child(row(key.clone(), value.clone()));
        }
        div()
            .size_full()
            .flex()
            .gap_4()
            .p_4()
            .bg(rgb(BG))
            .text_color(rgb(0xe5edf5))
            .text_sm()
            .font_family("Segoe UI")
            .child(nav)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap_3()
                    .child(div().text_xl().child(self.page.title()))
                    .child(
                        div()
                            .id("page")
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(content),
                    )
                    .when(
                        self.page == Page::Dashboard || self.page == Page::Preview,
                        |d| d.child(footer),
                    )
                    .child(
                        div()
                            .text_color(rgb(if self.error.is_empty() {
                                MUTED
                            } else {
                                0xff9580
                            }))
                            .child(if self.error.is_empty() {
                                s.message.clone()
                            } else {
                                self.error.clone()
                            }),
                    ),
            )
    }
}
