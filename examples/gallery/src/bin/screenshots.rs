//! Renders every gallery story in its own fixed-size window, in light and
//! dark, and saves a PNG of just that window:
//! `<out>/<story-id>-light.png` and `<out>/<story-id>-dark.png`.
//!
//! ```sh
//! cargo run -p refineui-gallery --bin refineui-screenshots -- --out docs/src/assets/components [--only button,input]
//! ```
//!
//! Prefer `scripts/screenshots.sh`, which builds in release mode and (on Linux)
//! starts a virtual X server. macOS needs the Screen Recording permission for
//! the terminal running it (System Settings → Privacy & Security).

use std::path::PathBuf;
use std::time::Duration;

use gpui::{
    App, AppContext, Application, AsyncApp, Bounds, Context, IntoElement, ParentElement, Render,
    Styled, Window, WindowBounds, WindowHandle, WindowKind, WindowOptions, point, px, size,
};
#[cfg(target_os = "macos")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use refineui::{Appearance, Theme};
use refineui_gallery::{Story, all_stories};

/// Padding around the story inside the capture.
const PADDING: f32 = 24.;

struct ShotView {
    story: Story,
}

impl Render for ShotView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = (self.story.render)(window, cx);
        refineui::root(cx)
            .p(px(PADDING))
            .flex()
            .flex_col()
            .items_start()
            .child(content)
    }
}

struct Args {
    out: PathBuf,
    only: Option<Vec<String>>,
    modes: Vec<Appearance>,
}

fn parse_args() -> Args {
    let mut out = PathBuf::from("docs/src/assets/components");
    let mut only = None;
    let mut modes = vec![Appearance::Light, Appearance::Dark];
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out = args.next().expect("--out <dir>").into(),
            "--only" => {
                only = args
                    .next()
                    .map(|s| s.split(',').map(str::to_owned).collect())
            }
            "--light" => modes = vec![Appearance::Light],
            "--dark" => modes = vec![Appearance::Dark],
            "-h" | "--help" => {
                println!("refineui-screenshots [--out DIR] [--only id,id] [--light|--dark]");
                std::process::exit(0);
            }
            other => eprintln!("ignoring unknown argument {other}"),
        }
    }
    Args { out, only, modes }
}

fn main() {
    let args = parse_args();
    std::fs::create_dir_all(&args.out).expect("create output dir");
    let stories: Vec<Story> = all_stories()
        .into_iter()
        .filter(|s| {
            args.only
                .as_ref()
                .is_none_or(|ids| ids.iter().any(|id| id == s.id))
        })
        .collect();
    let shell_shots = args
        .only
        .as_ref()
        .is_none_or(|ids| ids.iter().any(|id| id == "shell-demo"));
    let modes = args.modes.clone();
    let mut jobs = Vec::new();
    for story in stories {
        for mode in &args.modes {
            jobs.push((story.clone(), *mode));
        }
    }

    Application::new()
        .with_assets(refineui::Assets)
        .run(move |cx: &mut App| {
            refineui::init(cx);
            let out = args.out.clone();
            cx.spawn(async move |cx: &mut AsyncApp| {
                let mut failures = 0;
                // Some platforms quit when the last window closes, so each window is
                // removed only after the next one has opened.
                let mut previous: Option<WindowHandle<ShotView>> = None;
                for (story, mode) in jobs {
                    let name = format!(
                        "{}-{}.png",
                        story.id,
                        if mode == Appearance::Dark {
                            "dark"
                        } else {
                            "light"
                        }
                    );
                    let path = out.join(&name);
                    match shoot(&story, mode, &path, &mut previous, cx).await {
                        Ok(()) => println!("wrote {}", path.display()),
                        Err(e) => {
                            failures += 1;
                            eprintln!("failed {name}: {e:#}");
                        }
                    }
                }
                // The real `refineui-shell` window (id `shell-demo`).
                if shell_shots {
                    for mode in &modes {
                        let name = format!(
                            "shell-demo-{}.png",
                            if *mode == Appearance::Dark {
                                "dark"
                            } else {
                                "light"
                            }
                        );
                        let path = out.join(&name);
                        match shoot_shell(*mode, &path, cx).await {
                            Ok(()) => println!("wrote {}", path.display()),
                            Err(e) => {
                                failures += 1;
                                eprintln!("failed {name}: {e:#}");
                            }
                        }
                    }
                }
                if let Some(prev) = previous.take() {
                    prev.update(cx, |_, window, _| window.remove_window()).ok();
                }
                if failures > 0 {
                    eprintln!("{failures} screenshot(s) failed");
                }
                cx.update(|cx| cx.quit()).ok();
            })
            .detach();
        });
}

/// Size of the `shell-demo` capture (logical px).
const SHELL_SHOT: (f32, f32) = (1180., 720.);

