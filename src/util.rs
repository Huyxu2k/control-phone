use serde::{Deserialize, Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // invoke without arguments
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn invoke_without_args_promise(cmd: &str) -> Result<JsValue, JsValue>;

    // invoke with arguments (default)
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke_promise(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    // listen(event, handler) -> Promise<UnlistenFn>
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
    async fn listen_promise(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> Result<JsValue, JsValue>;
}

/// Event Object Tauri truyền vào callback: { event, id, payload: {...} }
#[derive(Deserialize)]
struct TauriEvent<T> {
    payload: T
}

/// Giữ Listener sống: khi bị drop sẽ tự gọi unlisten
pub struct EventListener {
    unlisten: js_sys::Function,
    _closure: Closure<dyn FnMut(JsValue)>
}
impl Drop for EventListener {
    fn drop(&mut self) {
        let _ = self.unlisten.call0(&JsValue::NULL);
    }
}

pub async fn listen<T, F>(event: &str, mut handler: F) -> Result<EventListener, String> 
where 
    T: DeserializeOwned + 'static,
    F: FnMut(T) + 'static,
{
    let event_name = event.to_string();
    let closure = Closure::<dyn FnMut(JsValue)>::new(move | raw: JsValue | {
        match serde_wasm_bindgen::from_value::<TauriEvent<T>>(raw) {
            Ok(v) => handler(v.payload),
            Err(e) => dioxus_logger::tracing::error!(
                "Invalid payload for event '{event_name}': {e}"
            )
        }
    });

    let unlisten = listen_promise(event, &closure).await
                                                                .map_err(js_err)?
                                                                .dyn_into::<js_sys::Function>()
                                                                .map_err(|_| "listen() did not return an unlisten function".to_string())?;

    Ok(EventListener { unlisten, _closure: closure })
}

pub async fn invoke<A, R>(cmd: &str, args: &A) -> Result<R,  String> 
where 
    A: Serialize + ?Sized,
    R: DeserializeOwned
{
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    let args = args.serialize(&serializer).map_err(|e| e.to_string())?;

    let result = if args.is_null() || args.is_undefined() {
        invoke_without_args_promise(cmd).await
    } else {
        invoke_promise(cmd, args).await
    };

    let value = result.map_err(js_err)?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

fn js_err(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}