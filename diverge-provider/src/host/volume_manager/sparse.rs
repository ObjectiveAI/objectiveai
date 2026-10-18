//! Making a new image sparse, where the filesystem has to be told.

use std::io;

use tokio::fs::File;

/// Mark `file` sparse, so the length `set_len` gives it is reserved
/// and not taken.
///
/// On NTFS a file is dense unless it is marked otherwise, and
/// extending a dense file allocates every cluster; this sends
/// `FSCTL_SET_SPARSE` through `DeviceIoControl` — a synchronous call
/// on a handle, so it runs on the blocking pool. The handle stays
/// open for the length of the call because the caller holds `file`
/// across it.
#[cfg(windows)]
pub async fn sparse(file: &File) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle as _;
    use std::ptr;

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::FSCTL_SET_SPARSE;

    // Carried as an integer: a raw pointer is not `Send`, and the
    // blocking task needs it to be. The handle is the file's, and the
    // file outlives this call.
    let handle = file.as_raw_handle() as usize;
    tokio::task::spawn_blocking(move || {
        let mut returned = 0u32;
        // SAFETY: the handle is an open file handle for the life of
        // this call, `FSCTL_SET_SPARSE` reads no input buffer and
        // writes no output buffer, and `returned` outlives the call.
        let ok = unsafe {
            DeviceIoControl(
                handle as HANDLE,
                FSCTL_SET_SPARSE,
                ptr::null(),
                0,
                ptr::null_mut(),
                0,
                &mut returned,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    })
    .await
    .map_err(io::Error::other)?
}

/// Nothing to do: on every other host `set_len` past the end of a
/// file leaves a hole, and the image is sparse by being made.
#[cfg(not(windows))]
pub async fn sparse(_file: &File) -> io::Result<()> {
    Ok(())
}
