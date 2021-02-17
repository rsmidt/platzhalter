use crate::ImageMeta;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn get_from_db(db: &sled::Db, meta: &ImageMeta) -> sled::Result<Option<Vec<u8>>> {
    let mut hash = DefaultHasher::new();
    meta.hash(&mut hash);
    let hash = hash.finish();
    let entry: Option<Vec<u8>> = db.get(hash.to_string())?.map(|e| e.to_vec());
    Ok(entry)
}

pub fn insert(db: &sled::Db, meta: &ImageMeta, bytes: Vec<u8>) -> sled::Result<Option<sled::IVec>> {
    let mut hash = DefaultHasher::new();
    meta.hash(&mut hash);
    let hash = hash.finish();
    db.insert(hash.to_string(), bytes)
}
