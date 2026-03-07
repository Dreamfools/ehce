use std::ops::Deref as _;
use crate::{ModFs, FsDriver, normalize_with_parent};
use async_fs::{ReadDir, read_dir};
use bevy_asset::io::{AssetReaderError};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use futures_core::Stream as _;

impl ModFs {
    pub(crate) async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<ModFsDirStream, AssetReaderError> {
        self.check_ignore(path)?;
        match self.driver.deref() {
            FsDriver::Embedded { dir } => {
                let Some(dir) = dir.get_dir(path) else {
                    return Err(AssetReaderError::NotFound(path.to_owned()));
                };
                let entries = dir
                    .entries()
                    .iter()
                    .map(|e| e.path().to_owned())
                    .collect::<Vec<_>>();
                Ok(ModFsDirStream {
                    kind: ModFsDirStreamKind::Entries { entries },
                    ignore: self.ignore.clone(),
                })
            }
            FsDriver::FileSystem { root } => {
                let path = normalize_with_parent(root, path)?;
                let dir = match read_dir(&path).await {
                    Ok(dir) => dir,
                    Err(err) => {
                        return if err.kind() == std::io::ErrorKind::NotFound {
                            Err(AssetReaderError::NotFound(path.to_owned()))
                        } else {
                            Err(err.into())
                        };
                    }
                };

                Ok(ModFsDirStream {
                    kind: ModFsDirStreamKind::AsyncFs { read_dir: dir },
                    ignore: self.ignore.clone(),
                })
            }
        }
    }
}

pub(crate) struct ModFsDirStream {
    kind: ModFsDirStreamKind,
    ignore: Arc<globset::GlobSet>,
}
enum ModFsDirStreamKind {
    Entries { entries: Vec<PathBuf> },
    AsyncFs { read_dir: ReadDir },
}

enum Output {
    Entry(PathBuf),
    Done,
    Error,
}

impl ModFsDirStreamKind {
    fn poll_one(&mut self, cx: &mut Context<'_>) -> Poll<Output> {
        match self {
            ModFsDirStreamKind::Entries { entries } => {
                if let Some(entry) = entries.pop() {
                    Poll::Ready(Output::Entry(entry))
                } else {
                    Poll::Ready(Output::Done)
                }
            }
            ModFsDirStreamKind::AsyncFs { read_dir } => {
                match Pin::new(read_dir).poll_next(cx) {
                    Poll::Ready(Some(Ok(entry))) => Poll::Ready(Output::Entry(entry.path())),
                    Poll::Ready(Some(Err(_))) => {
                        Poll::Ready(Output::Error)
                    }
                    Poll::Ready(None) => Poll::Ready(Output::Done),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

impl futures_core::Stream for ModFsDirStream {
    type Item = PathBuf;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let stream = self.get_mut();
        loop {
            let entry = match stream.kind.poll_one(cx) {
                Poll::Ready(Output::Entry(entry)) => entry,
                Poll::Ready(Output::Done) => return Poll::Ready(None),
                Poll::Ready(Output::Error) => continue, // skip entries that can't be read
                Poll::Pending => return Poll::Pending,
            };

            // Skip ignored files/directories
            if stream.ignore.is_match(&entry) {
                continue;
            }

            // Ignore .meta files, which are used by Bevy's asset system to store
            // metadata about assets
            if let Some(ext) = entry.extension().and_then(|e| e.to_str())
                && ext.eq_ignore_ascii_case("meta")
            {
                continue;
            }

            return Poll::Ready(Some(entry));
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.kind {
            ModFsDirStreamKind::Entries { entries } => {
                let len = entries.len();
                (len, Some(len))
            }
            ModFsDirStreamKind::AsyncFs { read_dir } => read_dir.size_hint(),
        }
    }
}
