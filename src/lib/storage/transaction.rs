use std::collections::VecDeque;
use std::path::PathBuf;

use itertools::Itertools;

use super::types::*;
use super::StorageProvider;

#[derive(Debug)]
pub enum TxnAction {
    Put(PathBuf, ByteStream),
    Delete(PathBuf),
    Barrier,
}

impl TxnAction {
    pub async fn execute<T: StorageProvider>(self, target: &T) -> Result<()> {
        match self {
            TxnAction::Put(key, data) => target.put_file(&key, data).await?,
            TxnAction::Delete(key) => target.delete_file(&key).await?,
            TxnAction::Barrier => panic!("barrier can't be executed"),
        }
        Ok(())
    }
}

// NOTE
// There's no rollback support now.
// Also, atomicity can't be ensured because S3 doesn't support atomic move operation.
#[derive(Debug, Default)]
pub struct Txn {
    seq: VecDeque<TxnAction>,
}

impl Txn {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn add(&mut self, action: TxnAction) {
        self.seq.push_back(action)
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
            if let TxnAction::Barrier = action {
                Txn::join_commit(&mut staging, target).await?
            } else {
                staging.push(action)
            }
        }
        if !staging.is_empty() {
            Txn::join_commit(&mut staging, target).await?
        }
        Ok(())
    }
}
