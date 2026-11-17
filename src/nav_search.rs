// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! Navigation search and collapsible groups (Slint strings have no `contains`).

use slint::{ComponentHandle, ToSharedString};

use crate::{AppWindow, NavSearch, config};

/// True when every whitespace-separated word of `query` occurs in `haystack`, ignoring case.
pub fn matches(query: &str, haystack: &str) -> bool {
    let haystack = haystack.to_lowercase();
    query
        .split_whitespace()
        .all(|word| haystack.contains(&word.to_lowercase()))
}

/// The collapsed groups as one string, each name wrapped in `|`.
fn join(groups: &[String]) -> String {
    if groups.is_empty() {
        return String::new();
    }
    format!("|{}|", groups.join("|"))
}

fn split(joined: &str) -> Vec<String> {
    joined
        .split('|')
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_collapsed(group: &str, joined: &str) -> bool {
    joined.contains(&format!("|{group}|"))
}

/// Add `group` to the collapsed groups, or remove it when it is already in there.
fn toggled(joined: &str, group: &str) -> Vec<String> {
    let mut groups = split(joined);
    match groups.iter().position(|name| name == group) {
        Some(index) => {
            groups.remove(index);
        }
        None => groups.push(group.to_string()),
    }
    groups
}

pub fn register(app: &AppWindow) {
    let state = app.global::<NavSearch>();
    state.set_collapsed(join(&config::AppConfig::load().collapsed_groups).to_shared_string());
    state.on_matches(|query, haystack| matches(query.as_str(), haystack.as_str()));
    state.on_is_collapsed(|group, joined| is_collapsed(group.as_str(), joined.as_str()));

    let handle = app.as_weak();
    state.on_toggle_group(move |group| {
        let ui = handle.unwrap();
        let state = ui.global::<NavSearch>();
        let groups = toggled(state.get_collapsed().as_str(), group.as_str());
        state.set_collapsed(join(&groups).to_shared_string());

        let mut cfg = config::AppConfig::load();
        cfg.collapsed_groups = groups;
        cfg.save();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_words_in_any_order_and_case() {
        assert!(matches("hash", "Hash Generator Security"));
        assert!(matches("GEN hash", "Hash Generator Security"));
        assert!(matches("  ", "anything"));
        assert!(matches("", "anything"));
        assert!(!matches("hash zip", "Hash Generator Security"));
        assert!(!matches("xyz", "Hash Generator"));
    }

    #[test]
    fn matches_non_ascii_case_insensitively() {
        assert!(matches("ÜBER", "Über uns"));
    }

    #[test]
    fn matches_tags() {
        let haystack = "Converter Data & Formats convert json yaml toml csv xml format";
        assert!(matches("json", haystack));
        assert!(matches("yaml convert", haystack));
        assert!(!matches("jwt", haystack));
    }

    #[test]
    fn collapsed_groups_round_trip() {
        assert_eq!(join(&[]), "");
        let groups = vec!["Security".to_string(), "Data & Formats".to_string()];
        let joined = join(&groups);
        assert_eq!(joined, "|Security|Data & Formats|");
        assert_eq!(split(&joined), groups);
        assert!(split("").is_empty());
    }

    #[test]
    fn collapsed_lookup_needs_the_whole_name() {
        let joined = "|Data & Formats|";
        assert!(is_collapsed("Data & Formats", joined));
        assert!(!is_collapsed("Data", joined));
        assert!(!is_collapsed("Security", ""));
    }

    #[test]
    fn toggling_adds_and_removes() {
        let once = toggled("", "Security");
        assert_eq!(once, ["Security"]);
        let twice = toggled(&join(&once), "Security");
        assert!(twice.is_empty());
        assert_eq!(toggled("|A|", "B"), ["A", "B"]);
    }
}
