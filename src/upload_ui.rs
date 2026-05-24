#![cfg(feature = "hydrate")]

use gloo_net::http::Request as HttpRequest;
use leptos::prelude::*;
use sha2::{Digest, Sha256};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{File, HtmlInputElement, Request, RequestInit, RequestMode};

use crate::upload_api::{
    BrowserUploadCompleteRequest, BrowserUploadFile, BrowserUploadPresignRequest,
    BrowserUploadPresignResponse, UploadHeader, UploadHistoryResponse, UploadHistoryRun,
};

#[component]
pub fn UploadApp() -> impl IntoView {
    let (status, set_status) = signal("Select photos to upload.".to_string());
    let (history, set_history) = signal(Vec::<UploadHistoryRun>::new());

    spawn_local(async move {
        if let Ok(runs) = load_history().await {
            set_history.set(runs);
        }
    });

    let upload = move |_| {
        set_status.set("Preparing upload...".to_string());
        spawn_local(async move {
            match upload_selected_files(set_status).await {
                Ok(()) => {
                    if let Ok(runs) = load_history().await {
                        set_history.set(runs);
                    }
                }
                Err(error) => set_status.set(error),
            }
        });
    };

    view! {
        <section class="panel upload-tool">
            <h2>"Manual upload"</h2>
            <div class="field-grid">
                <label>
                    <span>"Collection"</span>
                    <input id="upload-collection" type="text" value="Pixel uploads"/>
                </label>
                <label>
                    <span>"Photos"</span>
                    <input id="upload-files" type="file" multiple accept="image/*"/>
                </label>
            </div>
            <button type="button" on:click=upload>"Upload"</button>
            <p class="upload-status">{move || status.get()}</p>
        </section>

        <section class="panel">
            <h2>"Recent uploads"</h2>
            <Show
                when=move || !history.get().is_empty()
                fallback=|| view! {
                    <div class="empty-state">
                        <h2>"No upload history yet"</h2>
                        <p>"Uploaded batches will appear here."</p>
                    </div>
                }
            >
                <table class="upload-history">
                    <thead>
                        <tr>
                            <th>"Collection"</th>
                            <th>"Status"</th>
                            <th>"Files"</th>
                            <th>"Started"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || history.get()
                            key=|run| run.ingest_id.clone()
                            children=|run| view! {
                                <tr>
                                    <td>{run.collection_name}</td>
                                    <td>{run.status}</td>
                                    <td>{format!("{}/{}", run.completed_assets, run.total_assets)}</td>
                                    <td>{run.started_at}</td>
                                </tr>
                            }
                        />
                    </tbody>
                </table>
            </Show>
        </section>
    }
}

async fn load_history() -> Result<Vec<UploadHistoryRun>, String> {
    let response: UploadHistoryResponse = HttpRequest::get("/api/uploads/browser/history")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;

    Ok(response.runs)
}

async fn upload_selected_files(set_status: WriteSignal<String>) -> Result<(), String> {
    let collection_name = input_value("upload-collection")?;
    if collection_name.trim().is_empty() {
        return Err("Enter a collection name.".to_string());
    }

    let files = selected_files("upload-files")?;
    if files.is_empty() {
        return Err("Select at least one photo.".to_string());
    }

    set_status.set(format!("Hashing {} file(s)...", files.len()));
    let mut selected = Vec::with_capacity(files.len());
    for file in files {
        let bytes = read_file_bytes(&file).await?;
        let sha256 = hex::encode(Sha256::digest(&bytes));
        let mime_type = if file.type_().is_empty() {
            "application/octet-stream".to_string()
        } else {
            file.type_()
        };
        let facts = BrowserUploadFile {
            name: file.name(),
            sha256,
            bytes: file.size() as u64,
            mime_type,
        };

        selected.push((file, facts, bytes));
    }

    let request = BrowserUploadPresignRequest {
        collection_name,
        files: selected.iter().map(|(_, facts, _)| facts.clone()).collect(),
    };

    set_status.set("Creating upload session...".to_string());
    let response: BrowserUploadPresignResponse = HttpRequest::post("/api/uploads/browser/presign")
        .json(&request)
        .map_err(|error| error.to_string())?
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;

    let mut uploaded_sha256 = Vec::new();
    for (index, target) in response.files.iter().enumerate() {
        set_status.set(format!(
            "Uploading {}/{}: {}",
            index + 1,
            response.files.len(),
            target.name
        ));

        if !target.already_uploaded {
            let Some((file, facts, _)) = selected
                .iter()
                .find(|(_, facts, _)| facts.sha256 == target.sha256)
            else {
                return Err(format!("Missing selected file {}", target.name));
            };

            put_file(
                &target.original_upload_url,
                &target.original_headers,
                file.clone(),
            )
            .await?;
            put_metadata(
                &target.metadata_upload_url,
                &target.metadata_headers,
                metadata_json(&request.collection_name, facts),
            )
            .await?;
        }

        uploaded_sha256.push(target.sha256.clone());
    }

    let complete = BrowserUploadCompleteRequest { uploaded_sha256 };
    let complete_url = format!("/api/uploads/browser/{}/complete", response.ingest_id);
    HttpRequest::post(&complete_url)
        .json(&complete)
        .map_err(|error| error.to_string())?
        .send()
        .await
        .map_err(|error| error.to_string())?;

    set_status.set("Upload complete.".to_string());
    Ok(())
}

