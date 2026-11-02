use crate::parse::{diag, Diagnostic};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) fn str_field<'a>(value: &'a Value, name: &str) -> &'a str {
    value.get(name).and_then(Value::as_str).unwrap_or("")
}
pub(crate) fn nodes(diagram: &Value) -> &[Value] {
    diagram["nodes"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
pub(crate) fn edges(diagram: &Value) -> &[Value] {
    diagram["edges"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
pub(crate) fn node<'a>(diagram: &'a Value, id: &str) -> &'a Value {
    nodes(diagram)
        .iter()
        .find(|n| str_field(n, "id") == id)
        .expect("validated node reference")
}
pub(crate) fn outgoing<'a>(diagram: &'a Value, id: &str) -> Vec<&'a Value> {
    edges(diagram)
        .iter()
        .filter(|e| str_field(e, "source") == id)
        .collect()
}

fn pointer(base: &str, field: &str) -> String {
    format!("{base}/{field}")
}
fn string_field(
    value: &Value,
    key: &str,
    base: &str,
    required: bool,
    nonempty: bool,
    errors: &mut Vec<Diagnostic>,
) {
    match value.get(key) {
        None if required => errors.push(diag(
            "invalid-shape",
            format!("missing {key}"),
            pointer(base, key),
        )),
        None => (),
        Some(Value::String(s)) if !nonempty || !s.is_empty() => (),
        Some(_) => errors.push(diag(
            "invalid-shape",
            format!(
                "{key} must be {}string",
                if nonempty { "a nonempty " } else { "a " }
            ),
            pointer(base, key),
        )),
    }
}
fn identifier(value: &Value, key: &str, base: &str, errors: &mut Vec<Diagnostic>) {
    string_field(value, key, base, true, true, errors);
    if let Some(s) = value.get(key).and_then(Value::as_str) {
        if !valid_identifier(s) {
            errors.push(diag(
                "invalid-shape",
                format!("{key} is not a valid identifier"),
                pointer(base, key),
            ));
        }
    }
}
fn valid_identifier(s: &str) -> bool {
    s.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && s.as_bytes()
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
}
fn object<'a>(
    value: &'a Value,
    base: &str,
    errors: &mut Vec<Diagnostic>,
) -> Option<&'a serde_json::Map<String, Value>> {
    match value.as_object() {
        Some(o) => Some(o),
        None => {
            errors.push(diag("invalid-shape", "expected an object", base));
            None
        }
    }
}
fn fields(value: &Value, allowed: &[&str], base: &str, errors: &mut Vec<Diagnostic>) {
    if let Some(o) = object(value, base, errors) {
        for key in o.keys() {
            if !allowed.contains(&key.as_str()) {
                errors.push(diag(
                    "invalid-shape",
                    format!("unknown field {key}"),
                    pointer(base, key),
                ));
            }
        }
    }
}
fn array<'a>(
    value: &'a Value,
    key: &str,
    base: &str,
    errors: &mut Vec<Diagnostic>,
) -> Option<&'a Vec<Value>> {
    match value.get(key).and_then(Value::as_array) {
        Some(a) if !a.is_empty() => Some(a),
        _ => {
            errors.push(diag(
                "invalid-shape",
                format!("{key} must be a nonempty array"),
                pointer(base, key),
            ));
            None
        }
    }
}
pub(crate) fn shape(diagram: &Value) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    if object(diagram, "", &mut errors).is_none() {
        return errors;
    }
    let kind = str_field(diagram, "kind");
    let extra = match kind {
        "decision-tree" | "behavior-tree" => vec!["root"],
        "activity-diagram" => vec!["entry", "edges"],
        _ => {
            errors.push(diag("invalid-shape", "unknown diagram kind", "/kind"));
            Vec::new()
        }
    };
    let mut allowed = vec![
        "formatVersion",
        "kind",
        "id",
        "title",
        "description",
        "nodes",
    ];
    allowed.extend(extra);
    fields(diagram, &allowed, "", &mut errors);
    if str_field(diagram, "formatVersion") != "0.1" {
        errors.push(diag(
            "unsupported-format-version",
            "formatVersion must be 0.1",
            "/formatVersion",
        ));
    }
    string_field(diagram, "kind", "", true, true, &mut errors);
    identifier(diagram, "id", "", &mut errors);
    string_field(diagram, "title", "", true, true, &mut errors);
    string_field(diagram, "description", "", false, false, &mut errors);
    match kind {
        "decision-tree" | "behavior-tree" => identifier(diagram, "root", "", &mut errors),
        "activity-diagram" => identifier(diagram, "entry", "", &mut errors),
        _ => (),
    }
    if let Some(all) = array(diagram, "nodes", "", &mut errors) {
        for (i, n) in all.iter().enumerate() {
            let base = format!("/nodes/{i}");
            if object(n, &base, &mut errors).is_none() {
                continue;
            }
            let t = str_field(n, "type");
            let extra: &[&str] = match (kind, t) {
                ("decision-tree", "question") => &["choices"],
                ("decision-tree", "result") => &["value"],
                ("behavior-tree", "sequence" | "selector") => &["children"],
                ("behavior-tree", "action" | "condition") => &[],
                ("activity-diagram", "fork") => &["join"],
                ("activity-diagram", "join") => &["fork"],
                ("activity-diagram", "initial" | "action" | "decision" | "merge" | "final") => &[],
                _ => {
                    errors.push(diag(
                        "invalid-shape",
                        format!("unsupported node type {t} for {kind}"),
                        pointer(&base, "type"),
                    ));
                    &[]
                }
            };
            let mut allowed = vec!["id", "type", "title", "description"];
            allowed.extend_from_slice(extra);
            fields(n, &allowed, &base, &mut errors);
            identifier(n, "id", &base, &mut errors);
            string_field(n, "type", &base, true, true, &mut errors);
            string_field(n, "title", &base, true, true, &mut errors);
            string_field(n, "description", &base, false, false, &mut errors);
            match (kind, t) {
                ("decision-tree", "question") => {
                    if let Some(a) = array(n, "choices", &base, &mut errors) {
                        if a.len() < 2 {
                            errors.push(diag(
                                "invalid-shape",
                                "question needs at least two choices",
                                pointer(&base, "choices"),
                            ));
                        }
                        for (j, c) in a.iter().enumerate() {
                            let p = format!("{base}/choices/{j}");
                            fields(c, &["id", "label", "target"], &p, &mut errors);
                            identifier(c, "id", &p, &mut errors);
                            string_field(c, "label", &p, true, true, &mut errors);
                            identifier(c, "target", &p, &mut errors);
                        }
                    }
                }
                ("decision-tree", "result") => {
                    string_field(n, "value", &base, true, true, &mut errors)
                }
                ("behavior-tree", "sequence" | "selector") => {
                    if let Some(a) = array(n, "children", &base, &mut errors) {
                        let mut seen = HashSet::new();
                        for (j, c) in a.iter().enumerate() {
                            let p = format!("{base}/children/{j}");
                            if let Some(child) = c.as_str() {
                                if !valid_identifier(child) {
                                    errors.push(diag(
                                        "invalid-shape",
                                        "child must be an identifier",
                                        &p,
                                    ));
                                }
                                if !seen.insert(child) {
                                    errors.push(diag(
                                        "invalid-shape",
                                        "repeated child reference",
                                        &p,
                                    ));
                                }
                            } else {
                                errors.push(diag(
                                    "invalid-shape",
                                    "child must be an identifier",
                                    &p,
                                ));
                            }
                        }
                    }
                }
                ("activity-diagram", "fork") => identifier(n, "join", &base, &mut errors),
                ("activity-diagram", "join") => identifier(n, "fork", &base, &mut errors),
                _ => (),
            }
        }
    }
    if kind == "activity-diagram" {
        if let Some(all) = array(diagram, "edges", "", &mut errors) {
            for (i, e) in all.iter().enumerate() {
                let p = format!("/edges/{i}");
                fields(e, &["id", "source", "target", "label"], &p, &mut errors);
                identifier(e, "id", &p, &mut errors);
                identifier(e, "source", &p, &mut errors);
                identifier(e, "target", &p, &mut errors);
                string_field(e, "label", &p, false, true, &mut errors);
            }
        }
    }
    errors
}

