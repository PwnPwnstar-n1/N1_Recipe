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

/// Several tests below touch the REAL system clipboard. Windows clipboard
/// access must not race across threads, so they serialize on this lock (the
/// test harness runs tests on multiple threads).
static CLIPBOARD_LOCK: Mutex<()> = Mutex::new(());

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
            commands::list_variables,
            commands::list_categories,
            commands::get_settings,
            commands::set_language,
            commands::show_launcher,
            commands::hide_launcher,
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
    let _clipboard_guard = CLIPBOARD_LOCK.lock().unwrap();
    let (app, webview) = setup();
    let handle = app.handle().clone();

    // preserve whatever was in the user's clipboard and restore it afterwards
    let original = handle.clipboard().read_text().unwrap_or_default();

    let created = invoke(&webview, "create_snippet", json!({ "input": sample_input() }))
        .expect("create failed");
    let id = created["id"].as_i64().unwrap();

    let result = invoke(&webview, "copy_snippet", json!({ "id": id, "values": {} }))
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

// ---------------------------------------------------------------------------
// Template variables
// ---------------------------------------------------------------------------

fn create_with_content(webview: &MockWebview, title: &str, content: &str) -> i64 {
    let created = invoke(
        webview,
        "create_snippet",
        json!({ "input": {
            "title": title,
            "content": content,
            "type": "TEXT",
            "category": "",
            "language": "",
            "tags": [],
            "aliases": []
        }}),
    )
    .expect("create failed");
    created["id"].as_i64().expect("id missing")
}

#[test]
fn template_case1_ordinary_variable() {
    let _clipboard_guard = CLIPBOARD_LOCK.lock().unwrap();
    let (app, webview) = setup();
    let handle = app.handle().clone();
    let original = handle.clipboard().read_text().unwrap_or_default();

    let id = create_with_content(&webview, "greet", "Hello {{name}}");

    // dialog would be built from this list
    let vars = invoke(&webview, "list_variables", json!({ "id": id }))
        .expect("list_variables failed");
    assert_eq!(vars, json!(["name"]));

    let result = invoke(&webview, "copy_snippet", json!({ "id": id, "values": { "name": "N1" } }))
        .expect("copy_snippet failed");
    assert_eq!(result["text"], "Hello N1");
    assert_eq!(result["snippet"]["usageCount"], 1);
    assert_eq!(handle.clipboard().read_text().unwrap(), "Hello N1");

    handle.clipboard().write_text(original).expect("restore clipboard failed");
}

#[test]
fn template_case2_system_variable_only_no_dialog() {
    let _clipboard_guard = CLIPBOARD_LOCK.lock().unwrap();
    let (app, webview) = setup();
    let handle = app.handle().clone();
    let original = handle.clipboard().read_text().unwrap_or_default();

    let id = create_with_content(&webview, "ts", "Generated: {{datetime}}");

    // no ordinary variables -> the frontend would copy without a dialog
    let vars = invoke(&webview, "list_variables", json!({ "id": id }))
        .expect("list_variables failed");
    assert_eq!(vars, json!([]));

    let result = invoke(&webview, "copy_snippet", json!({ "id": id, "values": {} }))
        .expect("copy_snippet failed");
    let text = result["text"].as_str().unwrap();
    assert!(is_datetime_template(text), "unexpected datetime text: {text}");
    assert_eq!(result["snippet"]["usageCount"], 1);
    assert_eq!(handle.clipboard().read_text().unwrap(), text);

    handle.clipboard().write_text(original).expect("restore clipboard failed");
}

/// `Generated: YYYY-MM-DD HH:mm:ss`
fn is_datetime_template(text: &str) -> bool {
    let rest = text.strip_prefix("Generated: ").unwrap_or_default();
    rest.len() == "YYYY-MM-DD HH:mm:ss".len()
        && rest
            .chars()
            .enumerate()
            .all(|(i, c)| match i {
                4 | 7 => c == '-',
                13 | 16 => c == ':',
                10 => c == ' ',
                _ => c.is_ascii_digit(),
            })
}

