//! A stored volume's image under a scratch file: reads from the
//! image, writes into the scratch, block by block, for a serve that
//! starts from the image and keeps nothing.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use fstool::block::{BlockDevice, FileBackend};

use super::Lease;

/// One block of the overlay: 4 KiB, ext4's own block size.
const BLOCK: u64 = 4096;

/// A copy-on-write block device for one ephemeral serve.
///
/// The image is opened read-only and never written; the scratch is
/// a sparse file holding every written block at the block's own
/// offset in the image; one bit per block says which of the two holds
/// it — 32 MiB of bits for a 1 TiB image. A read is served block by
/// block from whichever holds each; a write lands in the scratch,
/// merged over the image's block where it covers only part of one
/// not yet written. The blocks written are counted against the cap
/// the serve was given, `overlay_disk`, and a write that needs a
/// block past the cap is refused whole with nothing written, the
/// blocks before it intact. The image's length is the device's, fixed
/// at open: an image never changes length inside a serve, since a
/// resize takes the exclusive hold. The journal's replay writes
/// through here like anything else, and counts.
///
/// The image is never synced: `FileBackend::sync` fsyncs a read-only
/// handle too, which the host may refuse. The lease lives here, so
/// the bytes go back to the cap when the device is dropped.
pub struct Overlay {
    base: FileBackend,
    scratch: File,
    written: Vec<u64>,
    /// How many bits of `written` are set.
    blocks: u64,
    /// The most `blocks` may reach.
    cap: u64,
    size: u64,
    position: u64,
    /// Held for its drop: the bytes go back to the cap with the
    /// device.
    _lease: Lease,
}

impl Overlay {
    /// Over the image at `image`, read-only, with `scratch` fresh and
    /// empty and `overlay_disk` bytes of it usable.
    pub fn open(image: &Path, scratch: File, overlay_disk: u64, lease: Lease) -> Result<Self, fstool::Error> {
        let base = FileBackend::open_read_only(image)?;
        let size = base.total_size();
        let count = size.div_ceil(BLOCK);
        Ok(Overlay {
            base,
            scratch,
            written: vec![0; usize::try_from(count.div_ceil(64)).map_err(|_| too_large(size))?],
            blocks: 0,
            cap: overlay_disk / BLOCK,
            size,
            position: 0,
            _lease: lease,
        })
    }

    fn is_written(&self, block: u64) -> bool {
        self.written[(block / 64) as usize] & (1 << (block % 64)) != 0
    }

    fn mark(&mut self, block: u64) {
        self.written[(block / 64) as usize] |= 1 << (block % 64);
        self.blocks += 1;
    }

    /// The length of `block`: `BLOCK`, or less for the image's last.
    fn block_len(&self, block: u64) -> usize {
        (self.size - block * BLOCK).min(BLOCK) as usize
    }

    fn bounds(&self, offset: u64, len: u64) -> Result<(), fstool::Error> {
        if offset.checked_add(len).is_none_or(|end| end > self.size) {
            return Err(fstool::Error::OutOfBounds {
                offset,
                len,
                size: self.size,
            });
        }
        Ok(())
    }

    /// What a write answers when a new block would pass the cap.
    fn full(&self) -> fstool::Error {
        fstool::Error::Io(io::Error::new(
            io::ErrorKind::StorageFull,
            format!("the serve's overlay disk of {} bytes is full", self.cap * BLOCK),
        ))
    }
}

/// An image too long for the bitmap to be addressed.
fn too_large(size: u64) -> fstool::Error {
    fstool::Error::Io(io::Error::new(io::ErrorKind::Unsupported, format!("an image of {size} bytes cannot be overlaid")))
}

impl BlockDevice for Overlay {
    fn block_size(&self) -> u32 {
        self.base.block_size()
    }

    fn total_size(&self) -> u64 {
        self.size
    }

    fn sync(&mut self) -> Result<(), fstool::Error> {
        self.scratch.sync_data()?;
        Ok(())
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), fstool::Error> {
        self.bounds(offset, buf.len() as u64)?;
        let mut done = 0;
        while done < buf.len() {
            let at = offset + done as u64;
            let block = at / BLOCK;
            let within = (at % BLOCK) as usize;
            let take = (BLOCK as usize - within).min(buf.len() - done);
            if self.is_written(block) {
                self.scratch.seek(SeekFrom::Start(at))?;
                self.scratch.read_exact(&mut buf[done..done + take])?;
            } else {
                self.base.read_at(at, &mut buf[done..done + take])?;
            }
            done += take;
        }
        Ok(())
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), fstool::Error> {
        self.bounds(offset, buf.len() as u64)?;
        if buf.is_empty() {
            return Ok(());
        }
        let first = offset / BLOCK;
        let last = (offset + buf.len() as u64 - 1) / BLOCK;
        let new = (first..=last).filter(|block| !self.is_written(*block)).count() as u64;
        if self.blocks + new > self.cap {
            return Err(self.full());
        }
        let mut done = 0;
        while done < buf.len() {
            let at = offset + done as u64;
            let block = at / BLOCK;
            let within = (at % BLOCK) as usize;
            let len = self.block_len(block);
            let take = (len - within).min(buf.len() - done);
            if self.is_written(block) || (within == 0 && take == len) {
                self.scratch.seek(SeekFrom::Start(at))?;
                self.scratch.write_all(&buf[done..done + take])?;
            } else {
                let mut whole = vec![0u8; len];
                self.base.read_at(block * BLOCK, &mut whole)?;
                whole[within..within + take].copy_from_slice(&buf[done..done + take]);
                self.scratch.seek(SeekFrom::Start(block * BLOCK))?;
                self.scratch.write_all(&whole)?;
            }
            if !self.is_written(block) {
                self.mark(block);
            }
            done += take;
        }
        Ok(())
    }
}

impl Read for Overlay {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = (self.size.saturating_sub(self.position)).min(buf.len() as u64) as usize;
        if n == 0 {
            return Ok(0);
        }
        self.read_at(self.position, &mut buf[..n]).map_err(io::Error::other)?;
        self.position += n as u64;
        Ok(n)
    }
}

impl Write for Overlay {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = (self.size.saturating_sub(self.position)).min(buf.len() as u64) as usize;
        if n == 0 && !buf.is_empty() {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "write past the end of the image"));
        }
        self.write_at(self.position, &buf[..n]).map_err(io::Error::other)?;
        self.position += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.scratch.flush()
    }
}

impl Seek for Overlay {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(at) => Some(at),
            SeekFrom::End(delta) => self.size.checked_add_signed(delta),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
        };
        let target = target.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek before the start of the image"))?;
        self.position = target;
        Ok(target)
    }
}