fn reachable(start: &str, adj: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut q = VecDeque::from([start.to_owned()]);
    while let Some(id) = q.pop_front() {
        if seen.insert(id.clone()) {
            if let Some(targets) = adj.get(&id) {
                q.extend(targets.iter().cloned());
            }
        }
    }
    seen
}
fn graph_tree(diagram: &Value, errors: &mut Vec<Diagnostic>) {
    let all = nodes(diagram);
    let root = str_field(diagram, "root");
    let ids: HashSet<&str> = all.iter().map(|n| str_field(n, "id")).collect();
    if !ids.contains(root) {
        errors.push(diag("unknown-reference", "unknown root", "/root"));
    }
    let mut incoming: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (i, n) in all.iter().enumerate() {
        let t = str_field(n, "type");
        if t == "question" {
            let mut choice_ids = HashSet::new();
            let mut targets = HashSet::new();
            for (j, choice) in n["choices"].as_array().unwrap().iter().enumerate() {
                let id = str_field(choice, "id");
                let target = str_field(choice, "target");
                if !choice_ids.insert(id) {
                    errors.push(diag(
                        "duplicate-id",
                        format!("duplicate choice ID {id}"),
                        format!("/nodes/{i}/choices/{j}/id"),
                    ));
                }
                if !targets.insert(target) {
                    errors.push(diag(
                        "invalid-tree",
                        "choices must have distinct targets",
                        format!("/nodes/{i}/choices/{j}/target"),
                    ));
                }
                if !ids.contains(target) {
                    errors.push(diag(
                        "unknown-reference",
                        format!("unknown target {target}"),
                        format!("/nodes/{i}/choices/{j}/target"),
                    ));
                }
                adj.entry(str_field(n, "id").into())
                    .or_default()
                    .push(target.into());
                *incoming.entry(target.into()).or_default() += 1;
            }
        } else if t == "sequence" || t == "selector" {
            for (j, child) in n["children"].as_array().unwrap().iter().enumerate() {
                let child = child.as_str().unwrap();
                if !ids.contains(child) {
                    errors.push(diag(
                        "unknown-reference",
                        format!("unknown child {child}"),
                        format!("/nodes/{i}/children/{j}"),
                    ));
                }
                adj.entry(str_field(n, "id").into())
                    .or_default()
                    .push(child.into());
                *incoming.entry(child.into()).or_default() += 1;
            }
        }
    }
    for (i, n) in all.iter().enumerate() {
        let id = str_field(n, "id");
        let count = incoming.get(id).copied().unwrap_or(0);
        if (id == root && count != 0) || (id != root && count != 1) {
            errors.push(diag(
                "invalid-tree",
                format!("node {id} has {count} parents"),
                format!("/nodes/{i}/id"),
            ));
        }
    }
    if ids.contains(root) {
        let seen = reachable(root, &adj);
        for (i, n) in all.iter().enumerate() {
            if !seen.contains(str_field(n, "id")) {
                errors.push(diag(
                    "unreachable-node",
                    "node is unreachable",
                    format!("/nodes/{i}"),
                ));
            }
        }
    }
}

