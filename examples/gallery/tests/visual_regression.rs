//! Visual regression: renders a representative set of gallery stories with
//! the `refineui-screenshots` binary (light + dark) and compares each PNG
//! against the committed baseline in `tests/visual/baselines/<os>/`.
//!
//! A pixel differs when any RGBA channel differs by more than
//! `REFINEUI_VISUAL_THRESHOLD` (default 24 / 255, absorbs GPU and font
//! anti-aliasing noise); a story fails when more than
//! `REFINEUI_VISUAL_TOLERANCE` of its pixels differ (default 0.005 = 0.5%)
//! or its size changed. Failures write `actual` and `diff` PNGs to
//! `target/visual-regression/`.
//!
//! The test **skips** (passes with a note on stderr) when it cannot render:
//! `REFINEUI_VISUAL=0`, Linux without `$DISPLAY`, no baselines for this OS,
//! or the renderer fails (no GPU / no screen-recording permission) or times
//! out. Set `REFINEUI_VISUAL_STRICT=1` to turn those skips into failures
//! (CI with a display). `REFINEUI_VISUAL_UPDATE=1` rewrites the baselines.
//! Use `scripts/visual-regression.sh`, which sets all of this up.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Stories under regression: basic control, a scrolling material list,
/// raw materials, components on materials, opaque shell, and the Phase 5
/// desktop pieces.
const STORIES: &[&str] = &[
    "button",
    "select",
    "material-recipes",
    "material-components",
    "application-shell",
    "preferences-window",
    "notification-center",
    "result",
    "activity-bar",
    "desktop-showcase",
];

fn os_dir() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1" || v == "true")
}

/// Skip (or, in strict mode, fail) with a reason.
fn skip(reason: &str) {
    if flag("REFINEUI_VISUAL_STRICT") {
        panic!("visual regression could not run: {reason}");
    }
    eprintln!("visual regression SKIPPED: {reason}");
}

struct Image {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

fn read_png(path: &Path) -> Result<Image, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut dec = png::Decoder::new(file);
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let px = (info.width * info.height) as usize;
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("indexed PNG not expanded".into()),
    };
    assert_eq!(rgba.len(), px * 4);
    Ok(Image {
        w: info.width,
        h: info.height,
        rgba,
    })
}

fn write_png(path: &Path, img: &Image) {
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.w, img.h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut w| w.write_image_data(&img.rgba))
        .expect("write png");
}

/// Fraction of differing pixels and a diff image (red = differs, faded
/// baseline elsewhere).
fn compare(a: &Image, b: &Image, threshold: u8) -> (f32, Image) {
    let mut diff = Vec::with_capacity(a.rgba.len());
    let mut bad = 0usize;
    for (pa, pb) in a
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.rgba.as_chunks::<4>().0)
    {
        let differs = pa.iter().zip(pb).any(|(x, y)| x.abs_diff(*y) > threshold);
        if differs {
            bad += 1;
            diff.extend_from_slice(&[255, 0, 64, 255]);
        } else {
            let l = ((pa[0] as u32 + pa[1] as u32 + pa[2] as u32) / 3) as u8;
            let f = 200 + l / 5;
            diff.extend_from_slice(&[f, f, f, 255]);
        }
    }
    let n = (a.w * a.h).max(1) as f32;
    (
        bad as f32 / n,
        Image {
            w: a.w,
            h: a.h,
            rgba: diff,
        },
    )
}

#[test]
fn compare_counts_differing_pixels() {
    let a = Image {
        w: 2,
        h: 1,
        rgba: vec![0, 0, 0, 255, 10, 10, 10, 255],
    };
    let b = Image {
        w: 2,
        h: 1,
        rgba: vec![0, 0, 20, 255, 200, 10, 10, 255],
    };
    let (ratio, diff) = compare(&a, &b, 24);
    assert_eq!(ratio, 0.5);
    assert_eq!(&diff.rgba[4..8], &[255, 0, 64, 255]);
}

