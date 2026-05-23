use leptos::prelude::*;

use crate::config::AppConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Uploads,
    Restores,
}

pub fn render_page(page: Page, config: &AppConfig) -> String {
    view! {
        <Document page=page config=config.clone()/>
    }
    .to_html()
}

#[component]
fn Document(page: Page, config: AppConfig) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>"Cloud Image Storage"</title>
                <style>{MAIN_CSS}</style>
            </head>
            <body>
                <AppShell page=page config=config/>
            </body>
        </html>
    }
}

#[component]
fn AppShell(page: Page, config: AppConfig) -> impl IntoView {
    view! {
        <nav class="top-nav">
            <a class:active=move || page == Page::Home href="/">"Home"</a>
            <a class:active=move || page == Page::Uploads href="/uploads">"Uploads"</a>
            <a class:active=move || page == Page::Restores href="/restores">"Restores"</a>
        </nav>
        <main>
            {match page {
                Page::Home => view! { <HomePage config=config/> }.into_any(),
                Page::Uploads => view! { <UploadsPage/> }.into_any(),
                Page::Restores => view! { <RestoresPage/> }.into_any(),
            }}
        </main>
    }
}

#[component]
fn HomePage(config: AppConfig) -> impl IntoView {
    view! {
        <section class="page-heading">
            <p class="eyebrow">"Private archive"</p>
            <h1>"Cloud Image Storage"</h1>
            <p>"A WireGuard-only image archive for Google Takeout imports, manual Pixel uploads, cold storage, and restores."</p>
        </section>

        <section class="panel">
            <h2>"System"</h2>
            <dl class="facts">
                <div>
                    <dt>"Bucket"</dt>
                    <dd><code>{config.storage.bucket}</code></dd>
                </div>
                <div>
                    <dt>"Region"</dt>
                    <dd><code>{config.storage.region}</code></dd>
                </div>
                <div>
                    <dt>"Originals prefix"</dt>
                    <dd><code>{config.storage.originals_prefix}</code></dd>
                </div>
                <div>
                    <dt>"Previews prefix"</dt>
                    <dd><code>{config.storage.previews_prefix}</code></dd>
                </div>
                <div>
                    <dt>"Metadata prefix"</dt>
                    <dd><code>{config.storage.metadata_prefix}</code></dd>
                </div>
                <div>
                    <dt>"Database configured"</dt>
                    <dd><code>{(!config.database.url.is_empty()).to_string()}</code></dd>
                </div>
                <div>
                    <dt>"Base URL"</dt>
                    <dd><code>{config.server.public_base_url}</code></dd>
                </div>
            </dl>
        </section>

        <section class="panel">
            <h2>"Ingest"</h2>
            <p>"Use the CLI for large batches. Browser uploads will use the same ingest API once persistence and S3 upload plumbing are in place."</p>
            <div class="command-list">
                <code>"cis import takeout --path /path/to/takeout --collection \"Google Takeout 2026-05\""</code>
                <code>"cis upload --path /path/to/photos --collection \"Pixel uploads\""</code>
            </div>
        </section>
    }
}

#[component]
fn UploadsPage() -> impl IntoView {
    view! {
        <section class="page-heading">
            <p class="eyebrow">"Ingest"</p>
            <h1>"Uploads"</h1>
            <p>"Manual upload batches and Google Takeout imports will appear here once Postgres persistence is connected."</p>
        </section>
        <section class="panel empty-state">
            <h2>"No upload history yet"</h2>
            <p>"The current server can scan manifests and expose API contracts. The next slice stores ingest runs, assets, and collection membership."</p>
        </section>
    }
}

#[component]
fn RestoresPage() -> impl IntoView {
    view! {
        <section class="page-heading">
            <p class="eyebrow">"Cold storage"</p>
            <h1>"Restores"</h1>
            <p>"Temporary restores default to 30 days. Pin-hot restores will copy objects back into a hot storage class."</p>
        </section>
        <section class="panel empty-state">
            <h2>"No restore jobs yet"</h2>
            <p>"Restore selection and status tracking will be backed by Postgres before S3 restore calls are enabled."</p>
        </section>
    }
}

const MAIN_CSS: &str = r#"
:root {
  color-scheme: light;
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  color: #1d211f;
  background: #f8f7f4;
}

body {
  margin: 0;
}

.top-nav {
  display: flex;
  gap: 1rem;
  align-items: center;
  min-height: 3.5rem;
  padding: 0 2rem;
  border-bottom: 1px solid #dfddd5;
  background: #ffffff;
}

.top-nav a {
  color: #3d443f;
  text-decoration: none;
  font-weight: 650;
}

.top-nav a.active {
  color: #0a6847;
}

main {
  width: min(70rem, calc(100vw - 3rem));
  margin: 2rem auto 4rem;
}

.page-heading {
  margin-bottom: 1.5rem;
}

.eyebrow {
  margin: 0 0 .4rem;
  color: #5c665f;
  font-size: .85rem;
  font-weight: 700;
  text-transform: uppercase;
}

h1 {
  margin: 0;
  font-size: 2rem;
}

h2 {
  margin: 0 0 1rem;
  font-size: 1.1rem;
}

p {
  line-height: 1.55;
}

.panel {
  border: 1px solid #dfddd5;
  border-radius: 8px;
  padding: 1rem;
  margin: 1rem 0;
  background: #ffffff;
}

.facts {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
  gap: 1rem;
  margin: 0;
}

.facts div {
  min-width: 0;
}

dt {
  color: #5c665f;
  font-size: .85rem;
  font-weight: 700;
}

dd {
  margin: .25rem 0 0;
}

code {
  display: inline-block;
  max-width: 100%;
  overflow-wrap: anywhere;
  border-radius: 6px;
  padding: .15rem .35rem;
  background: #eeece5;
}

.command-list {
  display: grid;
  gap: .5rem;
}

.empty-state {
  color: #3d443f;
}
"#;
