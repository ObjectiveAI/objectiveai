//! The four scopes a connection holds at most one of each.

use std::borrow::Borrow;
use std::ops::Deref;
use std::sync::Arc;

use tokio::sync::{Mutex, OwnedMutexGuard};

use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// One connection's four slots — one per whole-connection scope, see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time)
/// — made in [`handle`](super::handle::handle) and never shared with
/// another connection. The rules:
///
/// 1. **One at a time.** A slot is claimed by
///    [`Slot::claim`], and a slot claimed already is not claimed again
///    until its [`Single`] lets it go: the claim is a `try_lock` of
///    the slot's own lock, so two claims never both succeed, whichever
///    tasks make them.
/// 2. **Claimed in the dispatch.** The claim is made where the request
///    is read, before the handler is spawned, so a scope is open from
///    the server reading its request.
/// 3. **Refused, not queued.** A claim that fails hands the scope
///    back, and the dispatch answers it `AlreadyOpen` and the finish;
///    nothing waits for the slot.
/// 4. **Free before the finish.** A [`Single`] is the claim and the
///    scope as one value, and the claim always goes first:
///    [`Single::send_response_finish`] lets the slot go and then sends
///    the finish, and a `Single` dropped unfinished — its handler
///    returned, cancelled, or panicked — drops the claim before the
///    scope, the fields dropping in the order they are declared, so
///    the finish the scope's own drop sends follows it as well.
#[derive(Debug, Default)]
pub struct Singles {
    /// [`volumes::list`](diverge_sdk::provider::endpoints::volumes::list).
    pub volumes_list: Slot,
    /// [`images::list`](diverge_sdk::provider::endpoints::images::list).
    pub images_list: Slot,
    /// [`discover`](diverge_sdk::provider::endpoints::discover).
    pub discover: Slot,
    /// [`daemons::accept`](diverge_sdk::provider::endpoints::daemons::accept).
    pub daemons_accept: Slot,
}

/// One of the four, on one connection: free, or claimed by exactly one
/// [`Single`].
#[derive(Debug, Default)]
pub struct Slot(Arc<Mutex<()>>);

impl Slot {
    /// Claim the slot for `scope` (rule 1): the scope back when it is
    /// claimed already (rule 3).
    pub fn claim(&self, scope: ScopeHandle) -> Result<Single<ScopeHandle>, ScopeHandle> {
        match Arc::clone(&self.0).try_lock_owned() {
            Ok(claim) => Ok(Single { claim, scope }),
            Err(_) => Err(scope),
        }
    }
}

/// A claimed slot and the scope it was claimed for, as one value: the
/// scope reached through it, and finished through it (rule 4). `S` is
/// the scope as the handler holds it — the handle itself, or an
/// [`Arc`] of it where the scope is shared.
#[derive(Debug)]
pub struct Single<S> {
    /// The claim: first, so it drops before the scope.
    claim: OwnedMutexGuard<()>,
    /// The scope.
    scope: S,
}

impl<S: Borrow<ScopeHandle>> Single<S> {
    /// The scope as held, for a handler that shares it.
    pub fn scope(&self) -> &S {
        &self.scope
    }

    /// The same claim over the scope held another way.
    pub fn map<T>(self, held: impl FnOnce(S) -> T) -> Single<T> {
        let Single { claim, scope } = self;
        Single { claim, scope: held(scope) }
    }

    /// Let the slot go, and send the finish (rule 4). Shadows the
    /// scope's own, which would send the finish with the slot held.
    pub async fn send_response_finish(self) {
        let Single { claim, scope } = self;
        drop(claim);
        scope.borrow().send_response_finish().await;
    }
}

impl<S: Borrow<ScopeHandle>> Deref for Single<S> {
    type Target = ScopeHandle;

    fn deref(&self) -> &ScopeHandle {
        self.scope.borrow()
    }
}
