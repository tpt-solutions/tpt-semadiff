//! Times both matching engines on real files.
//!
//! For each file, the "new" side is the file with its first `fn` renamed, so every run
//! has one known edit. Usage:
//!
//!     cargo run --release -p tpt-semadiff-core --example bench -- path/to/file.rs ...

use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

use tpt_semadiff_core::diff::{diff_files, Engine, Options};
use tpt_semadiff_core::parse::SourceLang;
use tpt_semadiff_core::tree::Tree;

fn main() {
    println!(
        "{:<48} {:>7} {:>9} {:>12} {:>12} {:>8}",
        "file", "lines", "nodes", "chawathe", "zhang-shasha", "ops"
    );
    for arg in env::args().skip(1) {
        let path = Path::new(&arg);
        let Some(lang) = SourceLang::from_path(path) else {
            eprintln!("skipping {arg}: unknown language");
            continue;
        };
        let old = fs::read_to_string(path).expect("read file");
        let new = old.replacen("fn ", "fn renamed_", 1);
        let lines = old.lines().count();

        let nodes = tpt_semadiff_core::normalize::normalize(
            &old,
            lang,
            tpt_semadiff_core::normalize::TriviaPolicy::Keep,
        )
        .map(|n| n.tree().node_count())
        .unwrap_or(0);

        let chawathe =
            time(|| diff_files(&old, &new, lang, options(Engine::Chawathe)).map(|d| d.ops.len()));
        let zhang = time(|| {
            diff_files(&old, &new, lang, options(Engine::ZhangShasha)).map(|d| d.ops.len())
        });

        let name = path.file_name().unwrap_or_default().to_string_lossy();
        println!(
            "{:<48} {:>7} {:>9} {:>10.1}ms {:>10.1}ms {:>8}",
            truncate(&name, 48),
            lines,
            nodes,
            chawathe.0,
            zhang.0,
            chawathe
                .1
                .map_or_else(|_| "err".to_owned(), |n| n.to_string()),
        );
    }
}

fn options(engine: Engine) -> Options {
    Options {
        engine,
        ignore_formatting: false,
    }
}

/// Runs `f` and returns (milliseconds, its result).
fn time<F>(f: F) -> (f64, Result<usize, tpt_semadiff_core::parse::ParseError>)
where
    F: FnOnce() -> Result<usize, tpt_semadiff_core::parse::ParseError>,
{
    let start = Instant::now();
    let result = f();
    (start.elapsed().as_secs_f64() * 1000.0, result)
}

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_owned()
    } else {
        let tail: String = s.chars().skip(s.chars().count() - (width - 1)).collect();
        format!("…{tail}")
    }
}