fn automatic_cycle(diagram: &Value) -> bool {
    let auto: HashSet<&str> = nodes(diagram)
        .iter()
        .filter(|n| !matches!(str_field(n, "type"), "action" | "decision"))
        .map(|n| str_field(n, "id"))
        .collect();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for e in edges(diagram) {
        let (a, b) = (str_field(e, "source"), str_field(e, "target"));
        if auto.contains(a) && auto.contains(b) {
            adj.entry(a.into()).or_default().push(b.into());
        }
    }
    fn visit(
        id: &str,
        adj: &HashMap<String, Vec<String>>,
        gray: &mut HashSet<String>,
        black: &mut HashSet<String>,
    ) -> bool {
        if gray.contains(id) {
            return true;
        }
        if black.contains(id) {
            return false;
        }
        gray.insert(id.into());
        for target in adj.get(id).into_iter().flatten() {
            if visit(target, adj, gray, black) {
                return true;
            }
        }
        gray.remove(id);
        black.insert(id.into());
        false
    }
    let mut gray = HashSet::new();
    let mut black = HashSet::new();
    auto.iter().any(|id| visit(id, &adj, &mut gray, &mut black))
}

fn graph_activity(diagram: &Value, errors: &mut Vec<Diagnostic>) {
    let all = nodes(diagram);
    let ids: HashSet<&str> = all.iter().map(|n| str_field(n, "id")).collect();
    let entry = str_field(diagram, "entry");
    if !ids.contains(entry) {
        errors.push(diag("unknown-reference", "unknown entry", "/entry"));
    }
    let mut edge_ids = HashSet::new();
    let mut pairs = HashSet::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut rev: HashMap<String, Vec<String>> = HashMap::new();
    let mut indeg: HashMap<String, usize> = HashMap::new();
    let mut outdeg: HashMap<String, usize> = HashMap::new();
    for (i, e) in edges(diagram).iter().enumerate() {
        let (id, source, target) = (
            str_field(e, "id"),
            str_field(e, "source"),
            str_field(e, "target"),
        );
        if !edge_ids.insert(id) {
            errors.push(diag(
                "duplicate-id",
                format!("duplicate edge ID {id}"),
                format!("/edges/{i}/id"),
            ));
        }
        if !ids.contains(source) {
            errors.push(diag(
                "unknown-reference",
                format!("unknown source {source}"),
                format!("/edges/{i}/source"),
            ));
        }
        if !ids.contains(target) {
            errors.push(diag(
                "unknown-reference",
                format!("unknown target {target}"),
                format!("/edges/{i}/target"),
            ));
        }
        if !pairs.insert((source, target)) {
            errors.push(diag(
                "duplicate-edge",
                "duplicate source/target edge",
                format!("/edges/{i}"),
            ));
        }
        adj.entry(source.into()).or_default().push(target.into());
        rev.entry(target.into()).or_default().push(source.into());
        *indeg.entry(target.into()).or_default() += 1;
        *outdeg.entry(source.into()).or_default() += 1;
        if ids.contains(source)
            && str_field(node(diagram, source), "type") == "decision"
            && str_field(e, "label").is_empty()
        {
            errors.push(diag(
                "missing-choice-label",
                "decision edge needs a label",
                format!("/edges/{i}/label"),
            ));
        }
    }
    let initial_count = all
        .iter()
        .filter(|n| str_field(n, "type") == "initial")
        .count();
    let final_count = all
        .iter()
        .filter(|n| str_field(n, "type") == "final")
        .count();
    if initial_count != 1
        || final_count == 0
        || !ids.contains(entry)
        || (ids.contains(entry) && str_field(node(diagram, entry), "type") != "initial")
    {
        errors.push(diag(
            "invalid-entry-or-final",
            "activity needs one initial entry and at least one final",
            "/entry",
        ));
    }
    for (i, n) in all.iter().enumerate() {
        let id = str_field(n, "id");
        let (input, output) = (
            indeg.get(id).copied().unwrap_or(0),
            outdeg.get(id).copied().unwrap_or(0),
        );
        let valid = match str_field(n, "type") {
            "initial" => input == 0 && output == 1,
            "action" => input == 1 && output == 1,
            "decision" => input == 1 && output >= 2,
            "merge" => input >= 2 && output == 1,
            "fork" => input == 1 && output >= 2,
            "join" => input >= 2 && output == 1,
            "final" => input >= 1 && output == 0,
            _ => false,
        };
        if !valid {
            errors.push(diag(
                "invalid-degree",
                format!("invalid edge degree for {id}: {input} incoming, {output} outgoing"),
                format!("/nodes/{i}"),
            ));
        }
        if str_field(n, "type") == "fork" {
            let join = str_field(n, "join");
            if !ids.contains(join) {
                errors.push(diag(
                    "unknown-reference",
                    format!("unknown join {join}"),
                    format!("/nodes/{i}/join"),
                ));
            } else if str_field(node(diagram, join), "type") != "join"
                || str_field(node(diagram, join), "fork") != id
            {
                errors.push(diag(
                    "invalid-parallel-region",
                    "fork/join pair is not reciprocal",
                    format!("/nodes/{i}/join"),
                ));
            }
        }
        if str_field(n, "type") == "join" {
            let fork = str_field(n, "fork");
            if !ids.contains(fork) {
                errors.push(diag(
                    "unknown-reference",
                    format!("unknown fork {fork}"),
                    format!("/nodes/{i}/fork"),
                ));
            } else if str_field(node(diagram, fork), "type") != "fork"
                || str_field(node(diagram, fork), "join") != id
            {
                errors.push(diag(
                    "invalid-parallel-region",
                    "join/fork pair is not reciprocal",
                    format!("/nodes/{i}/fork"),
                ));
            }
        }
    }
    if ids.contains(entry) {
        let seen = reachable(entry, &adj);
        for (i, n) in all.iter().enumerate() {
            if !seen.contains(str_field(n, "id")) {
                errors.push(diag(
                    "unreachable-node",
                    "node is unreachable",
                    format!("/nodes/{i}"),
                ));
            }
        }
    }
    let finals: Vec<_> = all
        .iter()
        .filter(|n| str_field(n, "type") == "final")
        .map(|n| str_field(n, "id"))
        .collect();
    let mut to_final = HashSet::new();
    for final_id in finals {
        to_final.extend(reachable(final_id, &rev));
    }
    for (i, n) in all.iter().enumerate() {
        if !to_final.contains(str_field(n, "id")) {
            errors.push(diag(
                "no-final-path",
                "node has no path to a final",
                format!("/nodes/{i}"),
            ));
        }
    }
    if errors.iter().any(|e| e.code == "unknown-reference") {
        return;
    }
    if automatic_cycle(diagram) {
        errors.push(diag(
            "automatic-cycle",
            "automatic activity nodes contain a cycle",
            "/edges",
        ));
    }
    parallel_regions(diagram, errors);
}

