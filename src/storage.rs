use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use serde::de::DeserializeOwned;
use serde::Serialize;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn get_raw(key: &str) -> Option<String> {
    storage()?.get_item(key).ok()?
}

pub fn set_raw(key: &str, value: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(key, value);
    }
}

pub fn get_json<T: DeserializeOwned>(key: &str) -> Option<T> {
    let raw = get_raw(key)?;
    serde_json::from_str(&raw).ok()
}

pub fn set_json<T: Serialize>(key: &str, value: &T) {
    if let Ok(s) = serde_json::to_string(value) {
        set_raw(key, &s);
    }
}

pub async fn fetch_text(url: &str) -> Result<String, JsValue> {
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let request = Request::new_with_str_and_init(url, &opts)?;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;
    if !resp.ok() {
        return Err(JsValue::from_str(&format!("HTTP {}", resp.status())));
    }
    let text_promise = resp.text()?;
    let text_value = JsFuture::from(text_promise).await?;
    Ok(text_value.as_string().unwrap_or_default())
}

/// POSTs a JSON body and returns the response body as text (caller parses
/// it). Used for talking to the word-check backend.
pub async fn fetch_post_json(url: &str, body: &str) -> Result<String, JsValue> {
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(body));
    let headers = Headers::new()?;
    headers.set("Content-Type", "application/json")?;
    opts.set_headers(&headers);
    let request = Request::new_with_str_and_init(url, &opts)?;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;
    if !resp.ok() {
        return Err(JsValue::from_str(&format!("HTTP {}", resp.status())));
    }
    let text_promise = resp.text()?;
    let text_value = JsFuture::from(text_promise).await?;
    Ok(text_value.as_string().unwrap_or_default())
}
