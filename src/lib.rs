use std::sync::Arc;

use js_sys::Promise;
use minimint_api::db::mem_impl::MemDatabase;
use mint_client::util::{from_hex, ToHex};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue, UnwrapThrowExt};
use wasm_bindgen_futures::future_to_promise;

#[cfg(test)]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen]
pub struct UserClient {
    inner: Arc<mint_client::UserClient>,
}

#[wasm_bindgen]
impl UserClient {
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> UserClient {
        console_error_panic_hook::set_once();
        let config = config.into_serde().unwrap();
        let inner =
            mint_client::UserClient::new(config, Box::new(MemDatabase::new()), Default::default());
        UserClient {
            inner: Arc::new(inner),
        }
    }

    #[wasm_bindgen]
    pub fn peg_in_address(&self) -> String {
        let inner = self.inner.clone();
        let mut rng = rand::rngs::OsRng::new().unwrap();
        let addr = inner.get_new_pegin_address(&mut rng);
        addr.to_string()
    }

    #[wasm_bindgen]
    pub fn peg_in(&self, txout_proof: String, btc_transaction: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let mut rng = rand::rngs::OsRng::new().unwrap();
            let value = inner
                .peg_in(
                    from_hex(&txout_proof).unwrap(),
                    from_hex(&btc_transaction).unwrap(),
                    &mut rng,
                )
                .await
                .unwrap()
                .to_hex();
            Ok(value.into())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    fn cfg() -> JsValue {
        let config = include_str!("cfg.json");
        js_sys::JSON::parse(config).unwrap()
    }

    #[wasm_bindgen_test]
    async fn new_user_client() {
        UserClient::new(cfg());
    }
}
