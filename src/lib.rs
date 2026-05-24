mod upload_api;

#[cfg(feature = "hydrate")]
mod upload_ui;

#[cfg(feature = "hydrate")]
use wasm_bindgen::JsCast;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(element) = document
        .get_element_by_id("upload-app")
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return;
    };

    leptos::mount::mount_to(element, || {
        leptos::prelude::view! { <upload_ui::UploadApp/> }
    })
    .forget();
}
