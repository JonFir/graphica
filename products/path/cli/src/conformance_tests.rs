use crate::{
    parse::load_diagram,
    runtime::{apply_choice, apply_report, Run},
    validate::{shape, validate_diagram},
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs,
    path::Path,
};

fn example(name: &str) -> Value {
    let text = match name {
        "decision" => include_str!("../../examples/decision-tree.json"),
        "behavior" => include_str!("../../examples/behavior-tree.json"),
        "activity" => include_str!("../../examples/activity-diagram.json"),
        _ => unreachable!(),
    };
    let value = load_diagram(text, "json").unwrap();
    validate_diagram(&value).unwrap();
    value
}
fn input(run: &Run, node: &str) -> String {
    run.available_inputs()
        .iter()
        .find(|i| i.node_id == node)
        .unwrap()
        .activation_id
        .clone()
}
#[test]
fn strict_encoding_and_shape() {
    for (text, encoding) in [
        (r#"{"id":"a","id":"b"}"#, "json"),
        (r#"{"id":NaN}"#, "json"),
        ("id: a\nid: b\n", "yaml"),
        ("nodes:\n  - id: a\n    id: b\n", "yaml"),
        ("id: &anchor a\n", "yaml"),
        ("id: *anchor\n", "yaml"),
        ("id: !!str a\n", "yaml"),
        ("<<: {id: a}\n", "yaml"),
        ("id: a\n---\nid: b\n", "yaml"),
        ("formatVersion: 0.1\n", "yaml"),
        ("title: true\n", "yaml"),
        ("title: null\n", "yaml"),
        ("title: 2026-11-02\n", "yaml"),
        ("title: 1:20\n", "yaml"),
        ("1: x\n", "yaml"),
    ] {
        assert!(load_diagram(text, encoding).is_err(), "{text}");
    }
    assert_eq!(
        load_diagram("title: \"true\"\n", "yaml").unwrap()["title"],
        "true"
    );
    let mut diagram = example("decision");
    diagram["nodes"][0]["unexpected"] = json!("x");
    assert_eq!(
        validate_diagram(&diagram).unwrap_err()[0].code,
        "invalid-shape"
    );
}
#[test]
fn schema_shape_cases() {
    let schema: Value =
        serde_json::from_str(include_str!("../../format/path.schema.json")).unwrap();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert!(schema["$defs"].is_object());
    let cases: Value =
        serde_json::from_str(include_str!("../../format/schema-cases.json")).unwrap();
    let cases = cases.as_array().unwrap();
    assert!(!cases.is_empty());
    let mut names = HashSet::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert!(names.insert(name), "duplicate fixture: {name}");
        let diagnostics = shape(&case["document"]);
        assert_eq!(
            diagnostics.is_empty(),
            case["valid"].as_bool().unwrap(),
            "{name}: {diagnostics:?}"
        );
    }
}
#[test]
fn example_pairs_and_node_coverage() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let mut json_names = BTreeSet::new();
    let mut yaml_names = BTreeSet::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        match path.extension().and_then(|s| s.to_str()) {
            Some("json") => {
                json_names.insert(path.file_stem().unwrap().to_str().unwrap().to_owned());
            }
            Some("yaml") => {
                yaml_names.insert(path.file_stem().unwrap().to_str().unwrap().to_owned());
            }
            _ => (),
        }
    }
    assert!(!json_names.is_empty());
    assert_eq!(
        json_names, yaml_names,
        "every JSON example needs an equivalent YAML file"
    );
    let expected: HashMap<&str, HashSet<&str>> = HashMap::from([
        ("decision-tree", HashSet::from(["question", "result"])),
        (
            "behavior-tree",
            HashSet::from(["sequence", "selector", "action", "condition"]),
        ),
        (
            "activity-diagram",
            HashSet::from([
                "initial", "action", "decision", "merge", "fork", "join", "final",
            ]),
        ),
    ]);
    let mut coverage: HashMap<String, HashSet<String>> = HashMap::new();
    for name in json_names {
        let json_text = fs::read_to_string(dir.join(format!("{name}.json"))).unwrap();
        let yaml_text = fs::read_to_string(dir.join(format!("{name}.yaml"))).unwrap();
        let json = load_diagram(&json_text, "json").unwrap();
        let yaml = load_diagram(&yaml_text, "yaml").unwrap();
        assert_eq!(json, yaml, "example pair {name} differs");
        validate_diagram(&json).unwrap();
        validate_diagram(&yaml).unwrap();
        let kind = json["kind"].as_str().unwrap();
        coverage.entry(kind.to_owned()).or_default().extend(
            json["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|node| node["type"].as_str().unwrap().to_owned()),
        );
    }
    for (kind, required) in expected {
        let actual = coverage
            .get(kind)
            .unwrap_or_else(|| panic!("missing example for {kind}"));
        assert_eq!(
            actual,
            &required.into_iter().map(str::to_owned).collect(),
            "missing node coverage for {kind}"
        );
    }
}
#[test]
fn tree_graph_diagnostics() {
    let mut d = example("decision");
    d["nodes"][1]["id"] = json!("understood");
    assert!(validate_diagram(&d)
        .unwrap_err()
        .iter()
        .any(|e| e.code == "duplicate-id"));
    let mut d = example("decision");
    d["nodes"][0]["choices"][0]["target"] = json!("missing");
    assert!(validate_diagram(&d)
        .unwrap_err()
        .iter()
        .any(|e| e.code == "unknown-reference"));
    let mut d = example("decision");
    d["nodes"][0]["choices"][0]["target"] = json!("understood");
    assert!(validate_diagram(&d)
        .unwrap_err()
        .iter()
        .any(|e| e.code == "invalid-tree"));
}
#[test]
fn nested_parallel_regions_and_crossing() {
    let d = json!({
        "formatVersion":"0.1","kind":"activity-diagram","id":"nested","title":"Nested",
        "entry":"start",
        "nodes":[
            {"id":"start","type":"initial","title":"Start"},
            {"id":"outer","type":"fork","title":"Outer","join":"outerjoin"},
            {"id":"inner","type":"fork","title":"Inner","join":"innerjoin"},
            {"id":"a","type":"action","title":"A"},{"id":"b","type":"action","title":"B"},
            {"id":"innerjoin","type":"join","title":"Inner join","fork":"inner"},
            {"id":"after","type":"action","title":"After"},
            {"id":"other","type":"action","title":"Other"},
            {"id":"outerjoin","type":"join","title":"Outer join","fork":"outer"},
            {"id":"done","type":"final","title":"Done"}
        ],
        "edges":[
            {"id":"e1","source":"start","target":"outer"},
            {"id":"e2","source":"outer","target":"inner"},
            {"id":"e3","source":"outer","target":"other"},
            {"id":"e4","source":"inner","target":"a"},
            {"id":"e5","source":"inner","target":"b"},
            {"id":"e6","source":"a","target":"innerjoin"},
            {"id":"e7","source":"b","target":"innerjoin"},
            {"id":"e8","source":"innerjoin","target":"after"},
            {"id":"e9","source":"after","target":"outerjoin"},
            {"id":"e10","source":"other","target":"outerjoin"},
            {"id":"e11","source":"outerjoin","target":"done"}
        ]
    });
    validate_diagram(&d).unwrap();
    let mut run = Run::start(d.clone(), "nested-run".into()).unwrap();
    let other = input(&run, "other");
    apply_report(&mut run, 1, &other, "success", None).unwrap();
    assert_eq!(run.view().waiting_joins.len(), 1);
    let a = input(&run, "a");
    apply_report(&mut run, 2, &a, "success", None).unwrap();
    assert_eq!(run.view().waiting_joins.len(), 2);
    let b = input(&run, "b");
    apply_report(&mut run, 3, &b, "success", None).unwrap();
    let after = input(&run, "after");
    apply_report(&mut run, 4, &after, "success", None).unwrap();
    assert_eq!(run.outcome.as_ref().unwrap()["nodeId"], "done");
    assert_eq!(
        run.activations
            .iter()
            .filter(|a| a.node_id == "innerjoin")
            .count(),
        1
    );
    assert_eq!(
        run.activations
            .iter()
            .filter(|a| a.node_id == "outerjoin")
            .count(),
        1
    );
    let mut bad = d.clone();
    bad["edges"][5]["target"] = json!("outerjoin");
    assert!(validate_diagram(&bad)
        .unwrap_err()
        .iter()
        .any(|e| e.code == "invalid-parallel-region"));
}
#[test]
fn direct_branch_and_automatic_cycle() {
    let mut d = json!({
        "formatVersion":"0.1","kind":"activity-diagram","id":"direct","title":"Direct",
        "entry":"start",
        "nodes":[
            {"id":"start","type":"initial","title":"Start"},
            {"id":"fork","type":"fork","title":"Fork","join":"join"},
            {"id":"action","type":"action","title":"Action"},
            {"id":"join","type":"join","title":"Join","fork":"fork"},
            {"id":"done","type":"final","title":"Done"}
        ],
        "edges":[
            {"id":"e1","source":"start","target":"fork"},
            {"id":"direct","source":"fork","target":"join"},
            {"id":"branch","source":"fork","target":"action"},
            {"id":"arrival","source":"action","target":"join"},
            {"id":"e5","source":"join","target":"done"}
        ]
    });
    validate_diagram(&d).unwrap();
    let mut run = Run::start(d.clone(), "direct-run".into()).unwrap();
    assert_eq!(run.view().waiting_joins.len(), 1);
    let action = input(&run, "action");
    apply_report(&mut run, 1, &action, "success", None).unwrap();
    assert_eq!(run.outcome.as_ref().unwrap()["nodeId"], "done");

    d["nodes"]
        .as_array_mut()
        .unwrap()
        .insert(1, json!({"id":"merge","type":"merge","title":"Merge"}));
    d["edges"][0]["target"] = json!("merge");
    d["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"loop","source":"merge","target":"fork"}));
    d["edges"][4]["target"] = json!("merge");
    assert!(validate_diagram(&d)
        .unwrap_err()
        .iter()
        .any(|e| e.code == "automatic-cycle"));
}
#[test]
fn decision_and_behavior_runs() {
    let result_root = json!({
        "formatVersion":"0.1","kind":"decision-tree","id":"root-result","title":"Root result",
        "root":"done","nodes":[{"id":"done","type":"result","title":"Done","value":"done"}]
    });
    let completed = Run::start(result_root, "root-run".into()).unwrap();
    assert_eq!(completed.revision, 1);
    assert_eq!(completed.status, "completed");
    assert!(completed.available_inputs().is_empty());

    let mut run = Run::start(example("decision"), "decision-run".into()).unwrap();
    let a = input(&run, "understood");
    apply_choice(&mut run, 1, &a, "known", None).unwrap();
    assert_eq!(run.revision, 2);
    assert_eq!(
        apply_choice(&mut run, 1, &a, "known", None)
            .unwrap_err()
            .code,
        "revision-conflict"
    );
    let b = input(&run, "covered");
    apply_choice(&mut run, 2, &b, "untested", Some("no regression test")).unwrap();
    assert_eq!(run.outcome.as_ref().unwrap()["value"], "add-test");
    assert_eq!(
        apply_choice(&mut run, 3, &b, "tested", None)
            .unwrap_err()
            .code,
        "run-completed"
    );
    assert!(run
        .history
        .iter()
        .enumerate()
        .all(|(i, e)| e["sequence"] == i as u64 + 1));

    let mut run = Run::start(example("behavior"), "behavior-run".into()).unwrap();
    let ready = input(&run, "ready");
    assert_eq!(
        apply_report(&mut run, 1, &ready, "running", None)
            .unwrap_err()
            .code,
        "invalid-report"
    );
    apply_report(&mut run, 1, &ready, "failure", None).unwrap();
    let repair = input(&run, "repair");
    apply_report(&mut run, 2, &repair, "running", None).unwrap();
    let snapshot = serde_json::to_vec(&run).unwrap();
    let mut run: Run = serde_json::from_slice(&snapshot).unwrap();
    run.check_stored().unwrap();
    assert_eq!(input(&run, "repair"), repair);
    apply_report(&mut run, 3, &repair, "success", None).unwrap();
    let publish = input(&run, "publish");
    apply_report(&mut run, 4, &publish, "success", None).unwrap();
    assert_eq!(run.outcome.as_ref().unwrap()["status"], "success");

    let mut failed = Run::start(example("behavior"), "failed-run".into()).unwrap();
    let ready = input(&failed, "ready");
    apply_report(&mut failed, 1, &ready, "failure", None).unwrap();
    let repair = input(&failed, "repair");
    apply_report(&mut failed, 2, &repair, "failure", None).unwrap();
    assert_eq!(failed.outcome.as_ref().unwrap()["status"], "failure");
}
#[test]
fn activity_join_and_fresh_loop_activations() {
    let mut run = Run::start(example("activity"), "activity-run".into()).unwrap();
    let prepare = input(&run, "prepare");
    apply_report(&mut run, 1, &prepare, "success", None).unwrap();
    let review = input(&run, "review");
    apply_choice(&mut run, 2, &review, "revise", None).unwrap();
    assert_ne!(prepare, input(&run, "prepare"));
    let prepare = input(&run, "prepare");
    apply_report(&mut run, 3, &prepare, "success", None).unwrap();
    let review = input(&run, "review");
    apply_choice(&mut run, 4, &review, "approve", None).unwrap();
    let fork_completion = run
        .history
        .iter()
        .find(|event| event["type"] == "node-completed" && event["nodeId"] == "work")
        .unwrap()["sequence"]
        .as_u64()
        .unwrap();
    for branch in ["docs", "checks"] {
        let entered = run
            .history
            .iter()
            .find(|event| event["type"] == "node-entered" && event["nodeId"] == branch)
            .unwrap();
        assert_eq!(entered["causalEventSequence"], fork_completion, "{branch}");
    }
    let docs = input(&run, "docs");
    let checks = input(&run, "checks");
    apply_report(&mut run, 5, &docs, "success", None).unwrap();
    assert_eq!(run.view().waiting_joins.len(), 1);
    assert_eq!(run.view().waiting_joins[0].arrived_branch_ids.len(), 1);
    let snapshot = serde_json::to_vec(&run).unwrap();
    let mut run: Run = serde_json::from_slice(&snapshot).unwrap();
    apply_report(&mut run, 6, &checks, "success", None).unwrap();
    assert_eq!(run.view().waiting_joins.len(), 0);
    assert_eq!(
        run.activations
            .iter()
            .filter(|a| a.node_id == "ready")
            .count(),
        1
    );
    let release = input(&run, "release");
    apply_choice(&mut run, 7, &release, "ship", None).unwrap();
    let publish = input(&run, "publish");
    apply_report(&mut run, 8, &publish, "success", None).unwrap();
    assert_eq!(run.outcome.as_ref().unwrap()["nodeId"], "done");
}
#[test]
fn activity_reversed_arrivals_and_repeated_fork() {
    let mut run = Run::start(example("activity"), "repeat-run".into()).unwrap();
    let prepare = input(&run, "prepare");
    apply_report(&mut run, 1, &prepare, "success", None).unwrap();
    let review = input(&run, "review");
    apply_choice(&mut run, 2, &review, "approve", None).unwrap();
    let checks = input(&run, "checks");
    apply_report(&mut run, 3, &checks, "success", None).unwrap();
    assert_eq!(
        run.view().waiting_joins[0].arrived_branch_ids,
        vec!["work-checks"]
    );
    let docs = input(&run, "docs");
    apply_report(&mut run, 4, &docs, "success", None).unwrap();
    let release = input(&run, "release");
    apply_choice(&mut run, 5, &release, "retry", None).unwrap();
    let prepare_again = input(&run, "prepare");
    assert_ne!(prepare, prepare_again);
    apply_report(&mut run, 6, &prepare_again, "success", None).unwrap();
    let review = input(&run, "review");
    apply_choice(&mut run, 7, &review, "approve", None).unwrap();
    let docs = input(&run, "docs");
    apply_report(&mut run, 8, &docs, "success", None).unwrap();
    assert_eq!(run.view().waiting_joins[0].fork_instance_id, "f2");
    assert_eq!(
        run.view().waiting_joins[0].arrived_branch_ids,
        vec!["work-docs"]
    );
    let checks = input(&run, "checks");
    apply_report(&mut run, 9, &checks, "success", None).unwrap();
    assert_eq!(run.view().waiting_joins.len(), 0);
    assert_eq!(
        run.activations
            .iter()
            .filter(|a| a.node_id == "ready")
            .count(),
        2
    );
}
