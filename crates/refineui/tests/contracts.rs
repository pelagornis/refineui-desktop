//! Component contract validator (brief §8: contracts are machine-readable).
//!
//! * every contract matches the schema's required shape and enums;
//! * embedded density sizes equal the Rust `Density` metrics;
//! * every public component type in `crates/refineui/src` is covered by a
//!   contract, and every type a contract names exists;
//! * no action is hover-only; focusable components define keyboard
//!   behaviour.

use std::collections::BTreeSet;
use std::path::Path;

use refineui::tokens::contracts::{self, InteractionState};
use refineui::{ControlSize, Density};
use serde_json::Value;

/// Not components: infrastructure views.
const NOT_COMPONENTS: &[&str] = &["FrostedPanelWindow", "FrostedMenuWindow"];

fn contracts() -> Vec<Value> {
    contracts::ALL
        .iter()
        .map(|c| serde_json::from_str(c.json).unwrap_or_else(|e| panic!("{}.json: {e}", c.id)))
        .collect()
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn contracts_match_the_schema() {
    let schema: Value = serde_json::from_str(contracts::SCHEMA).unwrap();
    let required = strs(&schema["required"]);
    let allowed: BTreeSet<String> = schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let en = |path: &[&str]| -> Vec<String> {
        let mut v = &schema["properties"];
        for p in path {
            v = &v[*p];
        }
        strs(&v["enum"])
    };
    let layers = en(&["layer"]);
    let statuses = en(&["status"]);
    let metrics = {
        let v = &schema["properties"]["sizes"]["properties"]["metric"]["enum"];
        strs(v)
    };
    let tab_stops = strs(&schema["properties"]["focus"]["properties"]["tab_stop"]["enum"]);
    let cursors = strs(&schema["properties"]["pointer"]["properties"]["cursor"]["enum"]);
    for c in contracts() {
        let id = c["id"].as_str().unwrap().to_string();
        let obj = c.as_object().unwrap();
        for r in &required {
            assert!(obj.contains_key(r), "{id}: missing `{r}`");
        }
        for k in obj.keys() {
            assert!(allowed.contains(k), "{id}: unknown key `{k}`");
        }
        assert!(
            layers.contains(&c["layer"].as_str().unwrap().to_string()),
            "{id}: layer"
        );
        let status = c["status"].as_str().unwrap();
        assert!(statuses.contains(&status.to_string()), "{id}: status");
        if status == "optional" {
            assert_eq!(
                c["feature"].as_str(),
                Some("web-extras"),
                "{id}: optional needs a feature"
            );
        }
        let phase = c["phase"].as_u64().unwrap();
        assert!((1..=6).contains(&phase), "{id}: phase");
        assert!(
            !strs(&c["rust_types"]).is_empty() || status == "new",
            "{id}: rust_types"
        );
        // States: all eight keys, each null or {status, visual}.
        let states = c["states"].as_object().unwrap();
        assert_eq!(states.len(), 8, "{id}: states");
        for s in InteractionState::ALL {
            let v = &states[s.as_str()];
            assert!(
                v.is_null() || matches!(v["status"].as_str(), Some("implemented" | "planned")),
                "{id}: state {}",
                s.as_str()
            );
        }
        assert!(
            !states["default"].is_null(),
            "{id}: default state is required"
        );
        assert!(
            metrics.contains(&c["sizes"]["metric"].as_str().unwrap().to_string()),
            "{id}: metric"
        );
        let tab = c["focus"]["tab_stop"].as_str().unwrap();
        assert!(tab_stops.contains(&tab.to_string()), "{id}: tab_stop");
        assert!(
            cursors.contains(&c["pointer"]["cursor"].as_str().unwrap().to_string()),
            "{id}: cursor"
        );
        // Brief §7: nothing is operable by hover alone.
        assert_eq!(
            c["pointer"]["hover_only"].as_bool(),
            Some(false),
            "{id}: hover_only"
        );
        // Focusable components document their keys.
        let keys = c["keyboard"].as_array().unwrap();
        if matches!(tab, "single" | "roving") {
            assert!(!keys.is_empty(), "{id}: focusable but no keyboard map");
        }
        for k in keys {
            assert!(
                k["keys"].as_str().is_some_and(|s| !s.is_empty()),
                "{id}: key"
            );
            assert!(
                k["action"].as_str().is_some_and(|s| !s.is_empty()),
                "{id}: action"
            );
        }
        // Focused state ⇔ focusable.
        if tab == "none" {
            assert!(
                states["focused"].is_null(),
                "{id}: not focusable but has a focused state"
            );
        }
    }
}

#[test]
fn contract_sizes_equal_density_metrics() {
    let f = |v: &Value, k: &str| v[k].as_f64().unwrap() as f32;
    for c in contracts() {
        let id = c["id"].as_str().unwrap();
        let sizes = &c["sizes"];
        let metric = sizes["metric"].as_str().unwrap();
        for d in Density::ALL {
            let got = &sizes[d.label()];
            match metric {
                "button" | "field" | "select" | "chip" => {
                    for s in ControlSize::ALL {
                        let m = match metric {
                            "button" => d.button(s),
                            "field" => d.field(s),
                            "select" => d.select(s),
                            _ => d.chip(s),
                        };
                        let g = &got[s.label()];
                        assert_eq!(f(g, "height"), m.height, "{id} {} {}", d.label(), s.label());
                        assert_eq!(f(g, "padding_x"), m.padding_x, "{id} padding_x");
                        assert_eq!(f(g, "padding_y"), m.padding_y, "{id} padding_y");
                        assert_eq!(f(g, "radius"), m.radius, "{id} radius");
                        assert_eq!(f(g, "text_size"), m.text.size, "{id} text");
                        assert_eq!(f(g, "icon"), m.icon, "{id} icon");
                    }
                }
                "row" => assert_eq!(f(got, "row_height"), d.desktop().row_height, "{id}"),
                "desktop" => {
                    let m = d.desktop();
                    assert_eq!(f(got, "row_height"), m.row_height, "{id}");
                    assert_eq!(f(got, "toolbar_height"), m.toolbar_height, "{id}");
                    assert_eq!(f(got, "activity_bar_width"), m.activity_bar_width, "{id}");
                    assert_eq!(f(got, "panel_padding"), m.panel_padding, "{id}");
                }
                _ => {}
            }
        }
    }
}

/// Public component types: `pub struct X` that derive `IntoElement` or
/// have `impl Render for X`.
fn public_components() -> BTreeSet<String> {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(std::fs::read_to_string(&p).unwrap());
            }
        }
    }
    let mut files = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let mut found = BTreeSet::new();
    for src in &files {
        let lines: Vec<&str> = src.lines().collect();
        for (i, l) in lines.iter().enumerate() {
            let Some(rest) = l.strip_prefix("pub struct ") else {
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            // Attributes directly above.
            let mut j = i;
            let mut derives_element = false;
            while j > 0 && (lines[j - 1].starts_with("#[") || lines[j - 1].starts_with("///")) {
                j -= 1;
                if lines[j].starts_with("#[derive") && lines[j].contains("IntoElement") {
                    derives_element = true;
                }
            }
            let renders = files.iter().any(|f| {
                f.contains(&format!("impl Render for {name} "))
                    || f.contains(&format!("impl gpui::Render for {name} "))
            });
            if (derives_element || renders) && !NOT_COMPONENTS.contains(&name.as_str()) {
                found.insert(name);
            }
        }
    }
    found
}

#[test]
fn every_public_component_has_a_contract() {
    let covered: BTreeSet<String> = contracts()
        .iter()
        .flat_map(|c| strs(&c["rust_types"]))
        .collect();
    let public = public_components();
    assert!(public.len() > 50, "scanner found only {public:?}");
    let missing: Vec<_> = public.difference(&covered).collect();
    assert!(
        missing.is_empty(),
        "components without a contract: {missing:?}"
    );
    let stale: Vec<_> = covered.difference(&public).collect();
    assert!(stale.is_empty(), "contracts name unknown types: {stale:?}");
}
