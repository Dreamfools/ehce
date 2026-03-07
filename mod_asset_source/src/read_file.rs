use std::io::SeekFrom;
use std::ops::Deref as _;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use bevy_asset::io::{AssetReaderError, Reader, ReaderNotSeekableError, SeekableReader};
use bevy_tasks::futures_lite::{AsyncRead, AsyncSeek};
use crate::{normalize_with_parent, ModFs, FsDriver};

impl ModFs {
    pub(crate) async fn read_file(&self, path: &Path) -> Result<ModFsReader, AssetReaderError> {
        self.check_ignore(path)?;
        match self.driver.deref() {
            FsDriver::Embedded { dir } => {
                let file = dir
                    .get_file(path)
                    .ok_or_else(|| AssetReaderError::NotFound(path.to_owned()))?;
                Ok(ModFsReader::Bytes(file.contents(), 0))
            }
            FsDriver::FileSystem { root } => {
                let file_path = normalize_with_parent(root, path)?;
                let file = async_fs::File::open(file_path).await?;
                Ok(ModFsReader::File(file))
            }
        }
    }
}

pub(crate) enum ModFsReader {
    Bytes(&'static [u8], usize),
    File(async_fs::File),
}

impl AsyncRead for ModFsReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            ModFsReader::Bytes(bytes, bytes_read) => {
                Poll::Ready(Ok(slice_read(bytes, bytes_read, buf)))
            }
            ModFsReader::File(file) => Pin::new(file).poll_read(cx, buf),
        }
    }
}

impl AsyncSeek for ModFsReader {
    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        match self.get_mut() {
            ModFsReader::Bytes(bytes, bytes_read) => {
                Poll::Ready(slice_seek(bytes, bytes_read, pos))
            }
            ModFsReader::File(file) => Pin::new(file).poll_seek(cx, pos),
        }
    }
}

impl Reader for ModFsReader {
    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        Ok(self)
    }
}

/// Performs a read from the `slice` into `buf`.
fn slice_read(slice: &[u8], bytes_read: &mut usize, buf: &mut [u8]) -> usize {
    if *bytes_read >= slice.len() {
        0
    } else {
        let n = std::io::Read::read(&mut &slice[(*bytes_read)..], buf).unwrap();
        *bytes_read += n;
        n
    }
}

/// Performs a "seek" and updates the cursor of `bytes_read`. Returns the new byte position.
fn slice_seek(slice: &[u8], bytes_read: &mut usize, pos: SeekFrom) -> std::io::Result<u64> {
    let make_error = || {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "seek position is out of range",
        ))
    };
    let (origin, offset) = match pos {
        SeekFrom::Current(offset) => (*bytes_read, Ok(offset)),
        SeekFrom::Start(offset) => (0, offset.try_into()),
        SeekFrom::End(offset) => (slice.len(), Ok(offset)),
    };
    let Ok(offset) = offset else {
        return make_error();
    };
    let Ok(origin): Result<i64, _> = origin.try_into() else {
        return make_error();
    };
    let Ok(new_pos) = (origin + offset).try_into() else {
        return make_error();
    };
    *bytes_read = new_pos;
    Ok(new_pos as _)
}