#[test]
fn template_case3_clipboard_and_ordinary_variable() {
    let _clipboard_guard = CLIPBOARD_LOCK.lock().unwrap();
    let (app, webview) = setup();
    let handle = app.handle().clone();
    let original = handle.clipboard().read_text().unwrap_or_default();

    // pre-set the system clipboard, as the case requires
    handle
        .clipboard()
        .write_text("ctx from clipboard")
        .expect("failed to preset clipboard");

    let content = "Analyze:\n\n{{clipboard}}\n\nGoal:\n{{goal}}";
    let id = create_with_content(&webview, "analyze", content);

    let vars = invoke(&webview, "list_variables", json!({ "id": id }))
        .expect("list_variables failed");
    assert_eq!(vars, json!(["goal"]));

    let result = invoke(&webview, "copy_snippet", json!({ "id": id, "values": { "goal": "G1" } }))
        .expect("copy_snippet failed");
    let expected = "Analyze:\n\nctx from clipboard\n\nGoal:\nG1";
    assert_eq!(result["text"], expected);
    assert_eq!(handle.clipboard().read_text().unwrap(), expected);
    assert_eq!(result["snippet"]["usageCount"], 1);

    handle.clipboard().write_text(original).expect("restore clipboard failed");
}

#[test]
fn template_empty_value_replaces_with_empty_string() {
    let _clipboard_guard = CLIPBOARD_LOCK.lock().unwrap();
    let (app, webview) = setup();
    let handle = app.handle().clone();
    let original = handle.clipboard().read_text().unwrap_or_default();

    let id = create_with_content(&webview, "desc", "Please: {{description}}");
    let result = invoke(
        &webview,
        "copy_snippet",
        json!({ "id": id, "values": { "description": "" } }),
    )
    .expect("copy_snippet failed");
    assert_eq!(result["text"], "Please: ");
    assert_eq!(result["snippet"]["usageCount"], 1);

    handle.clipboard().write_text(original).expect("restore clipboard failed");
}

#[test]
fn template_missing_values_rejected_without_counting() {
    let (_app, webview) = setup();
    let id = create_with_content(&webview, "multi", "{{host}}:{{port}}");

    let result = invoke(&webview, "copy_snippet", json!({ "id": id, "values": {} }));
    let err = result.unwrap_err();
    assert!(err.contains("missing variable values"), "unexpected error: {err}");
    assert!(err.contains("host") && err.contains("port"));

    // usage_count must NOT have been incremented
    let snippet = invoke(&webview, "get_snippet", json!({ "id": id })).expect("get failed");
    assert_eq!(snippet["usageCount"], 0);

    // partial values are also rejected
    let result = invoke(&webview, "copy_snippet", json!({ "id": id, "values": { "host": "h" } }));
    assert!(result.is_err());
    let snippet = invoke(&webview, "get_snippet", json!({ "id": id })).expect("get failed");
    assert_eq!(snippet["usageCount"], 0);
}

#[test]
fn template_dialog_cancel_contract_no_copy_invocation() {
    // Cancel is a pure frontend action: the dialog is closed without calling
    // copy_snippet. This test pins the backend half of that contract — merely
    // listing variables (what the dialog does on open) must not touch
    // usage_count, and only copy_snippet performs the copy.
    let (_app, webview) = setup();
    let id = create_with_content(&webview, "cancel", "Hello {{name}}");

    let _ = invoke(&webview, "list_variables", json!({ "id": id })).unwrap();
    let snippet = invoke(&webview, "get_snippet", json!({ "id": id })).expect("get failed");
    assert_eq!(snippet["usageCount"], 0);
    assert!(snippet["lastUsedAt"].is_null());
}

// ---------------------------------------------------------------------------
// Window lifecycle (multi-window foundation)
// ---------------------------------------------------------------------------

// Note: get_settings/set_language are deliberately not tested at IPC level —
// the mock runtime's path resolver would read/write the REAL
// settings.json. The settings module itself is unit-tested, and the real
// persistence flow is covered by runtime verification.


#[test]
fn launcher_lifecycle_commands_wiring() {
    // Command-wiring test. The mock runtime's window ops are stubs (hide()
    // is a no-op), so visibility cannot be observed here; the REAL
    // show/hide lifecycle is verified at runtime via CDP.
    //
    // There is deliberately NO destroy/recreate command: creating a webview
    // from an IPC/sync-command thread deadlocks on Windows (upstream wry
    // issue), so the launcher window is created once at startup and only
    // shown/hidden — see src/windows.rs.
    let (app, webview) = setup();
    let handle = app.handle().clone();

    snipdeck_lib::windows::create_launcher(&handle).expect("failed to create launcher");
    assert!(
        snipdeck_lib::windows::get(&handle, snipdeck_lib::windows::roles::LAUNCHER).is_some(),
        "launcher should exist after creation"
    );

    invoke(&webview, "show_launcher", json!({})).expect("show_launcher failed");
    invoke(&webview, "hide_launcher", json!({})).expect("hide_launcher failed");

    // repeated show/hide cycles keep working (no state degradation)
    for _ in 0..3 {
        invoke(&webview, "show_launcher", json!({})).expect("show in cycle failed");
        invoke(&webview, "hide_launcher", json!({})).expect("hide in cycle failed");
    }
}
