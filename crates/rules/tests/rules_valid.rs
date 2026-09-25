mod common;

use common::*;
use fazasanj_model::{CleanupMethod, SafetyLevel};
use fazasanj_rules::{MatchKind, COMMAND_ALLOW_LIST};

#[test]
fn embedded_rules_load_and_validate() {
    let r = rules();
    assert!(r.validate().is_ok(), "{:?}", r.validate());
    assert!(r.len() >= 60, "only {} rules", r.len());
}

#[test]
fn ids_unique_and_texts_present() {
    let r = rules();
    let mut seen = std::collections::HashSet::new();
    for rule in r.rules() {
        assert!(seen.insert(rule.id.clone()), "duplicate {}", rule.id);
        for t in [&rule.title, &rule.why_big, &rule.if_deleted] {
            assert!(!t.fa.trim().is_empty() && !t.en.trim().is_empty(), "{}", rule.id);
        }
        if rule.permanent_ok {
            assert_eq!(rule.safety, SafetyLevel::Safe, "{}", rule.id);
        }
        if rule.method == CleanupMethod::Command {
            let c = rule.command.as_ref().map(|c| c.program.as_str()).unwrap_or("");
            assert!(COMMAND_ALLOW_LIST.contains(&c), "{}", rule.id);
        }
    }
}

#[test]
fn explanation_comes_from_knowledge_base() {
    let r = rules();
    let i = r.by_id("user-temp").unwrap();
    let e = r.explanation(i).unwrap();
    assert_eq!(e.rule_id, "user-temp");
    assert_eq!(e.source, fazasanj_model::ExplanationSource::KnowledgeBase);
    assert!(e.confidence.is_none());
    assert!(r.explanation(u32::MAX).is_none());
}

#[test]
fn virtual_disks_are_never_deleted() {
    let r = rules();
    for rule in r.rules() {
        let vhd = rule.file_patterns.iter().any(|p| p.to_ascii_lowercase().contains(".vhd"));
        if vhd {
            assert!(
                matches!(rule.method, CleanupMethod::CompactVhdx | CleanupMethod::ManualOnly),
                "{} must compact",
                rule.id
            );
        }
    }
}

#[test]
fn persian_text_uses_zwnj() {
    let r = rules();
    // Prefixes and suffixes that must be joined with a ZWNJ, never a plain space.
    let loose = ["می", "نمی", "ها", "های", "هایی", "تر", "ترین", "ای"];
    for rule in r.rules() {
        let mut texts = vec![&rule.title.fa, &rule.why_big.fa, &rule.if_deleted.fa];
        if let Some(i) = &rule.instructions {
            texts.push(&i.fa);
        }
        for t in texts {
            for w in t.split(|c: char| c.is_whitespace() || "،؛.:()«»".contains(c)) {
                assert!(!loose.contains(&w), "{}: loose '{w}' in: {t}", rule.id);
            }
            assert!(!t.contains('\u{064A}') && !t.contains('\u{0643}'), "{}: Arabic yeh/kaf in: {t}", rule.id);
        }
    }
}

#[test]
fn english_text_avoids_filler_words() {
    let r = rules();
    let banned = ["comprehensive", "robust", "seamless", "leverage", "enhance", "delve", "streamline", "elevate"];
    for rule in r.rules() {
        let mut texts = vec![&rule.title.en, &rule.why_big.en, &rule.if_deleted.en];
        if let Some(i) = &rule.instructions {
            texts.push(&i.en);
        }
        for t in texts {
            let l = t.to_lowercase();
            for b in banned {
                assert!(!l.contains(b), "{}: '{b}'", rule.id);
            }
        }
    }
}

#[test]
fn every_example_matches_its_own_rule_on_any_drive() {
    let r = rules();
    for drive in ["C:", "T:", "D:"] {
        for (i, path, is_dir, modified) in example_paths(&r, drive) {
            let got = r.match_path(&path, is_dir, modified, NOW);
            assert_eq!(got.map(|g| id(&r, g)), Some(id(&r, i)), "example {path}");
        }
    }
}

#[test]
fn no_example_is_hidden_by_a_parent_rule() {
    let r = rules();
    for (i, path, is_dir, modified) in example_paths(&r, "T:") {
        let g = governing(&r, &path, is_dir, modified);
        assert_eq!(g.as_ref().map(|(g, _)| id(&r, *g)), Some(id(&r, i)), "{path} is shadowed by {g:?}");
    }
}

#[test]
fn file_rules_only_match_files() {
    let r = rules();
    for (i, path, is_dir, modified) in example_paths(&r, "T:") {
        let rule = r.get(i).unwrap();
        assert_eq!(rule.match_kind == MatchKind::File, !is_dir);
        let other = r.match_path(&path, !is_dir, modified, NOW);
        assert_ne!(other, Some(i), "{path} matched with the wrong kind");
    }
}
