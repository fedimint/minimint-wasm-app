#![allow(dead_code)]

#[cfg(test)]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use std::future::Future;

use minimint_api::db::{
    batch::{BatchItem, DbBatch},
    DatabaseError, PrefixIter,
};
use wasm_bindgen::JsCast;

struct WebDb {
    db: rexie::Rexie,
}

impl WebDb {
    pub async fn new(name: &str) -> Result<Self, rexie::Error> {
        Ok(Self {
            db: rexie::Rexie::builder(name)
                .add_object_store(rexie::ObjectStore::new("default"))
                .build()
                .await?,
        })
    }

    pub async fn transaction<R, Fut: Future<Output = Result<R, rexie::Error>>>(
        &self,
        f: impl FnOnce(rexie::Store) -> Fut,
    ) -> Result<R, rexie::Error> {
        let tx = self
            .db
            .transaction(&["default"], rexie::TransactionMode::ReadWrite)?;
        let result = f(tx.store("default")?).await;
        tx.done().await?;
        result
    }
}

impl WebDb {
    async fn raw_insert_entry(
        &self,
        key: &[u8],
        value: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, rexie::Error> {
        let key = js_sys::Uint8Array::from(key);
        let value = js_sys::Uint8Array::from(&value[..]);
        self.transaction(|store| async move {
            let old_value = store.get(&key).await?;
            store.put(&value, Some(&key)).await?;
            if old_value.is_undefined() {
                Ok(None)
            } else {
                Ok(Some(
                    old_value.dyn_into::<js_sys::Uint8Array>().unwrap().to_vec(),
                ))
            }
        })
        .await
    }

    async fn raw_get_value(&self, key: &[u8]) -> Result<Option<Vec<u8>>, rexie::Error> {
        let key = js_sys::Uint8Array::from(key);
        self.transaction(|store| async move {
            let result = store.get(&key).await?;
            if result.is_undefined() {
                Ok(None)
            } else {
                Ok(Some(
                    result.dyn_into::<js_sys::Uint8Array>().unwrap().to_vec(),
                ))
            }
        })
        .await
    }

    async fn raw_remove_entry(&self, key: &[u8]) -> Result<Option<Vec<u8>>, rexie::Error> {
        self.transaction(|store| async move {
            let key = js_sys::Uint8Array::from(key);
            let old_value = store.get(&key).await?;
            store.delete(&key).await?;
            if old_value.is_undefined() {
                Ok(None)
            } else {
                Ok(Some(
                    old_value.dyn_into::<js_sys::Uint8Array>().unwrap().to_vec(),
                ))
            }
        })
        .await
    }

    async fn raw_find_by_prefix(
        &self,
        key_prefix: &[u8],
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, rexie::Error> {
        let mut key_prefix_end = key_prefix.to_vec();
        key_prefix_end.last_mut().map(|b| *b += 1);
        let range = if key_prefix.is_empty() {
            None
        } else {
            let lower = js_sys::Uint8Array::from(key_prefix);
            let upper = js_sys::Uint8Array::from(&key_prefix_end[..]);
            Some(rexie::KeyRange::bound(&lower, &upper, false, true)?)
        };
        self.transaction(
            |store| async move { store.get_all(range.as_ref(), None, None, None).await },
        )
        .await
        .map(|entries| {
            entries
                .into_iter()
                .map(|(key, value)| {
                    (
                        js_sys::Uint8Array::new(&key).to_vec(),
                        value.dyn_into::<js_sys::Uint8Array>().unwrap().to_vec(),
                    )
                })
                .collect()
        })
    }

    async fn raw_apply_batch(&self, batch: DbBatch) -> Result<(), rexie::Error> {
        let batch: Vec<_> = batch.into();
        for change in batch.iter() {
            match change {
                BatchItem::InsertNewElement(element) => {
                    if self
                        .raw_insert_entry(&element.key.to_bytes(), element.value.to_bytes())
                        .await?
                        .is_some()
                    {
                        // error!("Database replaced element! This should not happen!");
                        // trace!("Problematic key: {:?}", element.key);
                    }
                }
                BatchItem::InsertElement(element) => {
                    self.raw_insert_entry(&element.key.to_bytes(), element.value.to_bytes())
                        .await?;
                }
                BatchItem::DeleteElement(key) => {
                    if self.raw_remove_entry(&key.to_bytes()).await?.is_none() {
                        // error!("Database deleted absent element! This should not happen!");
                        // trace!("Problematic key: {:?}", key);
                    }
                }
                BatchItem::MaybeDeleteElement(key) => {
                    let _ = self.raw_remove_entry(&key.to_bytes()).await?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minimint_api::db::{DatabaseKeyPrefixConst, SerializableDatabaseValue};
    use minimint_api::encoding::{Decodable, Encodable};
    use wasm_bindgen_test::*;

    #[derive(Debug, Encodable, Decodable)]
    struct TestKey(u64);

    impl DatabaseKeyPrefixConst for TestKey {
        const DB_PREFIX: u8 = 0x42;
        type Key = Self;
        type Value = TestVal;
    }

    #[derive(Debug, Encodable, Decodable, Eq, PartialEq)]
    struct TestVal(u64);

    #[wasm_bindgen_test]
    pub async fn create_db() {
        let _ = WebDb::new("minimint").await;
    }

    #[wasm_bindgen_test]
    pub async fn insert_get_value() {
        let db = WebDb::new("minimint").await.unwrap();
        let key = TestKey(1);
        let value = TestVal(1);
        db.raw_insert_entry(&key.to_bytes(), value.to_bytes())
            .await
            .unwrap();
        let got_value = db.raw_get_value(&key.to_bytes()).await.unwrap();
        assert_eq!(got_value, Some(value.to_bytes()));
    }

    #[wasm_bindgen_test]
    pub async fn insert_value() {
        let db = WebDb::new("minimint").await.unwrap();
        let key = TestKey(1);
        let value = TestVal(1);
        db.raw_insert_entry(&key.to_bytes(), value.to_bytes())
            .await
            .unwrap();
    }

    #[wasm_bindgen_test]
    pub async fn insert_get_prefix() {
        let db = WebDb::new("minimint").await.unwrap();
        let key = TestKey(1);
        let value = TestVal(1);
        db.raw_insert_entry(&key.to_bytes(), value.to_bytes())
            .await
            .unwrap();
        let got_value = db.raw_find_by_prefix(&key.to_bytes()).await.unwrap();
        assert_eq!(got_value, vec![(key.to_bytes(), value.to_bytes())]);
    }
}