fn parallel_regions(diagram: &Value, errors: &mut Vec<Diagnostic>) {
    let mut regions: Vec<(String, HashSet<String>)> = Vec::new();
    for (i, fork) in nodes(diagram)
        .iter()
        .enumerate()
        .filter(|(_, n)| str_field(n, "type") == "fork")
    {
        let fid = str_field(fork, "id");
        let jid = str_field(fork, "join");
        if !nodes(diagram)
            .iter()
            .any(|n| str_field(n, "id") == jid && str_field(n, "type") == "join")
        {
            continue;
        }
        let starts = outgoing(diagram, fid);
        let arrivals: Vec<_> = edges(diagram)
            .iter()
            .filter(|e| str_field(e, "target") == jid)
            .collect();
        if starts.len() != arrivals.len() {
            errors.push(diag(
                "invalid-parallel-region",
                "fork branch and join arrival counts differ",
                format!("/nodes/{i}"),
            ));
        }
        let mut all_region = HashSet::new();
        let mut branch_sets = Vec::<HashSet<String>>::new();
        let mut valid = true;
        for start in &starts {
            let start_id = str_field(start, "target");
            let mut seen = HashSet::new();
            let mut queue = VecDeque::from([start_id.to_owned()]);
            while let Some(current) = queue.pop_front() {
                if current == jid {
                    continue;
                }
                if current == fid || str_field(node(diagram, &current), "type") == "final" {
                    valid = false;
                    continue;
                }
                if !seen.insert(current.clone()) {
                    continue;
                }
                for edge in outgoing(diagram, &current) {
                    queue.push_back(str_field(edge, "target").into());
                }
            }
            if !all_region.is_disjoint(&seen) {
                valid = false;
            }
            all_region.extend(seen.iter().cloned());
            branch_sets.push(seen);
        }
        let mut used_arrivals = HashSet::new();
        for (branch, set) in branch_sets.iter().enumerate() {
            let start_edge = str_field(starts[branch], "id");
            let arrivals_for_branch: Vec<_> = arrivals
                .iter()
                .filter(|e| {
                    set.contains(str_field(e, "source"))
                        || (str_field(e, "id") == start_edge && str_field(e, "source") == fid)
                })
                .collect();
            if arrivals_for_branch.len() != 1 {
                valid = false;
            }
            for e in arrivals_for_branch {
                used_arrivals.insert(str_field(e, "id"));
            }
            for current in set {
                for e in edges(diagram)
                    .iter()
                    .filter(|e| str_field(e, "target") == current)
                {
                    let source = str_field(e, "source");
                    if !(set.contains(source)
                        || (str_field(e, "id") == start_edge && source == fid))
                    {
                        valid = false;
                    }
                }
                for e in outgoing(diagram, current) {
                    let target = str_field(e, "target");
                    if target != jid && !set.contains(target) {
                        valid = false;
                    }
                }
            }
            let reverse: HashMap<String, Vec<String>> = edges(diagram)
                .iter()
                .filter(|e| {
                    set.contains(str_field(e, "source"))
                        && (set.contains(str_field(e, "target")) || str_field(e, "target") == jid)
                })
                .fold(HashMap::new(), |mut a, e| {
                    a.entry(str_field(e, "target").into())
                        .or_default()
                        .push(str_field(e, "source").into());
                    a
                });
            let can_finish = reachable(jid, &reverse);
            if !set.iter().all(|id| can_finish.contains(id)) {
                valid = false;
            }
        }
        if used_arrivals.len() != arrivals.len() {
            valid = false;
        }
        if !valid {
            errors.push(diag(
                "invalid-parallel-region",
                "parallel branches must be disjoint, enclosed, and join once each",
                format!("/nodes/{i}"),
            ));
        }
        all_region.insert(fid.into());
        all_region.insert(jid.into());
        regions.push((fid.into(), all_region));
    }
    for a in 0..regions.len() {
        for b in a + 1..regions.len() {
            let x = &regions[a].1;
            let y = &regions[b].1;
            if !x.is_disjoint(y) && !x.is_subset(y) && !y.is_subset(x) {
                errors.push(diag(
                    "invalid-parallel-region",
                    format!(
                        "parallel regions {} and {} overlap",
                        regions[a].0, regions[b].0
                    ),
                    "/nodes",
                ));
            }
        }
    }
}

pub fn validate_diagram(diagram: &Value) -> Result<(), Vec<Diagnostic>> {
    let mut errors = shape(diagram);
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut ids = HashSet::new();
    for (i, n) in nodes(diagram).iter().enumerate() {
        if !ids.insert(str_field(n, "id")) {
            errors.push(diag(
                "duplicate-id",
                "duplicate node ID",
                format!("/nodes/{i}/id"),
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    match str_field(diagram, "kind") {
        "decision-tree" | "behavior-tree" => graph_tree(diagram, &mut errors),
        "activity-diagram" => graph_activity(diagram, &mut errors),
        _ => unreachable!(),
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