#[test]
fn stories_match_baselines() {
    if std::env::var("REFINEUI_VISUAL").is_ok_and(|v| v == "0") {
        return skip("REFINEUI_VISUAL=0");
    }
    if cfg!(target_os = "linux")
        && std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
    {
        return skip("no $DISPLAY (run scripts/visual-regression.sh, which starts Xvfb)");
    }
    let update = flag("REFINEUI_VISUAL_UPDATE");
    let root = root();
    let baselines = root.join("tests/visual/baselines").join(os_dir());
    if !update && !baselines.is_dir() {
        return skip(&format!(
            "no baselines for {} (create them with scripts/visual-regression.sh --update)",
            os_dir()
        ));
    }
    let only: Vec<String> = std::env::var("REFINEUI_VISUAL_ONLY")
        .ok()
        .map(|s| s.split(',').map(str::to_string).collect())
        .unwrap_or_else(|| STORIES.iter().map(|s| s.to_string()).collect());

    let out = root.join("target/visual-regression");
    let actual = out.join("actual");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&actual).expect("output dir");

    // Render.
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_refineui-screenshots"));
    cmd.arg("--out")
        .arg(&actual)
        .arg("--only")
        .arg(only.join(","))
        .current_dir(&root)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if cfg!(target_os = "linux") && std::env::var_os("GPUI_X11_SCALE_FACTOR").is_none() {
        // Same scale as scripts/screenshots.sh and the committed baselines.
        cmd.env("GPUI_X11_SCALE_FACTOR", "2");
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return skip(&format!("cannot start the renderer: {e}")),
    };
    let limit = Duration::from_secs(env_f32("REFINEUI_VISUAL_TIMEOUT", 600.) as u64);
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() > limit => {
                let _ = child.kill();
                return skip("renderer timed out");
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(200)),
            Err(e) => return skip(&format!("renderer wait failed: {e}")),
        }
    };
    if !status.success() {
        let mut err = String::new();
        if let Some(mut s) = child.stderr.take() {
            use std::io::Read;
            let _ = s.read_to_string(&mut err);
        }
        let tail: String = err.lines().rev().take(5).collect::<Vec<_>>().join(" | ");
        return skip(&format!(
            "renderer failed ({status}); no GPU / display / screen-recording permission? {tail}"
        ));
    }

    if update {
        std::fs::create_dir_all(&baselines).expect("baseline dir");
        for id in &only {
            for mode in ["light", "dark"] {
                let name = format!("{id}-{mode}.png");
                std::fs::copy(actual.join(&name), baselines.join(&name))
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
            }
        }
        eprintln!(
            "visual regression: updated {} baselines in {}",
            only.len() * 2,
            baselines.display()
        );
        return;
    }

    let threshold = env_f32("REFINEUI_VISUAL_THRESHOLD", 24.).clamp(0., 255.) as u8;
    let tolerance = env_f32("REFINEUI_VISUAL_TOLERANCE", 0.005);
    let mut failures = Vec::new();
    let mut report = Vec::new();
    for id in &only {
        for mode in ["light", "dark"] {
            let name = format!("{id}-{mode}.png");
            let base_path = baselines.join(&name);
            if !base_path.exists() {
                failures.push(format!("{name}: no baseline (run with --update)"));
                continue;
            }
            let base = read_png(&base_path).expect("baseline png");
            let got = read_png(&actual.join(&name)).expect("rendered png");
            if (base.w, base.h) != (got.w, got.h) {
                failures.push(format!(
                    "{name}: size {}x{} ≠ baseline {}x{}",
                    got.w, got.h, base.w, base.h
                ));
                continue;
            }
            let (ratio, diff) = compare(&got, &base, threshold);
            report.push(format!("{name}: {:.3}% differ", ratio * 100.));
            if ratio > tolerance {
                write_png(&out.join(format!("{id}-{mode}.diff.png")), &diff);
                failures.push(format!(
                    "{name}: {:.3}% of pixels differ (tolerance {:.3}%), see target/visual-regression/",
                    ratio * 100.,
                    tolerance * 100.
                ));
            }
        }
    }
    eprintln!(
        "visual regression ({}):\n  {}",
        os_dir(),
        report.join("\n  ")
    );
    assert!(
        failures.is_empty(),
        "visual regressions:\n{}",
        failures.join("\n")
    );
}
