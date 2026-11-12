//! The account logins begun at the page, while they wait.

use std::collections::HashMap;

use serde::Serialize;
use tokio::sync::Mutex;

/// The seconds a login is remembered for after its code expired:
/// long enough for a page that is still open to read how it ended.
const REMEMBERED: i64 = 60 * 60;

/// How a login stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Standing {
    /// Nobody has decided at the issuer.
    Waiting,
    /// Approved: the account, `<issuer>.<id>`, is kept.
    SignedIn {
        /// The account.
        account: String,
    },
    /// Denied, expired, refused, or not kept: why.
    Failed {
        /// Why.
        error: String,
    },
}

/// One login begun at the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Login {
    /// The issuer it is at, by name.
    pub issuer: &'static str,
    /// The code the person enters at the issuer's page.
    pub user_code: String,
    /// The issuer's page.
    pub verification_uri: String,
    /// The page with the code in it, where the issuer gives one.
    pub verification_uri_complete: Option<String>,
    /// When the code expires, in seconds since the Unix epoch.
    pub expires: i64,
    /// How it stands.
    #[serde(flatten)]
    pub standing: Standing,
}

/// The logins held, and the id the next one is given.
#[derive(Default)]
struct Held {
    next: u64,
    logins: HashMap<u64, Login>,
}

/// The logins the page began, in memory: a login is a few minutes
/// long, and one a restart forgets is begun again. The rules:
///
/// 1. **An id each**, counted up from one, which the page asks after
///    it by. It says nothing of the login: the device code, which
///    would, is held by the task that waits and by nothing else.
/// 2. **Settled once**, by the task that waited: signed in, with the
///    account kept, or failed, with why.
/// 3. **Forgotten** [`REMEMBERED`] seconds past its code's expiry,
///    when the next is begun.
#[derive(Default)]
pub struct Logins {
    held: Mutex<Held>,
}

impl Logins {
    /// Hold `login`, begun at `now`: its id.
    pub async fn begin(&self, login: Login, now: i64) -> u64 {
        let mut held = self.held.lock().await;
        // Rule 3.
        held.logins.retain(|_, login| login.expires.saturating_add(REMEMBERED) > now);
        held.next += 1;
        let id = held.next;
        held.logins.insert(id, login);
        id
    }

    /// The login `id`, as it stands.
    pub async fn get(&self, id: u64) -> Option<Login> {
        self.held.lock().await.logins.get(&id).cloned()
    }

    /// The login `id` ended as `standing` — rule 2. One already
    /// forgotten is nobody's to tell.
    pub async fn settle(&self, id: u64, standing: Standing) {
        if let Some(login) = self.held.lock().await.logins.get_mut(&id) {
            login.standing = standing;
        }
    }
}
