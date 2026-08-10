//! `lspconf bench` — per-request-kind latency, in the D5 JSONL shape,
//! **report-only**.
//!
//! Sprint §8 is explicit that the gate flip is a flag in wolf-lang's CI and not
//! work here. The duty is that the numbers exist, in the right shape, from the
//! first server that answers a request — so that when s57 lands residency there
//! is a baseline to have beaten rather than a graph starting at zero.
//!
//! Shape (`bench-results/*.jsonl`, one object per line):
//!
//! ```text
//! {"bench":"lsp/textDocument/hover","track":"compile","metric":"p50_ms",
//!  "value":41.2,"unit":"ms","commit":"<wolf pin>","config":{…}}
//! ```
//!
//! D36 discipline from day one even while report-only: **N ≥ 10 paired runs,
//! median and MAD, wall time reported as noisy.** The median is what gets
//! recorded and the MAD is recorded beside it, because a p50 with no spread is
//! a number that cannot be argued with and therefore cannot be trusted. Two
//! runs at the same commit must not show a significant delta — [`significant`]
//! is the test, and it is deliberately loose: interactive latency carries
//! client debounce, filesystem state, and first-run effects that a compile-time
//! benchmark does not.
//!
//! # The five classes
//!
//! Report 09's table, restricted to what v0 actually answers. A class with no
//! implemented request in it is **absent from the output**, not present with a
//! zero — a zero would read as "instantaneous" on any graph anybody builds.

use std::fmt;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::profiles::Profile;
use crate::session::{self, Session, file_uri};

/// Minimum paired runs. D36's floor; below it the median means nothing.
pub const MIN_RUNS: usize = 10;

/// Report 09's budget classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// hover, signatureHelp — p95 ≤ 200 ms at v0.
    Instant,
    /// completion, goto-def, documentSymbol — p95 ≤ 400 ms at v0.
    Interactive,
    /// references, workspace/symbol, rename — p95 ≤ 1500 ms at v0. At v0 the
    /// only implemented member is `codeAction`, which resolves fix-its and is
    /// the closest thing the capability set has to a whole-file query.
    Deliberate,
    /// edit → publish — p95 ≤ 2000 ms at v0.
    Background,
    /// initialize → first diagnostics — ≤ 5000 ms. Not arbitrary: report 09
    /// records fackr blocking its UI thread for up to 5 s waiting for a server
    /// to become ready, so exceeding it freezes the user's editor.
    Cold,
}