async fn shoot_shell(
    mode: Appearance,
    path: &std::path::Path,
    cx: &mut AsyncApp,
) -> anyhow::Result<()> {
    use refineui_gallery::{demo, shell_demo::ShellDemo};
    let title = format!("refineui-shot-shell-{}", std::process::id());
    let state = std::env::temp_dir().join(format!("refineui-shot-shell-{}", std::process::id()));
    let handle: WindowHandle<ShellDemo> = cx.update(|cx| {
        demo::install(cx);
        Theme::set_appearance(mode, cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(40.), px(80.)),
                    size: size(px(SHELL_SHOT.0), px(SHELL_SHOT.1)),
                })),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(title.clone().into()),
                    ..Default::default()
                }),
                focus: true,
                show: true,
                is_movable: false,
                is_resizable: false,
                kind: if cfg!(target_os = "macos") {
                    WindowKind::PopUp
                } else {
                    WindowKind::Normal
                },
                ..Default::default()
            },
            |window, cx| cx.new(|cx| ShellDemo::new(state.join("window.state"), window, cx)),
        )
    })??;
    for _ in 0..8 {
        cx.background_executor()
            .timer(Duration::from_millis(150))
            .await;
        handle.update(cx, |_, window, _| window.refresh())?;
        capture::nudge(&title);
    }
    cx.background_executor()
        .timer(Duration::from_millis(400))
        .await;
    let r = handle
        .update(cx, |_, window, _| capture::capture(window, &title, path))
        .and_then(|r| r);
    handle
        .update(cx, |_, window, _| window.remove_window())
        .ok();
    r
}

async fn shoot(
    story: &Story,
    mode: Appearance,
    path: &std::path::Path,
    previous: &mut Option<WindowHandle<ShotView>>,
    cx: &mut AsyncApp,
) -> anyhow::Result<()> {
    let story_clone = story.clone();
    let (w, h) = story.shot;
    let title = format!("refineui-shot-{}-{}", story.id, std::process::id());
    let handle: WindowHandle<ShotView> = cx.update(|cx| {
        Theme::set_appearance(mode, cx);
        let bounds = Bounds {
            origin: point(px(40.), px(80.)),
            size: size(px(w), px(h)),
        };
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                // No window manager on a headless X server, so the title bar never draws;
                // the title lets the Linux capture find this window.
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(title.clone().into()),
                    ..Default::default()
                }),
                focus: false,
                show: true,
                is_movable: false,
                is_resizable: false,
                kind: if cfg!(target_os = "macos") {
                    WindowKind::PopUp
                } else {
                    WindowKind::Normal
                },
                ..Default::default()
            },
            |_, cx| cx.new(|_| ShotView { story: story_clone }),
        )
    })??;
    if let Some(prev) = previous.replace(handle) {
        prev.update(cx, |_, window, _| window.remove_window()).ok();
    }

    // Let layout, fonts, images and enter animations settle; poke the window so
    // platforms that only draw on demand (e.g. a headless X server) present a frame.
    for _ in 0..6 {
        cx.background_executor()
            .timer(Duration::from_millis(150))
            .await;
        handle.update(cx, |_, window, _| window.refresh())?;
        capture::nudge(&title);
    }
    cx.background_executor()
        .timer(Duration::from_millis(400))
        .await;

    handle
        .update(cx, |_, window, _| capture::capture(window, &title, path))
        .and_then(|r| r)?;
    autocrop(path, w)
}

/// Trim uniform background on the right and bottom (stories declare a generous
/// canvas), keeping `PADDING` around the content.
fn autocrop(path: &std::path::Path, logical_width: f32) -> anyhow::Result<()> {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path)?));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return Ok(()),
    };
    anyhow::ensure!(
        info.bit_depth == png::BitDepth::Eight,
        "unexpected bit depth"
    );
    let (w, h) = (info.width as usize, info.height as usize);
    let at = |x: usize, y: usize| &buf[(y * w + x) * channels..(y * w + x) * channels + 3];
    let bg = at(w - 1, h - 1).to_vec();
    let (mut max_x, mut max_y) = (0, 0);
    for y in 0..h {
        for x in 0..w {
            let p = at(x, y);
            if p.iter().zip(&bg).any(|(a, b)| a.abs_diff(*b) > 3) {
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    let pad = (PADDING * w as f32 / logical_width).round() as usize;
    let (cw, ch) = ((max_x + 1 + pad).min(w), (max_y + 1 + pad).min(h));
    if (cw, ch) == (w, h) {
        return Ok(());
    }
    let mut out = Vec::with_capacity(cw * ch * channels);
    for y in 0..ch {
        out.extend_from_slice(&buf[y * w * channels..(y * w + cw) * channels]);
    }
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, cw as u32, ch as u32);
    enc.set_color(info.color_type);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&out)?;
    Ok(())
}

#[cfg(target_os = "macos")]
mod capture {
    use super::*;

    pub fn nudge(_title: &str) {}

