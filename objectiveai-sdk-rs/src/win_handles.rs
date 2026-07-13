//! Windows handle-inheritance hygiene shared by every subprocess
//! spawn site in this crate.

/// Clear `HANDLE_FLAG_INHERIT` on this process's std handles so a
/// child spawned with `bInheritHandles=TRUE` (any piped-stdio spawn)
/// does not receive duplicated copies of them. Without this, a
/// long-lived child (a detached server, or a resident daemon spawned
/// by a child CLI) keeps the write end of whatever pipe THIS process's
/// stdout/stderr happen to be — so whoever is tailing this process
/// never sees pipe EOF and waits forever after it exits.
///
/// Idempotent and best-effort: a pseudo/closed std handle that can't
/// be updated has nothing to leak. The flag only governs inheritance —
/// our own reads and writes through these handles are unaffected, and
/// `Stdio::inherit` children still work (std re-duplicates the handle
/// inheritable at spawn time).
pub(crate) fn disinherit_std_handles() {
    use std::os::windows::io::AsRawHandle;

    const HANDLE_FLAG_INHERIT: u32 = 0x1;
    // kernel32 is always linked on Windows targets; no #[link] needed.
    unsafe extern "system" {
        fn SetHandleInformation(
            h_object: *mut std::ffi::c_void,
            dw_mask: u32,
            dw_flags: u32,
        ) -> i32;
    }

    for handle in [
        std::io::stdin().as_raw_handle(),
        std::io::stdout().as_raw_handle(),
        std::io::stderr().as_raw_handle(),
    ] {
        if !handle.is_null() {
            unsafe {
                SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0);
            }
        }
    }
}
