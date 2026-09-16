//! Where an image comes from, as the reference podman is handed.

/// The image of a deploy: one of the request's three sources, each
/// resolved to the reference podman is handed and whether it is
/// pulled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A caller-held image, on the provider's own registry:
    /// `127.0.0.1:<port>/<repository>/<name>@<digest>`, pulled over
    /// plain HTTP.
    Client(String),
    /// One of the provider's own: `<name>@<digest>`, in the store
    /// already, pulled from nowhere.
    Server(String),
    /// Wherever the caller said: the reference as given, under a host
    /// the configuration lists, pulled with that host's credential.
    Registry(String),
}

impl Source {
    /// The reference podman is handed.
    pub fn reference(&self) -> &str {
        match self {
            Source::Client(reference) | Source::Server(reference) | Source::Registry(reference) => reference,
        }
    }

    /// Whether the image is pulled before the run.
    pub fn pulled(&self) -> bool {
        !matches!(self, Source::Server(_))
    }

    /// Whether the pull is from the provider's own registry, which
    /// speaks plain HTTP.
    pub fn own(&self) -> bool {
        matches!(self, Source::Client(_))
    }
}

/// Whether `name` is a repository path as the OCI distribution
/// specification has one: one or more segments joined by `/`, each
/// lowercase letters and digits with single `.`, `_`, `__` or `-`
/// runs between them. Anything else — an empty segment, `..`, an
/// uppercase letter, a space — is refused, since the name is put
/// into a URL by concatenation and never normalized.
pub fn name_ok(name: &str) -> bool {
    !name.is_empty() && name.split('/').all(segment_ok)
}

/// One segment of a repository path: names joined by separators,
/// where a name is lowercase letters and digits and a separator is
/// `.`, `_`, `__` or a run of `-`, never first, never last, never
/// two in a row.
fn segment_ok(segment: &str) -> bool {
    let bytes = segment.as_bytes();
    let name = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    let mut index = 0;
    while index < bytes.len() {
        if name(bytes[index]) {
            index += 1;
            continue;
        }
        if index == 0 {
            return false;
        }
        match bytes[index] {
            b'.' => index += 1,
            b'_' => {
                index += 1;
                if bytes.get(index) == Some(&b'_') {
                    index += 1;
                }
            }
            b'-' => {
                while bytes.get(index) == Some(&b'-') {
                    index += 1;
                }
            }
            _ => return false,
        }
        if !bytes.get(index).is_some_and(|&byte| name(byte)) {
            return false;
        }
    }
    !bytes.is_empty()
}

/// The host a reference names, if it names one: its first segment,
/// when that contains a `.` or a `:` or is `localhost` — podman's
/// own reading of a reference. `None` is a reference with no host.
pub fn host_of(reference: &str) -> Option<&str> {
    let first = reference.split('/').next()?;
    if reference.contains('/') && (first.contains('.') || first.contains(':') || first == "localhost") {
        Some(first)
    } else {
        None
    }
}
