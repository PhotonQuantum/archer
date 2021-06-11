use std::collections::VecDeque;
use std::path::PathBuf;

use itertools::Itertools;

use crate::error::StorageError;

use super::types::*;
use super::StorageProvider;
use tokio::io::AsyncReadExt;

// NOTE
// The assertion here doesn't guarantee atomicity because S3 doesn't provide it.
// It can only be used as a naive safety check, and its result can't be trusted
// especially when the file is large.
pub enum TxnAction {
    Put(PathBuf, ByteStream),
    Delete(PathBuf),
    Assertion(PathBuf, Box<dyn Fn(Option<Vec<u8>>) -> Result<()>>),
    Barrier,
}

impl TxnAction {
    pub async fn execute<T: StorageProvider>(self, target: &T) -> Result<()> {
        match self {
            TxnAction::Put(key, data) => target.put_file(&key, data).await?,
            TxnAction::Delete(key) => target.delete_file(&key).await?,
            TxnAction::Barrier => panic!("barrier can't be executed"),
            TxnAction::Assertion(key, func) => {
                let stream = target.get_file(&key).await.map(Some).or_else(|e| {
                    if let StorageError::FileNotExists(_) = e {
                        Ok(None)
                    } else {
                        Err(e)
                    }
                })?;
                let buf = if let Some(mut stream) = stream {
                    let mut buf: Vec<u8> = Vec::new();
                    stream.read_to_end(&mut buf).await?;
                    Some(buf)
                } else {
                    None
                };
                func(buf)?
            }
        }
        Ok(())
    }
}

// NOTE
// There's no rollback support now.
// Also, atomicity can't be ensured because S3 doesn't support atomic move operation.
#[derive(Default)]
pub struct Txn {
    seq: VecDeque<TxnAction>,
}

impl Txn {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn add(&mut self, action: TxnAction) {
        self.seq.push_back(action);
    }
    async fn join_commit<T: StorageProvider>(
        staging: &mut Vec<TxnAction>,
        target: &T,
    ) -> Result<()> {
        let staging_futures = staging
            .drain(..)
            .map(|act: TxnAction| act.execute(target))
            .collect_vec();
        futures::future::try_join_all(staging_futures).await?;
        staging.clear();
        Ok(())
    }
    pub async fn commit<T: StorageProvider>(mut self, target: &T) -> Result<()> {
        let mut staging = vec![];
        while let Some(action) = self.seq.pop_front() {
            match action {
                TxnAction::Assertion(_, _) => {
                    Txn::join_commit(&mut staging, target).await?;
                    action.execute(target).await?;
                }
                TxnAction::Barrier => Txn::join_commit(&mut staging, target).await?,
                _ => staging.push(action),
            }
        }
        if !staging.is_empty() {
            Txn::join_commit(&mut staging, target).await?;
        }
        Ok(())
    }
}
