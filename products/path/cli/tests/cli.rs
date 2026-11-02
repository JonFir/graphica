use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{Arc, Barrier},
};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_path")
}
fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples")
        .join(name)
}
fn invoke(args: &[&str]) -> (i32, Value, Output) {
    let output = Command::new(binary()).args(args).output().unwrap();
    let status = output.status.code().unwrap();
    let text = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(
        text.lines().count(),
        1,
        "stdout must contain one JSON object: {text}"
    );
    let value = serde_json::from_str(text).unwrap();
    (status, value, output)
}
fn invoke_text(args: &[&str]) -> (i32, String) {
    let output = Command::new(binary()).args(args).output().unwrap();
    let status = output.status.code().unwrap();
    (status, String::from_utf8(output.stdout).unwrap())
}
fn run(state: &Path, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["--json", "run", "--state-dir"];
    let dir = state.to_str().unwrap();
    all.push(dir);
    all.extend_from_slice(args);
    let (code, value, _) = invoke(&all);
    (code, value)
}
fn input(value: &Value, node: &str) -> String {
    value["data"]["view"]["availableInputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["nodeId"] == node)
        .unwrap()["activationId"]
        .as_str()
        .unwrap()
        .to_owned()
}
#[test]
fn json_arguments_and_validation() {
    let (code, value, _) = invoke(&["--json", "run", "choose"]);
    assert_eq!(code, 2);
    assert_eq!(value["error"]["code"], "invalid-arguments");
    let file = example("decision-tree.yaml");
    let (code, value, _) = invoke(&["--json", "validate", file.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert_eq!(value["data"]["valid"], true);
}
#[test]
fn persistent_run_queries_and_rejections() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("runs");
    let file = example("decision-tree.json");
    let (code, mut value) = run(&state, &["start", file.to_str().unwrap()]);
    assert_eq!(code, 0);
    let id = value["data"]["view"]["runId"].as_str().unwrap().to_owned();
    let run_file = state.join(format!("{id}.json"));
    let before = fs::read(&run_file).unwrap();
    let (code, next) = run(&state, &["next", &id]);
    assert_eq!(code, 0);
    assert_eq!(next["data"]["revision"], 1);
    let (code, _) = run(&state, &["status", &id]);
    assert_eq!(code, 0);
    let (code, _) = run(&state, &["history", &id]);
    assert_eq!(code, 0);
    assert_eq!(before, fs::read(&run_file).unwrap());
    let a = input(&value, "understood");
    let (code, rejected) = run(&state, &["choose", &id, &a, "missing", "--revision", "1"]);
    assert_eq!(code, 2);
    assert_eq!(rejected["error"]["code"], "invalid-choice");
    assert_eq!(before, fs::read(&run_file).unwrap());
    let (code, changed) = run(&state, &["choose", &id, &a, "known", "--revision", "1"]);
    assert_eq!(code, 0);
    value = changed;
    let after = fs::read(&run_file).unwrap();
    let (code, rejected) = run(&state, &["choose", &id, &a, "known", "--revision", "1"]);
    assert_eq!(code, 3);
    assert_eq!(rejected["error"]["code"], "revision-conflict");
    assert_eq!(after, fs::read(&run_file).unwrap());
    let b = input(&value, "covered");
    let (code, completed) = run(&state, &["choose", &id, &b, "untested", "--revision", "2"]);
    assert_eq!(code, 0);
    assert_eq!(completed["data"]["view"]["outcome"]["value"], "add-test");
    let (code, rejected) = run(&state, &["choose", &id, &b, "tested", "--revision", "3"]);
    assert_eq!(code, 3);
    assert_eq!(rejected["error"]["code"], "run-completed");
}
#[test]
fn missing_corrupt_and_invalid_start() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("runs");
    let (code, missing) = run(&state, &["status", "missing"]);
    assert_eq!(code, 3);
    assert_eq!(missing["error"]["code"], "run-not-found");
    assert!(!state.exists());
    fs::create_dir(&state).unwrap();
    let (code, missing) = run(
        &state,
        &["choose", "absent", "a1", "known", "--revision", "1"],
    );
    assert_eq!(code, 3);
    assert_eq!(missing["error"]["code"], "run-not-found");
    assert!(!state.join("absent.lock").exists());
    let (code, bad_id) = run(&state, &["status", "../secret"]);
    assert_eq!(code, 2);
    assert_eq!(bad_id["error"]["code"], "invalid-arguments");
    let bad_doc = temp.path().join("invalid.json");
    fs::write(
        &bad_doc,
        r#"{"formatVersion":"0.1","kind":"decision-tree"}"#,
    )
    .unwrap();
    let (code, _) = run(&state, &["start", bad_doc.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(fs::read_dir(&state).unwrap().next().is_none());
    fs::write(state.join("corrupt.json"), "{}").unwrap();
    let (code, corrupt) = run(&state, &["status", "corrupt"]);
    assert_eq!(code, 1);
    assert_eq!(corrupt["error"]["code"], "corrupt-run");
}
#[test]
fn inconsistent_stored_state_is_rejected_without_panicking() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("runs");
    let file = example("behavior-tree.json");
    let (_, started) = run(&state, &["start", file.to_str().unwrap()]);
    let id = started["data"]["view"]["runId"].as_str().unwrap();
    let saved = state.join(format!("{id}.json"));
    let mut stored: Value = serde_json::from_slice(&fs::read(&saved).unwrap()).unwrap();
    stored["state"]["stack"].as_array_mut().unwrap().pop();
    fs::write(&saved, serde_json::to_vec(&stored).unwrap()).unwrap();

    for args in [
        vec!["status", id],
        vec!["report", id, "a2", "success", "--revision", "1"],
    ] {
        let (code, response) = run(&state, &args);
        assert_eq!(code, 1);
        assert_eq!(response["error"]["code"], "corrupt-run");
    }

    let file = example("activity-diagram.json");
    let (_, started) = run(&state, &["start", file.to_str().unwrap()]);
    let id = started["data"]["view"]["runId"].as_str().unwrap();
    let saved = state.join(format!("{id}.json"));
    let mut stored: Value = serde_json::from_slice(&fs::read(&saved).unwrap()).unwrap();
    stored["state"]["tokens"][0]["nodeId"] = Value::String("docs".into());
    fs::write(&saved, serde_json::to_vec(&stored).unwrap()).unwrap();
    let (code, response) = run(&state, &["status", id]);
    assert_eq!(code, 1);
    assert_eq!(response["error"]["code"], "corrupt-run");
}
#[test]
fn two_mutations_at_one_revision_commit_once() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("runs");
    let file = example("decision-tree.json");
    let (_, started) = run(&state, &["start", file.to_str().unwrap()]);
    let id = started["data"]["view"]["runId"]
        .as_str()
        .unwrap()
        .to_owned();
    let activation = input(&started, "understood");
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = ["known", "unknown"]
        .into_iter()
        .map(|choice| {
            let barrier = barrier.clone();
            let state = state.clone();
            let id = id.clone();
            let activation = activation.clone();
            std::thread::spawn(move || {
                barrier.wait();
                run(
                    &state,
                    &["choose", &id, &activation, choice, "--revision", "1"],
                )
                .0
            })
        })
        .collect();
    barrier.wait();
    let mut codes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    codes.sort();
    assert_eq!(codes, vec![0, 3]);
    let (_, status) = run(&state, &["status", &id]);
    assert_eq!(status["data"]["view"]["revision"], 2);
}
#[test]
fn frozen_source_and_unsupported_run_version() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("runs");
    let source = temp.path().join("diagram.json");
    fs::copy(example("decision-tree.json"), &source).unwrap();
    let (_, started) = run(&state, &["start", source.to_str().unwrap()]);
    let id = started["data"]["view"]["runId"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::remove_file(&source).unwrap();
    let (_, status) = run(&state, &["status", &id]);
    assert_eq!(status["data"]["diagram"], started["data"]["diagram"]);
    let activation = input(&status, "understood");
    let (code, _) = run(
        &state,
        &["choose", &id, &activation, "unknown", "--revision", "1"],
    );
    assert_eq!(code, 0);
    let file = state.join(format!("{id}.json"));
    let mut stored: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    stored["runVersion"] = Value::String("999".into());
    fs::write(&file, serde_json::to_vec(&stored).unwrap()).unwrap();
    let (code, error) = run(&state, &["status", &id]);
    assert_eq!(code, 1);
    assert_eq!(error["error"]["code"], "unsupported-run-version");
}
#[test]
fn behavior_failure_is_a_successful_command() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("runs");
    let file = example("behavior-tree.json");
    let (_, started) = run(&state, &["start", file.to_str().unwrap()]);
    let id = started["data"]["view"]["runId"]
        .as_str()
        .unwrap()
        .to_owned();
    let ready = input(&started, "ready");
    let (code, continued) = run(
        &state,
        &["report", &id, &ready, "failure", "--revision", "1"],
    );
    assert_eq!(code, 0);
    let repair = input(&continued, "repair");
    let (code, finished) = run(
        &state,
        &["report", &id, &repair, "failure", "--revision", "2"],
    );
    assert_eq!(code, 0);
    assert_eq!(finished["data"]["view"]["outcome"]["status"], "failure");
}
#[test]
fn render_all_example_kinds_without_terminal_control() {
    for name in ["decision-tree", "behavior-tree", "activity-diagram"] {
        let yaml = example(&format!("{name}.yaml"));
        let json = example(&format!("{name}.json"));
        let (code, output) = invoke_text(&["render", yaml.to_str().unwrap()]);
        assert_eq!(code, 0);
        assert_eq!(output, invoke_text(&["render", json.to_str().unwrap()]).1);
        assert!(output.contains("Nodes:"));
        assert!(!output.contains('\u{1b}'));
        let (code, ascii) = invoke_text(&["render", yaml.to_str().unwrap(), "--ascii"]);
        assert_eq!(code, 0);
        assert!(
            ascii.is_ascii(),
            "ASCII rendering used non-ASCII characters"
        );
    }
    let activity = example("activity-diagram.yaml");
    let (_, output) = invoke_text(&["render", activity.to_str().unwrap()]);
    for id in [
        "prepare-entry:",
        "review:",
        "work:",
        "docs:",
        "checks:",
        "ready:",
        "release:",
    ] {
        assert!(output.contains(id), "missing {id}");
    }
    assert!(output.contains("review / revise:"));
    let (_, json_output, _) = invoke(&["--json", "render", activity.to_str().unwrap()]);
    assert!(json_output["data"]["text"]
        .as_str()
        .unwrap()
        .contains("Nodes:"));
}
#[test]
fn run_render_uses_frozen_snapshot_and_does_not_write() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("runs");
    let source = temp.path().join("diagram.json");
    fs::copy(example("activity-diagram.json"), &source).unwrap();
    let (_, started) = run(&state, &["start", source.to_str().unwrap()]);
    let id = started["data"]["view"]["runId"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::remove_file(&source).unwrap();
    let run_file = state.join(format!("{id}.json"));
    let before = fs::read(&run_file).unwrap();
    let (code, output) =
        invoke_text(&["run", "render", &id, "--state-dir", state.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(output.contains("▶ prepare:"));
    let prepare = input(&started, "prepare");
    assert!(output.contains(&format!("{prepare}  report  prepare")));
    assert_eq!(before, fs::read(&run_file).unwrap());

    let (_, after_report) = run(
        &state,
        &["report", &id, &prepare, "success", "--revision", "1"],
    );
    let review = input(&after_report, "review");
    let (_, after_choice) = run(
        &state,
        &["choose", &id, &review, "approve", "--revision", "2"],
    );
    let docs = input(&after_choice, "docs");
    let (_, _) = run(
        &state,
        &["report", &id, &docs, "success", "--revision", "3"],
    );
    let (_, waiting) = invoke_text(&["run", "render", &id, "--state-dir", state.to_str().unwrap()]);
    assert!(waiting.contains("◷ ready:"));
    assert!(waiting.contains("pending: work-checks"));
}
#[test]
fn render_escapes_control_characters_in_diagram_text() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("escape.json");
    let document = serde_json::json!({
        "formatVersion":"0.1","kind":"decision-tree","id":"escape",
        "title":format!("{}[31m", '\u{1b}'),
        "root":"done","nodes":[{"id":"done","type":"result","title":"Done","value":"done"}]
    });
    fs::write(&file, document.to_string()).unwrap();
    let (code, output) = invoke_text(&["render", file.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(!output.contains('\u{1b}'));
    assert!(output.contains("\\u{1b}"));
}
#[cfg(unix)]
#[test]
fn failed_atomic_write_preserves_previous_run() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("runs");
    let file = example("decision-tree.json");
    let (_, started) = run(&state, &["start", file.to_str().unwrap()]);
    let id = started["data"]["view"]["runId"]
        .as_str()
        .unwrap()
        .to_owned();
    let activation = input(&started, "understood");
    let run_file = state.join(format!("{id}.json"));
    let before = fs::read(&run_file).unwrap();
    let original = fs::metadata(&state).unwrap().permissions();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o500)).unwrap();
    let (code, failed) = run(
        &state,
        &["choose", &id, &activation, "known", "--revision", "1"],
    );
    fs::set_permissions(&state, original).unwrap();
    assert_eq!(code, 1);
    assert_eq!(failed["error"]["code"], "io-error");
    assert_eq!(before, fs::read(&run_file).unwrap());
}
