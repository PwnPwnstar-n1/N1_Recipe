//! End-to-end IPC tests: drive the Tauri command layer through the mock
//! runtime exactly like the frontend does (JSON args, camelCase keys),
//! verifying serde marshalling, command wiring and the clipboard plugin.

use serde_json::{json, Value};
use snipdeck_lib::{commands, db};
use std::sync::Mutex;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_clipboard_manager::ClipboardExt;

type MockApp = tauri::App<tauri::test::MockRuntime>;
type MockWebview = tauri::WebviewWindow<tauri::test::MockRuntime>;

fn setup() -> (MockApp, MockWebview) {
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_snippets,
            commands::search_snippets,
            commands::get_snippet,
            commands::create_snippet,
            commands::update_snippet,
            commands::delete_snippet,
            commands::toggle_favorite,
            commands::copy_snippet,
            commands::list_categories,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build mock app");

    let conn = db::open_in_memory(false).expect("failed to open in-memory db");
    app.manage(db::Db(Mutex::new(conn)));

    let webview = WebviewWindowBuilder::new(&app, "main", WebviewUrl::App("index.html".into()))
        .build()
        .expect("failed to build mock webview");
    (app, webview)
}

fn invoke(webview: &MockWebview, cmd: &str, args: Value) -> Result<Value, String> {
    let response = tauri::test::get_ipc_response(
        webview,
        tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        },
    );
    match response {
        Ok(tauri::ipc::InvokeResponseBody::Json(raw)) => {
            serde_json::from_str::<Value>(&raw).map_err(|e| e.to_string())
        }
        Ok(other) => Err(format!("unexpected response body: {other:?}")),
        Err(err) => match err {
            Value::String(s) => Err(s),
            other => Err(other.to_string()),
        },
    }
}

fn sample_input() -> Value {
    json!({
        "title": "ret2libc EXP Template",
        "content": "from pwn import *\np = process('./binary')",
        "type": "EXP",
        "category": "Security/Pwn",
        "language": "python",
        "tags": ["pwn", "pwntools"],
        "aliases": ["rtl"]
    })
}

#[test]
fn create_search_update_delete_via_ipc() {
    let (_app, webview) = setup();

    // create — camelCase payload keys must map onto the Rust command args
    let created = invoke(&webview, "create_snippet", json!({ "input": sample_input() }))
        .expect("create_snippet failed");
    assert_eq!(created["title"], "ret2libc EXP Template");
    assert_eq!(created["type"], "EXP");
    assert_eq!(created["usageCount"], 0);
    assert_eq!(created["tags"], json!(["pwn", "pwntools"]));
    let id = created["id"].as_i64().expect("id missing");

    // search — finds via alias `rtl`
    let hits = invoke(&webview, "search_snippets", json!({ "query": "rtl" }))
        .expect("search_snippets failed");
    assert_eq!(hits.as_array().map(Vec::len), Some(1));
    assert_eq!(hits[0]["id"], id);

    // update
    let updated = invoke(
        &webview,
        "update_snippet",
        json!({ "id": id, "input": {
            "title": "changed",
            "content": "new",
            "type": "COMMAND",
            "category": "Dev/Git",
            "language": "bash",
            "tags": ["git"],
            "aliases": []
        }}),
    )
    .expect("update_snippet failed");
    assert_eq!(updated["title"], "changed");
    assert_eq!(updated["tags"], json!(["git"]));

    // delete
    let deleted = invoke(&webview, "delete_snippet", json!({ "id": id }))
        .expect("delete_snippet failed");
    assert_eq!(deleted, Value::Null);

    let hits = invoke(&webview, "search_snippets", json!({ "query": "changed" }))
        .expect("search_snippets failed");
    assert_eq!(hits.as_array().map(Vec::len), Some(0));
}

#[test]
fn views_and_favorites_via_ipc() {
    let (_app, webview) = setup();
    let _a = invoke(&webview, "create_snippet", json!({ "input": sample_input() }))
        .expect("create failed");
    let b = invoke(
        &webview,
        "create_snippet",
        json!({ "input": {
            "title": "curl template",
            "content": "curl example",
            "type": "COMMAND",
            "category": "Dev/HTTP",
            "language": "bash",
            "tags": [],
            "aliases": []
        }}),
    )
    .expect("create failed");
    let b_id = b["id"].as_i64().unwrap();

    invoke(&webview, "toggle_favorite", json!({ "id": b_id })).expect("toggle failed");
    let favs = invoke(&webview, "list_snippets", json!({ "view": "favorites", "category": null }))
        .expect("list failed");
    let favs = favs.as_array().unwrap();
    assert_eq!(favs.len(), 1);
    assert_eq!(favs[0]["id"], b_id);

    let all = invoke(&webview, "list_snippets", json!({ "view": "all", "category": null }))
        .expect("list failed");
    assert_eq!(all.as_array().unwrap().len(), 2);

    // categories
    let cats = invoke(&webview, "list_categories", json!({})).expect("list failed");
    assert_eq!(cats.as_array().unwrap().len(), 2);
}

#[test]
fn copy_via_ipc_writes_clipboard_and_bumps_usage() {
    let (app, webview) = setup();
    let handle = app.handle().clone();

    // preserve whatever was in the user's clipboard and restore it afterwards
    let original = handle.clipboard().read_text().unwrap_or_default();

    let created = invoke(&webview, "create_snippet", json!({ "input": sample_input() }))
        .expect("create failed");
    let id = created["id"].as_i64().unwrap();

    let result = invoke(&webview, "copy_snippet", json!({ "id": id }))
        .expect("copy_snippet failed");
    assert_eq!(result["text"], sample_input()["content"]);
    assert_eq!(result["snippet"]["usageCount"], 1);
    assert!(result["snippet"]["lastUsedAt"].is_i64());

    // the OS clipboard must actually contain the copied text
    let clipboard_text = handle.clipboard().read_text().expect("failed to read clipboard");
    assert_eq!(clipboard_text, "from pwn import *\np = process('./binary')");

    handle
        .clipboard()
        .write_text(original)
        .expect("failed to restore clipboard");
}

#[test]
fn invalid_type_is_rejected_via_ipc() {
    let (_app, webview) = setup();
    let mut input = sample_input();
    input["type"] = json!("NOPE");
    let result = invoke(&webview, "create_snippet", json!({ "input": input }));
    assert!(result.is_err(), "invalid type must be rejected");
    assert!(result.unwrap_err().contains("NOPE"));
}

#[test]
fn title_required_via_ipc() {
    let (_app, webview) = setup();
    let mut input = sample_input();
    input["title"] = json!("   ");
    let result = invoke(&webview, "create_snippet", json!({ "input": input }));
    assert_eq!(result.unwrap_err(), "title is required");
}