    #[allow(unsafe_code)]
    fn window_number(raw: RawWindowHandle) -> Option<isize> {
        use objc2::{msg_send, runtime::AnyObject};
        let RawWindowHandle::AppKit(h) = raw else {
            return None;
        };
        // SAFETY: `ns_view` is a live NSView owned by GPUI for the duration of this call;
        // `-window` and `-windowNumber` are side-effect-free getters.
        unsafe {
            let view: &AnyObject = h.ns_view.cast::<AnyObject>().as_ref();
            let win: *mut AnyObject = msg_send![view, window];
            let win = win.as_ref()?;
            let n: isize = msg_send![win, windowNumber];
            Some(n)
        }
    }

    pub fn capture(window: &Window, _title: &str, path: &std::path::Path) -> anyhow::Result<()> {
        let raw = HasWindowHandle::window_handle(window)
            .map_err(|e| anyhow::anyhow!("no window handle: {e:?}"))?
            .as_raw();
        let id = window_number(raw).ok_or_else(|| anyhow::anyhow!("no NSWindow for view"))?;
        // -l: capture this window only, -o: no shadow, -x: no sound.
        let status = std::process::Command::new("screencapture")
            .arg(format!("-l{id}"))
            .args(["-o", "-x", "-t", "png"])
            .arg(path)
            .status()?;
        anyhow::ensure!(
            status.success(),
            "screencapture exited with {status} (grant Screen Recording permission?)"
        );
        anyhow::ensure!(
            path.exists(),
            "screencapture produced no file (grant Screen Recording permission?)"
        );
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod capture {
    use super::*;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, ImageFormat};
    use x11rb::rust_connection::RustConnection;

    /// Find a top-level window by its title (WM_NAME / _NET_WM_NAME).
    fn find_window(conn: &RustConnection, root: u32, title: &str) -> Option<u32> {
        let net_wm_name = conn
            .intern_atom(false, b"_NET_WM_NAME")
            .ok()?
            .reply()
            .ok()?
            .atom;
        let utf8 = conn
            .intern_atom(false, b"UTF8_STRING")
            .ok()?
            .reply()
            .ok()?
            .atom;
        let mut stack = vec![root];
        while let Some(w) = stack.pop() {
            for (prop, ty) in [
                (net_wm_name, utf8),
                (AtomEnum::WM_NAME.into(), AtomEnum::STRING.into()),
            ] {
                if let Ok(Ok(r)) = conn
                    .get_property(false, w, prop, ty, 0, 1024)
                    .map(|c| c.reply())
                    && r.value == title.as_bytes()
                {
                    return Some(w);
                }
            }
            if let Ok(Ok(tree)) = conn.query_tree(w).map(|c| c.reply()) {
                stack.extend(tree.children);
            }
        }
        None
    }

    fn connect_and_find(title: &str) -> anyhow::Result<(RustConnection, u32)> {
        let (conn, screen) = x11rb::connect(None)?;
        let root = conn.setup().roots[screen].root;
        let win = find_window(&conn, root, title).ok_or_else(|| {
            anyhow::anyhow!("window {title:?} not found (X11/Xvfb required, not Wayland)")
        })?;
        Ok((conn, win))
    }

    /// Move the pointer inside the window: headless X servers only present a
    /// GPUI frame after an input event.
    pub fn nudge(title: &str) {
        static TICK: std::sync::atomic::AtomicI16 = std::sync::atomic::AtomicI16::new(0);
        let Ok((conn, win)) = connect_and_find(title) else {
            return;
        };
        let t = TICK.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 2;
        let _ = conn.warp_pointer(x11rb::NONE, win, 0, 0, 0, 0, 2 + t, 2 + t);
        let _ = conn.flush();
    }

    pub fn capture(_window: &Window, title: &str, path: &std::path::Path) -> anyhow::Result<()> {
        let (conn, win) = connect_and_find(title)?;
        let geo = conn.get_geometry(win)?.reply()?;
        let (w, h) = (geo.width as usize, geo.height as usize);
        let img = conn
            .get_image(ImageFormat::Z_PIXMAP, win, 0, 0, geo.width, geo.height, !0)?
            .reply()?;
        anyhow::ensure!(
            img.data.len() >= w * h * 4,
            "unexpected image format (depth {})",
            img.depth
        );
        let mut rgb = Vec::with_capacity(w * h * 3);
        for px in img.data.as_chunks::<4>().0.iter().take(w * h) {
            rgb.extend_from_slice(&[px[2], px[1], px[0]]); // BGRX → RGB
        }
        let file = std::io::BufWriter::new(std::fs::File::create(path)?);
        let mut enc = png::Encoder::new(file, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()?.write_image_data(&rgb)?;
        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod capture {
    use super::*;
    pub fn nudge(_title: &str) {}
    pub fn capture(_window: &Window, _title: &str, _path: &std::path::Path) -> anyhow::Result<()> {
        anyhow::bail!("screenshots are supported on macOS and Linux (X11)")
    }
}