fn input_value(id: &str) -> Result<String, String> {
    let input = document_element(id)?
        .dyn_into::<HtmlInputElement>()
        .map_err(|_| format!("{id} is not an input"))?;
    Ok(input.value())
}

fn selected_files(id: &str) -> Result<Vec<File>, String> {
    let input = document_element(id)?
        .dyn_into::<HtmlInputElement>()
        .map_err(|_| format!("{id} is not an input"))?;
    let Some(file_list) = input.files() else {
        return Ok(Vec::new());
    };

    let mut files = Vec::new();
    for index in 0..file_list.length() {
        if let Some(file) = file_list.item(index) {
            files.push(file);
        }
    }

    Ok(files)
}

fn document_element(id: &str) -> Result<web_sys::Element, String> {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(id))
        .ok_or_else(|| format!("Missing #{id}"))
}

async fn read_file_bytes(file: &File) -> Result<Vec<u8>, String> {
    let array_buffer = JsFuture::from(file.array_buffer())
        .await
        .map_err(|_| format!("Could not read {}", file.name()))?;
    Ok(js_sys::Uint8Array::new(&array_buffer).to_vec())
}

async fn put_file(url: &str, headers: &[UploadHeader], file: File) -> Result<(), String> {
    let init = RequestInit::new();
    init.set_method("PUT");
    init.set_mode(RequestMode::Cors);
    init.set_body(&file);
    put_request(url, headers, &init).await
}

async fn put_metadata(url: &str, headers: &[UploadHeader], body: String) -> Result<(), String> {
    let init = RequestInit::new();
    init.set_method("PUT");
    init.set_mode(RequestMode::Cors);
    init.set_body(&wasm_bindgen::JsValue::from_str(&body));
    put_request(url, headers, &init).await
}

async fn put_request(
    url: &str,
    headers: &[UploadHeader],
    init: &RequestInit,
) -> Result<(), String> {
    let request = Request::new_with_str_and_init(url, init)
        .map_err(|_| "Could not build upload request.".to_string())?;
    for header in headers {
        request
            .headers()
            .set(&header.name, &header.value)
            .map_err(|_| format!("Could not set upload header {}", header.name))?;
    }

    let response = JsFuture::from(
        web_sys::window()
            .ok_or_else(|| "Missing browser window.".to_string())?
            .fetch_with_request(&request),
    )
    .await
    .map_err(|_| "Upload request failed.".to_string())?
    .dyn_into::<web_sys::Response>()
    .map_err(|_| "Upload response was invalid.".to_string())?;

    if response.ok() {
        Ok(())
    } else {
        Err(format!("Upload failed with status {}", response.status()))
    }
}

fn metadata_json(collection_name: &str, file: &BrowserUploadFile) -> String {
    serde_json::json!({
        "source": "manual_upload",
        "collection_name": collection_name,
        "file_name": file.name,
        "sha256": file.sha256,
        "bytes": file.bytes,
        "mime_type": file.mime_type,
        "uploaded_at": js_sys::Date::new_0().to_string().as_string().unwrap_or_default(),
    })
    .to_string()
}
