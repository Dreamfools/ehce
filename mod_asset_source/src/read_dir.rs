use crate::{FsDriver, KNOWN_EXTENSIONS, ModFs, ignore_matches, normalize_with_parent};
use async_fs::{ReadDir, read_dir};
use bevy_asset::io::AssetReaderError;
use bevy_tasks::futures_lite::FutureExt as _;
use futures_core::Stream as _;
use ignore::gitignore;
use include_dir::DirEntry;
use std::ops::Deref as _;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tracing::error;

impl ModFs {
    pub(crate) async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<ModFsDirStream, AssetReaderError> {
        self.check_ignore(path, true)?;
        match self.driver.deref() {
            FsDriver::Embedded { dir } => {
                let Some(dir) = dir.get_dir(path) else {
                    return Err(AssetReaderError::NotFound(path.to_owned()));
                };
                let entries = dir
                    .entries()
                    .iter()
                    .map(|e| {
                        (
                            e.path().to_owned(),
                            match e {
                                DirEntry::Dir(_) => FileKind::Directory,
                                DirEntry::File(_) => FileKind::File,
                            },
                        )
                    })
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
                    kind: ModFsDirStreamKind::AsyncFs {
                        root: root.to_owned(),
                        read_dir: dir,
                        cur_task: None,
                    },
                    ignore: self.ignore.clone(),
                })
            }
        }
    }
}

pub(crate) struct ModFsDirStream {
    kind: ModFsDirStreamKind,
    ignore: Arc<gitignore::Gitignore>,
}

enum ModFsDirStreamKind {
    Entries {
        entries: Vec<(PathBuf, FileKind)>,
    },
    AsyncFs {
        root: PathBuf,
        read_dir: ReadDir,
        cur_task: Option<Pin<Box<dyn Future<Output = Output> + Send>>>,
    },
}

#[derive(Debug, Eq, PartialEq)]
enum FileKind {
    File,
    Directory,
}

enum Output {
    Entry(PathBuf, FileKind),
    Done,
    Error,
}

async fn resolve_file_type(path: PathBuf, entry: async_fs::DirEntry) -> Output {
    // Skip on error
    let Ok(kind) = entry.file_type().await else {
        return Output::Error;
    };
    if kind.is_dir() {
        Output::Entry(path, FileKind::Directory)
    } else if kind.is_file() {
        Output::Entry(path, FileKind::File)
    } else {
        // Skip symlinks and other non-file, non-directory entries
        Output::Error
    }
}

impl ModFsDirStreamKind {
    fn poll_one(&mut self, cx: &mut Context<'_>) -> Poll<Output> {
        match self {
            ModFsDirStreamKind::Entries { entries } => {
                if let Some((path, kind)) = entries.pop() {
                    Poll::Ready(Output::Entry(path, kind))
                } else {
                    Poll::Ready(Output::Done)
                }
            }
            ModFsDirStreamKind::AsyncFs {
                root,
                read_dir,
                cur_task,
            } => {
                if let Some(fut) = cur_task
                    && let Poll::Ready(res) = fut.poll(cx)
                {
                    *cur_task = None;
                    return Poll::Ready(res);
                }
                match Pin::new(read_dir).poll_next(cx) {
                    Poll::Ready(Some(Ok(entry))) => {
                        let path = match entry.path().strip_prefix(root) {
                            Ok(path) => path.to_owned(),
                            Err(err) => {
                                error!(err = ?err, "Failed to strip prefix from path, skipping entry");
                                // This should never happen, but if it does, skip this entry
                                return Poll::Ready(Output::Error);
                            }
                        };
                        let mut fut = resolve_file_type(path, entry).boxed();
                        if let Poll::Ready(res) = fut.poll(cx) {
                            return Poll::Ready(res);
                        }
                        *cur_task = Some(fut);
                        Poll::Pending
                    }
                    Poll::Ready(Some(Err(_))) => Poll::Ready(Output::Error),
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
            let (entry, kind) = match stream.kind.poll_one(cx) {
                Poll::Ready(Output::Entry(entry, kind)) => (entry, kind),
                Poll::Ready(Output::Done) => return Poll::Ready(None),
                Poll::Ready(Output::Error) => continue, // skip entries that can't be read
                Poll::Pending => return Poll::Pending,
            };

            // Check for ignores filed/directories
            if ignore_matches(&stream.ignore, &entry, kind == FileKind::Directory) {
                continue;
            }

            if let Some(ext) = entry.extension().and_then(|e| e.to_str()) {
                let lower_ext = ext.to_ascii_lowercase();
                if !KNOWN_EXTENSIONS.contains(&lower_ext) {
                    continue;
                }
            }
            if entry.extension().is_none() {
                // If the entry has no extension, only include it if it's a directory
                if kind != FileKind::Directory {
                    continue;
                }
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
            ModFsDirStreamKind::AsyncFs { read_dir, .. } => read_dir.size_hint(),
        }
    }
}
