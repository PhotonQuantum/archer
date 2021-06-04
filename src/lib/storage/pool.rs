use std::path::PathBuf;
use std::sync::Mutex;

use crate::storage::StorageProvider;
use crate::storage::transaction::{Txn, TxnAction};

use super::types::*;

pub struct PackagePool<T: StorageProvider> {
    remote: T,
    // remote storage
    local: PathBuf,
    // local cache base
    remote_map: Mutex<MetaKeyMap>,
    // meta->key
    local_map: Mutex<MetaKeyMap>,
    // meta->filename
    stage_map: MetaKeyMap,  // meta->path
}

impl<T: StorageProvider> PackagePool<T> {
    pub fn new(remote: T, local: PathBuf) -> Self {
        Self {
            remote,
            local,
            remote_map: Mutex::new(Default::default()),
            local_map: Mutex::new(Default::default()),
            stage_map: Default::default()
        }
    }

    // generate & commit transaction to remote, and clear stage area
    pub async fn commit(&mut self) -> Result<()> {
        let mut txn = Txn::new();
        // locking remote and local maps, preventing inconsistency when getting file
        let mut remote_map = self.remote_map.lock().unwrap();
        let mut local_map = self.local_map.lock().unwrap();

        for (meta, path) in &self.stage_map {
            let unit = LocalPackageUnit::new(meta, path);
            // update remote & local maps
            let key = PathBuf::from(unit.canonicalize_filename());
            remote_map.insert(meta.clone(), key.clone());
            local_map.insert(meta.clone(), key.clone());

            // put package transaction
            txn.add(TxnAction::Put(
                key.clone(),
                ByteStream::from_path(path)?,
            ));

            // pre-cache package
            // file will be copied into dest before txn is committed
            // this is safe because we locked local_map
            tokio::fs::copy(path, self.local.join(key)).await?;
        }

        // ensure all packages are saved
        txn.add(TxnAction::Barrier);

        // generate & put lock file
        txn.add(TxnAction::Delete(PathBuf::from("index.lock")));
        txn.add(TxnAction::Barrier);    // ensure order (s3 doesn't support atomic renaming, so...)
        let new_lock_file = LockFile::from(&*remote_map);
        let lockfile_data = serde_json::to_vec(&new_lock_file)?;
        txn.add(TxnAction::Put(PathBuf::from("index.lock"), ByteStream::from(lockfile_data)));

        // commit transaction
        txn.commit(&self.remote).await?;

        // update stage map
        self.stage_map.clear();

        Ok(())
    }

    // stage built package
    pub fn stage(&mut self, unit: LocalPackageUnit) {
        self.stage_map.insert(unit.meta, unit.path);
    }

    // get package path (first from stage, then local cache, then remote)
    pub async fn get(&mut self, meta: &PackageMeta) -> Result<Option<PathBuf>> {
        if let Some(path) = self.stage_map.get(meta) {
            // exists in staged area
            return Ok(Some(path.clone()));
        }

        let maybe_local_filename = self.local_map.lock().unwrap().get(meta).cloned();
        if let Some(filename) = maybe_local_filename {
            // exists in local cache
            return Ok(Some(self.local.join(filename)))
        }

        let maybe_remote_key = self.remote_map.lock().unwrap().get(meta).cloned();
        return if let Some(key) = maybe_remote_key {
            // optimistic lock: first try to download
            let data = self.remote.get_file(&key).await?;
            let mut local_map = self.local_map.lock().unwrap();
            if let Some(filename) = local_map.get(meta) {
                // conflict, take the previously downloaded file
                Ok(Some(self.local.join(filename)))
            } else {
                // save the file into local cache and return its path
                // NOTE
                // assume that remote file is at root directory
                let local_path = self.local.join(&key);  // take its remote key as cache name
                data.into_file(&local_path).await?;

                local_map.insert(meta.clone(), key);   // update local map

                Ok(Some(local_path))
            }
        } else {
            Ok(None)
        }
    }
}
