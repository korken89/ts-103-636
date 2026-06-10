//! Snapshot code generator for `src/mac/messages/generated/`.
//!
//! Modes:
//! * default: regenerate the committed files (then rustfmt them).
//! * `--check`: regenerate into a temp dir and fail on any
//!   difference against the committed files (drift guard for CI).

mod defs;
mod emit;
mod figure;
mod ir;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("codegen/ sits in the repo root")
        .to_path_buf()
}

/// (path relative to repo root, content) for every generated file.
fn generate() -> Vec<(PathBuf, String)> {
    let defs = defs::all();
    let mut files = vec![(
        PathBuf::from("src/mac/messages/generated.rs"),
        emit::emit_index(&defs),
    )];
    for def in &defs {
        files.push((
            PathBuf::from(format!("src/mac/messages/generated/{}.rs", def.module)),
            emit::emit_message(def),
        ));
    }
    files
}

fn rustfmt(paths: &[PathBuf]) {
    let status = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .args(paths)
        .status()
        .expect("failed to run rustfmt");
    assert!(status.success(), "rustfmt failed");
}

fn main() {
    let check = std::env::args().any(|a| a == "--check");
    let root = repo_root();
    let files = generate();

    // Write everything into a temp tree and rustfmt it there, so both
    // modes compare/install identical post-format output.
    let tmp = std::env::temp_dir().join("ts-103-636-codegen");
    let _ = fs::remove_dir_all(&tmp);
    let mut tmp_paths = Vec::new();
    for (rel, content) in &files {
        let p = tmp.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, content).unwrap();
        tmp_paths.push(p);
    }
    rustfmt(&tmp_paths);

    // The set of committed generated files must match exactly.
    let gen_dir = root.join("src/mac/messages/generated");
    let expected: Vec<PathBuf> = files.iter().map(|(rel, _)| root.join(rel)).collect();
    let mut stale: Vec<PathBuf> = Vec::new();
    if gen_dir.is_dir() {
        for entry in fs::read_dir(&gen_dir).unwrap() {
            let p = entry.unwrap().path();
            if p.extension().is_some_and(|e| e == "rs") && !expected.contains(&p) {
                stale.push(p);
            }
        }
    }

    if check {
        let mut bad = Vec::new();
        for (rel, _) in &files {
            let want = fs::read_to_string(tmp.join(rel)).unwrap();
            match fs::read_to_string(root.join(rel)) {
                Ok(have) if have == want => {}
                _ => bad.push(rel.clone()),
            }
        }
        bad.extend(stale.iter().map(|p| p.strip_prefix(&root).unwrap().into()));
        if bad.is_empty() {
            println!("codegen: {} files up to date", files.len());
        } else {
            eprintln!("codegen: out of date (run `make codegen` and commit):");
            for p in bad {
                eprintln!("  {}", p.display());
            }
            exit(1);
        }
    } else {
        for (rel, _) in &files {
            let target = root.join(rel);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(tmp.join(rel), &target).unwrap();
        }
        for p in stale {
            fs::remove_file(&p).unwrap();
            println!("codegen: removed stale {}", p.display());
        }
        println!("codegen: wrote {} files", files.len());
    }
}
