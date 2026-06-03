use super::{InstallError, build_headers};
use indexmap::IndexMap;
use std::path::PathBuf;

#[test]
fn install_error_display_strings_are_useful() {
    let e = InstallError::ManifestBadStatus {
        code: reqwest::StatusCode::NOT_FOUND,
        url: "https://raw.githubusercontent.com/o/r/HEAD/objectiveai.json"
            .to_string(),
        body: "Not Found".to_string(),
    };
    let s = e.to_string();
    assert!(s.contains("404"), "{s}");
    assert!(s.contains("objectiveai.json"), "{s}");
    assert!(s.contains("Not Found"), "{s}");

    let e = InstallError::BinaryBadStatus {
        code: reqwest::StatusCode::FORBIDDEN,
        url: "https://github.com/o/r/releases/download/v1.0.0/myplugin"
            .to_string(),
    };
    let s = e.to_string();
    assert!(s.contains("403"), "{s}");
    assert!(s.contains("myplugin"), "{s}");

    let e = InstallError::BinaryWrite(
        PathBuf::from("/tmp/oai/plugins/r/plugin"),
        std::io::Error::other("disk full"),
    );
    let s = e.to_string();
    assert!(s.contains("plugin"), "{s}");
    assert!(s.contains("disk full"), "{s}");

    let e = InstallError::InvalidHeaderName {
        name: "bad name!".to_string(),
        reason: "invalid HTTP header name".to_string(),
    };
    let s = e.to_string();
    assert!(s.contains("bad name!"), "{s}");
}

#[test]
fn build_headers_rejects_invalid_name() {
    let mut h = IndexMap::new();
    h.insert("invalid header!".to_string(), "v".to_string());
    let result = build_headers(Some(&h));
    match result {
        Err(InstallError::InvalidHeaderName { name, .. }) => {
            assert_eq!(name, "invalid header!");
        }
        Err(other) => panic!("expected InvalidHeaderName, got {other:?}"),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn build_headers_accepts_valid_pair() {
    let mut h = IndexMap::new();
    h.insert("Authorization".to_string(), "token abc".to_string());
    let map = build_headers(Some(&h)).unwrap();
    assert_eq!(
        map.get("authorization").unwrap().to_str().unwrap(),
        "token abc"
    );
}

#[test]
fn build_headers_none_returns_empty_map() {
    let map = build_headers(None).unwrap();
    assert!(map.is_empty());
}
