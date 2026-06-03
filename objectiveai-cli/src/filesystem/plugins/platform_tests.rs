use super::Platform;

#[test]
fn platform_serializes_to_snake_pair() {
    let cases = [
        (Platform::LinuxX86_64, "\"linux_x86_64\""),
        (Platform::LinuxAarch64, "\"linux_aarch64\""),
        (Platform::WindowsX86_64, "\"windows_x86_64\""),
        (Platform::WindowsAarch64, "\"windows_aarch64\""),
        (Platform::MacosX86_64, "\"macos_x86_64\""),
        (Platform::MacosAarch64, "\"macos_aarch64\""),
    ];
    for (p, expected) in cases {
        let got = serde_json::to_string(&p).unwrap();
        assert_eq!(got, expected, "serialize {p:?}");
    }
}

#[test]
fn platform_deserializes_from_snake_pair() {
    let cases = [
        ("\"linux_x86_64\"", Platform::LinuxX86_64),
        ("\"linux_aarch64\"", Platform::LinuxAarch64),
        ("\"windows_x86_64\"", Platform::WindowsX86_64),
        ("\"windows_aarch64\"", Platform::WindowsAarch64),
        ("\"macos_x86_64\"", Platform::MacosX86_64),
        ("\"macos_aarch64\"", Platform::MacosAarch64),
    ];
    for (json, expected) in cases {
        let got: Platform = serde_json::from_str(json).unwrap();
        assert_eq!(got, expected, "deserialize {json}");
    }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn platform_current_matches_runtime() {
    let expected = match (
        cfg!(target_os = "linux"),
        cfg!(target_os = "windows"),
        cfg!(target_os = "macos"),
        cfg!(target_arch = "x86_64"),
        cfg!(target_arch = "aarch64"),
    ) {
        (true, _, _, true, _) => Some(Platform::LinuxX86_64),
        (true, _, _, _, true) => Some(Platform::LinuxAarch64),
        (_, true, _, true, _) => Some(Platform::WindowsX86_64),
        (_, true, _, _, true) => Some(Platform::WindowsAarch64),
        (_, _, true, true, _) => Some(Platform::MacosX86_64),
        (_, _, true, _, true) => Some(Platform::MacosAarch64),
        _ => None,
    };
    assert_eq!(Platform::current(), expected);
}

#[test]
#[cfg(all(
    any(target_os = "linux", target_os = "windows", target_os = "macos"),
    any(target_arch = "x86_64", target_arch = "aarch64"),
))]
fn platform_current_some_on_supported_host() {
    assert!(Platform::current().is_some());
}