impl Class {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Class::Instant => "instant",
            Class::Interactive => "interactive",
            Class::Deliberate => "deliberate",
            Class::Background => "background",
            Class::Cold => "cold",
        }
    }

    /// The v0 (non-resident) p95 budget, milliseconds. Recorded in `config`
    /// so a reader of the JSONL can see what the number was measured against,
    /// even though nothing gates on it yet.
    #[must_use]
    pub fn budget_ms(self) -> f64 {
        match self {
            Class::Instant => 200.0,
            Class::Interactive => 400.0,
            Class::Deliberate => 1500.0,
            Class::Background => 2000.0,
            Class::Cold => 5000.0,
        }
    }

    /// The class a request kind belongs to.
    #[must_use]
    pub fn of(bench: &str) -> Class {
        match bench {
            "lsp/textDocument/hover" | "lsp/textDocument/signatureHelp" => Class::Instant,
            "lsp/textDocument/documentSymbol"
            | "lsp/textDocument/completion"
            | "lsp/textDocument/definition"
            | "lsp/textDocument/formatting" => Class::Interactive,
            "lsp/textDocument/codeAction"
            | "lsp/textDocument/references"
            | "lsp/workspace/symbol" => Class::Deliberate,
            "lsp/diagnostics-after-edit" => Class::Background,
            _ => Class::Cold,
        }
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One measured request kind, with every sample kept.
#[derive(Debug, Clone)]
pub struct Series {
    pub bench: String,
    pub class: Class,
    pub samples_ms: Vec<f64>,
}

impl Series {
    #[must_use]
    pub fn p50(&self) -> f64 {
        percentile(&self.samples_ms, 0.50)
    }

    #[must_use]
    pub fn p95(&self) -> f64 {
        percentile(&self.samples_ms, 0.95)
    }

    /// Median absolute deviation — the spread D36 wants beside every median.
    #[must_use]
    pub fn mad(&self) -> f64 {
        let med = self.p50();
        let devs: Vec<f64> = self.samples_ms.iter().map(|v| (v - med).abs()).collect();
        percentile(&devs, 0.50)
    }

    /// The D5 records for this series.
    #[must_use]
    pub fn records(&self, commit: &str, config: &Value) -> Vec<Value> {
        let mut config = config.clone();
        config["class"] = Value::from(self.class.as_str());
        config["budget_p95_ms"] = json!(self.class.budget_ms());
        config["runs"] = json!(self.samples_ms.len());
        let metric = |metric: &str, value: f64| {
            json!({
                "bench": self.bench,
                // Sprint §8 fixes this to the compile-time track: LSP latency
                // is the interactive face of the same rebuild architecture
                // (D9), and giving it its own track would let the two drift
                // apart in the one place they must not.
                "track": "compile",
                "metric": metric,
                "value": round3(value),
                "unit": "ms",
                "commit": commit,
                "config": config,
            })
        };
        vec![
            metric("p50_ms", self.p50()),
            metric("p95_ms", self.p95()),
            metric("mad_ms", self.mad()),
        ]
    }
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// Nearest-rank percentile over a copy of the samples.
#[must_use]
pub fn percentile(samples: &[f64], q: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut v = samples.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let rank = (q * v.len() as f64).ceil().max(1.0) as usize;
    v[rank.min(v.len()) - 1]
}

/// Is the difference between two medians worth reporting?
///
/// Loose on purpose (§8, L4): interactive latency is noisier than a
/// compile-time benchmark, so the noise floor is the larger of the two MADs
/// and a fixed millisecond floor. Under-reporting a real regression is
/// recoverable; crying wolf on every run produces waiver fatigue, and a gate
/// nobody believes is worse than no gate — which is why this is report-only in
/// the first place.
#[must_use]
pub fn significant(a: &Series, b: &Series) -> bool {
    let floor = 2.0_f64.max(a.mad().max(b.mad()) * 3.0);
    (a.p50() - b.p50()).abs() > floor
}

/// What a bench run produced.
#[derive(Debug, Clone, Default)]
pub struct Run {
    pub series: Vec<Series>,
    /// Set when the run could not reach `MIN_RUNS`; the numbers are still
    /// emitted, and they say so.
    pub under_sampled: bool,
}

/// Errors from a bench run.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Session(session::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Session(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<session::Error> for Error {
    fn from(e: session::Error) -> Self {
        Error::Session(e)
    }
}

/// Measure every v0 request kind over one sample, `runs` times.
///
/// Timed from "the client writes the framed request" to "the client reads the
/// framed response", which is the closest a black-box harness can get to
/// report 09's shim-to-shim definition. Process start is excluded from every
/// class except `cold`, which is defined to include it.
pub fn measure(
    bin: &Path,
    workspace: &Path,
    sample: &str,
    profile: &Profile,
    runs: usize,
) -> Result<Run, Error> {
    let path = workspace.join(sample);
    let bytes = std::fs::read(&path)?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let uri = file_uri(&path);

    let mut cold = Vec::new();
    let mut hover = Vec::new();
    let mut symbols = Vec::new();
    let mut formatting = Vec::new();
    let mut code_action = Vec::new();
    let mut after_edit = Vec::new();

    for run in 0..runs {
        // A fresh process per run: the v0 server is the *non-resident*
        // compiler, so reusing one would measure a warmed query host that no
        // editor will ever have and would flatter the numbers s57 has to beat.
        let spawn_at = Instant::now();
        let mut session = Session::spawn(bin, workspace)?;
        session.initialize(&profile.capabilities)?;
        session.notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "wolf",
                                    "version": 1, "text": text}}),
        )?;
        let _ = session.notification("textDocument/publishDiagnostics", Some(&uri), spawn_at)?;
        cold.push(ms(spawn_at.elapsed()));

        hover.push(time(
            &mut session,
            "textDocument/hover",
            &json!({
            "textDocument": {"uri": uri}, "position": {"line": 0, "character": 0}}),
        )?);
        symbols.push(time(
            &mut session,
            "textDocument/documentSymbol",
            &json!({
            "textDocument": {"uri": uri}}),
        )?);
        formatting.push(time(
            &mut session,
            "textDocument/formatting",
            &json!({
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": true}}),
        )?);
        code_action.push(time(
            &mut session,
            "textDocument/codeAction",
            &json!({
            "textDocument": {"uri": uri},
            "range": {"start": {"line": 0, "character": 0},
                      "end": {"line": 0, "character": 0}},
            "context": {"diagnostics": []}}),
        )?);

        // Background: the edit→publish path, which carries the server's own
        // 100 ms debounce. Reported as measured — the debounce is part of what
        // the user waits for, and subtracting it would report a latency nobody
        // experiences.
        let edited = format!("{text}\n// bench edit {run}\n");
        let edit_at = Instant::now();
        session.notify(
            "textDocument/didChange",
            json!({"textDocument": {"uri": uri, "version": 2 + run},
                   "contentChanges": [{"text": edited}]}),
        )?;
        let (_, took) =
            session.notification("textDocument/publishDiagnostics", Some(&uri), edit_at)?;
        after_edit.push(ms(took));

        session.shutdown_exit()?;
    }

    let series = |bench: &str, samples_ms: Vec<f64>| Series {
        bench: bench.to_string(),
        class: Class::of(bench),
        samples_ms,
    };
    Ok(Run {
        series: vec![
            series("lsp/cold-first-diagnostics", cold),
            series("lsp/textDocument/hover", hover),
            series("lsp/textDocument/documentSymbol", symbols),
            series("lsp/textDocument/formatting", formatting),
            series("lsp/textDocument/codeAction", code_action),
            series("lsp/diagnostics-after-edit", after_edit),
        ],
        under_sampled: runs < MIN_RUNS,
    })
}

