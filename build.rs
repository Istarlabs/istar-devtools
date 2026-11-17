// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{collections::HashMap, path::PathBuf};

fn git_commit_id() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn main() {
    // Re-run when the checked-out commit changes (HEAD moves, or the branch ref it points to).
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-changed=.git/packed-refs");
    println!("cargo:rustc-env=GIT_COMMIT_ID={}", git_commit_id());

    // Trigger rebuild when any translation file changes.
    for (lang, file) in [
        ("de", "ui/lang/de/LC_MESSAGES/istar-devtools.po"),
        ("es", "ui/lang/es/LC_MESSAGES/istar-devtools.po"),
        ("ja", "ui/lang/ja/LC_MESSAGES/istar-devtools.po"),
    ] {
        let _ = lang;
        println!("cargo:rerun-if-changed={file}");
    }

    let library = HashMap::from([("lucide".to_string(), PathBuf::from(lucide_slint::lib()))]);
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(library)
        .with_bundled_translations("ui/lang/")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    println!(
        "cargo:warning=lucide path: {}",
        lucide_slint::lib().to_string()
    );
    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}
