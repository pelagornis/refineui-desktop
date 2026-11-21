//! Backdrop blur benchmark: a busy backdrop with N material panels, measured
//! in three phases:
//!
//! 1. `blur/moving`  — panels move every frame: every blur is recomputed.
//! 2. `blur/static`  — panels still, only a small widget elsewhere animates:
//!    the renderer reuses the cached blurred backdrops (composite only).
//! 3. `fallback`     — reduced transparency: opaque fills, no blur.
//!
//! ```sh
//! cargo run --release -p refineui-material --example blur_bench -- [panels]
//! GPUI_BLUR_CACHE=0 cargo run --release -p refineui-material --example blur_bench -- [panels]
//! ```
//!
//! The second command disables the cache, so `blur/static` shows the cost
//! without reuse. On Linux the numbers include the GPU (or software Vulkan,
//! e.g. lavapipe on a headless X server) because Blade waits for the
//! previous frame. Frame pacing is capped by the refresh timer (60 Hz), so
//! results are quantized to multiples of ~16.7 ms; use enough panels that
//! the uncached case drops below the cap. On a headless Xvfb server frames
//! only arrive while the pointer moves over the window, e.g.
//! `while :; do xdotool mousemove 500 500 mousemove 520 510; done`.

use std::time::{Duration, Instant};

use gpui::{
    App, AppContext, Application, Bounds, Context, FontWeight, IntoElement, ParentElement, Render,
    Styled, Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use refineui_material::prelude::*;

const WARMUP: Duration = Duration::from_millis(600);
const MEASURE: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Moving,
    Static,
    Fallback,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Self::Moving => "blur/moving",
            Self::Static => "blur/static",
            Self::Fallback => "fallback",
        }
    }
}

struct Bench {
    panels: usize,
    phase: Phase,
    started: Instant,
    frames: u32,
    tick: u32,
    results: Vec<String>,
}

impl Bench {
    fn next_phase(&mut self, cx: &mut Context<Self>) {
        let elapsed = self.started.elapsed();
        let secs = (elapsed - WARMUP).as_secs_f64();
        let fps = self.frames as f64 / secs;
        self.results.push(format!(
            "{:<12} panels={:<2} frames={:<5} avg={:>7.2} ms  fps={:>7.1}",
            self.phase.name(),
            self.panels,
            self.frames,
            1000.0 / fps.max(0.001),
            fps
        ));
        self.phase = match self.phase {
            Phase::Moving => Phase::Static,
            Phase::Static => {
                MaterialSettings::set(
                    MaterialSettings {
                        reduced_transparency: true,
                    },
                    cx,
                );
                Phase::Fallback
            }
            Phase::Fallback => {
                let cache = std::env::var("GPUI_BLUR_CACHE").map_or(true, |v| v != "0");
                println!("blur cache: {}", if cache { "on" } else { "off" });
                for r in &self.results {
                    println!("{r}");
                }
                cx.quit();
                return;
            }
        };
        self.started = Instant::now();
        self.frames = 0;
    }
}

impl Render for Bench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let elapsed = self.started.elapsed();
        if elapsed > WARMUP {
            self.frames += 1;
        }
        if elapsed > WARMUP + MEASURE {
            self.next_phase(cx);
        }
        self.tick = self.tick.wrapping_add(1);
        window.request_animation_frame();

        let shift = if self.phase == Phase::Moving {
            (self.tick % 120) as f32
        } else {
            0.
        };
        let mut backdrop = div().absolute().size_full().flex().flex_wrap();
        for i in 0..96 {
            let c = [0xF25022, 0x7FBA00, 0x00A4EF, 0xFFB900][i % 4];
            backdrop = backdrop.child(div().w(px(40. + (i % 3) as f32 * 8.)).h(px(60.)).bg(rgb(c)));
        }
        let mut panels = div().absolute().size_full();
        for i in 0..self.panels {
            let x = 20. + (i % 4) as f32 * 190. + shift;
            let y = 20. + (i / 4) as f32 * 150.;
            panels = panels.child(
                material_surface(Material::regular(Mode::Light))
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .w(px(170.))
                    .h(px(130.))
                    .p(px(12.))
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .child(format!("Panel {i}")),
                    ),
            );
        }
        // A small animated widget away from the panels keeps frames coming
        // in the static phase.
        let pulse = div()
            .absolute()
            .left(px(900.))
            .top(px(600.))
            .size(px(24.))
            .bg(rgb(0x101010 + (self.tick % 200) * 0x010101));
        div()
            .size_full()
            .bg(rgb(0xffffff))
            .child(backdrop)
            .child(panels)
            .child(pulse)
    }
}

fn main() {
    let panels = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(8);
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| Bench {
                    panels,
                    phase: Phase::Moving,
                    started: Instant::now(),
                    frames: 0,
                    tick: 0,
                    results: Vec::new(),
                })
            },
        )
        .expect("open window");
        cx.activate(true);
    });
}