fn time(session: &mut Session, method: &str, params: &Value) -> Result<f64, session::Error> {
    let started = Instant::now();
    let id = session.request(method, params.clone())?;
    let (_, took) = session.response(id, started)?;
    Ok(ms(took))
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Render a run as D5 JSONL.
#[must_use]
pub fn to_jsonl(run: &Run, commit: &str, config: &Value) -> String {
    let mut out = String::new();
    for series in &run.series {
        for record in series.records(commit, config) {
            let mut v = record;
            lsp_transcript::record::sort_keys(&mut v);
            out.push_str(&v.to_string());
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(bench: &str, samples: &[f64]) -> Series {
        Series {
            bench: bench.to_string(),
            class: Class::of(bench),
            samples_ms: samples.to_vec(),
        }
    }

    #[test]
    fn percentiles_are_nearest_rank_and_do_not_interpolate() {
        let s = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert_eq!(percentile(&s, 0.5), 5.0);
        assert_eq!(percentile(&s, 0.95), 10.0);
        assert_eq!(percentile(&s, 0.0), 1.0);
    }

    #[test]
    fn an_empty_series_is_zero_rather_than_a_panic() {
        assert_eq!(percentile(&[], 0.5), 0.0);
    }

    #[test]
    fn mad_is_zero_for_a_flat_series_and_positive_for_a_spread_one() {
        assert_eq!(series("lsp/textDocument/hover", &[5.0; 10]).mad(), 0.0);
        assert!(series("lsp/textDocument/hover", &[1.0, 5.0, 9.0]).mad() > 0.0);
    }

    #[test]
    fn two_runs_at_the_same_commit_are_not_significant() {
        // The acceptance criterion, as a unit: same distribution, jittered.
        let a = series("lsp/textDocument/hover", &[40.0, 41.0, 42.0, 40.5, 41.5]);
        let b = series("lsp/textDocument/hover", &[41.0, 40.0, 42.5, 41.2, 40.8]);
        assert!(!significant(&a, &b));
    }

    #[test]
    fn a_tenfold_regression_is_significant() {
        let a = series("lsp/textDocument/hover", &[40.0, 41.0, 42.0]);
        let b = series("lsp/textDocument/hover", &[400.0, 410.0, 420.0]);
        assert!(significant(&a, &b));
    }

    #[test]
    fn every_v0_request_kind_lands_in_one_of_the_five_classes() {
        for (bench, class) in [
            ("lsp/textDocument/hover", Class::Instant),
            ("lsp/textDocument/documentSymbol", Class::Interactive),
            ("lsp/textDocument/formatting", Class::Interactive),
            ("lsp/textDocument/codeAction", Class::Deliberate),
            ("lsp/diagnostics-after-edit", Class::Background),
            ("lsp/cold-first-diagnostics", Class::Cold),
        ] {
            assert_eq!(Class::of(bench), class, "{bench}");
        }
    }

    #[test]
    fn the_jsonl_carries_the_d5_members_and_nothing_float_formatted_oddly() {
        let run = Run {
            series: vec![series("lsp/textDocument/hover", &[40.0, 41.0, 42.0])],
            under_sampled: true,
        };
        let text = to_jsonl(&run, "deadbeef", &json!({"profile": "minimal"}));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "p50, p95, mad");
        let v: Value = serde_json::from_str(lines[0]).unwrap();
        for key in [
            "bench", "track", "metric", "value", "unit", "commit", "config",
        ] {
            assert!(v.get(key).is_some(), "missing {key} in {v}");
        }
        assert_eq!(v["track"], "compile");
        assert_eq!(v["unit"], "ms");
        assert_eq!(v["config"]["class"], "instant");
        assert_eq!(v["config"]["budget_p95_ms"], 200.0);
        assert_eq!(v["config"]["runs"], 3);
    }

    #[test]
    fn the_cold_budget_is_the_five_seconds_fackr_blocks_for() {
        assert_eq!(Class::Cold.budget_ms(), 5000.0);
    }
}
