use super::{WhitelistEntry, check_plugin_whitelist, default_whitelist};

#[test]
fn default_whitelist_passes_objectiveai_owner() {
    let wl = default_whitelist();
    let ok = check_plugin_whitelist(
        "ObjectiveAI",
        "anything",
        "anysha",
        "0.1.0",
        &wl,
    )
    .unwrap();
    assert!(ok);
}

#[test]
fn default_whitelist_rejects_other_owners() {
    let wl = default_whitelist();
    let ok =
        check_plugin_whitelist("evil-org", "anything", "anysha", "0.1.0", &wl)
            .unwrap();
    assert!(!ok);
}

#[test]
fn default_whitelist_is_case_insensitive() {
    let wl = default_whitelist();
    for owner in ["objectiveai", "OBJECTIVEAI", "ObJeCtIvEaI", "ObjectiveAI"] {
        let ok =
            check_plugin_whitelist(owner, "anything", "anysha", "0.1.0", &wl)
                .unwrap();
        assert!(ok, "expected case-insensitive match for {owner:?}");
    }
}

#[test]
fn anchored_pattern_does_not_partial_match() {
    let wl = vec![WhitelistEntry {
        owner: "Object".to_string(),
        repository: ".*".to_string(),
        commit_sha: ".*".to_string(),
        version: ".*".to_string(),
    }];
    let ok =
        check_plugin_whitelist("ObjectAttacker", "x", "x", "x", &wl).unwrap();
    assert!(!ok, "anchored `^Object$` must not match `ObjectAttacker`");
}

#[test]
fn anchored_pattern_does_not_match_prefix() {
    let wl = vec![WhitelistEntry {
        owner: ".*Owner".to_string(),
        repository: ".*".to_string(),
        commit_sha: ".*".to_string(),
        version: ".*".to_string(),
    }];
    let ok =
        check_plugin_whitelist("FakeOwnerExtra", "x", "x", "x", &wl).unwrap();
    assert!(!ok, "anchored `^.*Owner$` must not match `FakeOwnerExtra`");
}

#[test]
fn multiple_entries_or_semantics() {
    let wl = vec![
        WhitelistEntry {
            owner: "ObjectiveAI".to_string(),
            repository: ".*".to_string(),
            commit_sha: ".*".to_string(),
            version: ".*".to_string(),
        },
        WhitelistEntry {
            owner: "TrustedCorp".to_string(),
            repository: ".*".to_string(),
            commit_sha: ".*".to_string(),
            version: ".*".to_string(),
        },
    ];
    let ok = check_plugin_whitelist("TrustedCorp", "any", "any", "any", &wl)
        .unwrap();
    assert!(ok, "any matching entry should allow the install");
}

#[test]
fn empty_whitelist_rejects_everything() {
    let ok = check_plugin_whitelist(
        "ObjectiveAI",
        "anything",
        "anysha",
        "0.1.0",
        &[],
    )
    .unwrap();
    assert!(!ok);
}

#[test]
fn invalid_regex_returns_error() {
    let wl = vec![WhitelistEntry {
        owner: "[invalid".to_string(),
        repository: ".*".to_string(),
        commit_sha: ".*".to_string(),
        version: ".*".to_string(),
    }];
    let result = check_plugin_whitelist("ObjectiveAI", "x", "x", "x", &wl);
    assert!(result.is_err(), "expected regex compile error");
}

#[test]
fn all_four_fields_must_match() {
    let wl = vec![WhitelistEntry {
        owner: "ObjectiveAI".to_string(),
        repository: "approved-repo".to_string(),
        commit_sha: "abc123".to_string(),
        version: "1\\.0\\.0".to_string(),
    }];

    // All match → ok
    assert!(
        check_plugin_whitelist(
            "ObjectiveAI",
            "approved-repo",
            "abc123",
            "1.0.0",
            &wl
        )
        .unwrap()
    );

    // Single mismatch → reject
    assert!(
        !check_plugin_whitelist(
            "ObjectiveAI",
            "approved-repo",
            "abc123",
            "2.0.0",
            &wl
        )
        .unwrap()
    );
    assert!(
        !check_plugin_whitelist(
            "ObjectiveAI",
            "approved-repo",
            "deadbe",
            "1.0.0",
            &wl
        )
        .unwrap()
    );
    assert!(
        !check_plugin_whitelist(
            "ObjectiveAI",
            "other-repo",
            "abc123",
            "1.0.0",
            &wl
        )
        .unwrap()
    );
    assert!(
        !check_plugin_whitelist(
            "Other",
            "approved-repo",
            "abc123",
            "1.0.0",
            &wl
        )
        .unwrap()
    );
}
