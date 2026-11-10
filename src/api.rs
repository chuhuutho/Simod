//! Cầu nối gọi lệnh Tauri từ WASM.
use crate::ctx::Ctx;
use leptos::task::spawn_local;
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"], js_name = "invoke")]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[derive(Serialize)]
struct Wrap<'a, T: Serialize> {
    req: &'a T,
}

/// Gọi một lệnh Tauri. Mọi lệnh nhận một tham số `req`.
pub async fn call<A: Serialize, R: DeserializeOwned>(cmd: &str, req: &A) -> Result<R, String> {
    let args = serde_wasm_bindgen::to_value(&Wrap { req }).map_err(|e| e.to_string())?;
    match tauri_invoke(cmd, args).await {
        Ok(v) => serde_wasm_bindgen::from_value(v).map_err(|e| format!("Lỗi dữ liệu: {e}")),
        Err(e) => Err(e
            .as_string()
            .unwrap_or_else(|| simod_shared::msg::MSG34.to_string())),
    }
}

/// Gọi lệnh ở nền; lỗi hiển thị bằng toast, thành công gọi `ok`.
pub fn fetch<A, R>(ctx: Ctx, cmd: &'static str, req: A, ok: impl FnOnce(R) + 'static)
where
    A: Serialize + 'static,
    R: DeserializeOwned + 'static,
{
    spawn_local(async move {
        match call::<A, R>(cmd, &req).await {
            Ok(v) => ok(v),
            Err(e) => ctx.err(e),
        }
    });
}

/// Như `fetch` nhưng có xử lý lỗi riêng (ví dụ hiển thị lỗi tại chỗ).
pub fn fetch_or<A, R>(cmd: &'static str, req: A, done: impl FnOnce(Result<R, String>) + 'static)
where
    A: Serialize + 'static,
    R: DeserializeOwned + 'static,
{
    spawn_local(async move { done(call::<A, R>(cmd, &req).await) });
}
