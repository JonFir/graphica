use std::collections::HashMap;

use ascii_dag::{Graph, RenderOptions};
use serde_json::Value;

use crate::{
    runtime::RunView,
    validate::{nodes, str_field},
};

const TITLE_LIMIT: usize = 28;

fn safe_text(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| {
            if ch.is_control() {
                ch.escape_default().collect::<Vec<_>>()
            } else {
                vec![ch]
            }
        })
        .collect()
}

fn short_title(title: &str, ascii: bool) -> String {
    let title = safe_text(title);
    if title.chars().count() <= TITLE_LIMIT {
        return title;
    }
    let suffix = if ascii { "..." } else { "…" };
    let content: String = title
        .chars()
        .take(TITLE_LIMIT - suffix.chars().count())
        .collect();
    format!("{content}{suffix}")
}

fn marker(node_id: &str, view: Option<&RunView>, ascii: bool) -> &'static str {
    let Some(view) = view else {
        return "";
    };
    if view
        .available_inputs
        .iter()
        .any(|input| input.node_id == node_id)
    {
        if ascii {
            "> "
        } else {
            "▶ "
        }
    } else if view
        .waiting_joins
        .iter()
        .any(|join| join.node_id == node_id)
    {
        if ascii {
            "~ "
        } else {
            "◷ "
        }
    } else if view
        .activations
        .iter()
        .any(|activation| activation.node_id == node_id && activation.status == "active")
    {
        if ascii {
            "+ "
        } else {
            "◆ "
        }
    } else if view
        .activations
        .iter()
        .any(|activation| activation.node_id == node_id && activation.status == "completed")
    {
        if ascii {
            "x "
        } else {
            "✓ "
        }
    } else {
        ""
    }
}

struct Link {
    from: usize,
    to: usize,
    label: Option<String>,
}

pub(crate) fn render(diagram: &Value, view: Option<&RunView>, ascii: bool) -> String {
    let all_nodes = nodes(diagram);
    let index: HashMap<&str, usize> = all_nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (str_field(node, "id"), position + 1))
        .collect();
    let labels: Vec<String> = all_nodes
        .iter()
        .map(|node| {
            let id = str_field(node, "id");
            format!(
                "{}{}: {}",
                marker(id, view, ascii),
                id,
                short_title(str_field(node, "title"), ascii)
            )
        })
        .collect();
    let mut links = Vec::new();
    match str_field(diagram, "kind") {
        "decision-tree" => {
            for node in all_nodes {
                if let Some(choices) = node.get("choices").and_then(Value::as_array) {
                    for choice in choices {
                        links.push(Link {
                            from: index[str_field(node, "id")],
                            to: index[str_field(choice, "target")],
                            label: Some(safe_text(str_field(choice, "id"))),
                        });
                    }
                }
            }
        }
        "behavior-tree" => {
            for node in all_nodes {
                if let Some(children) = node.get("children").and_then(Value::as_array) {
                    for (position, child) in children.iter().enumerate() {
                        links.push(Link {
                            from: index[str_field(node, "id")],
                            to: index[child.as_str().expect("validated child")],
                            label: Some((position + 1).to_string()),
                        });
                    }
                }
            }
        }
        "activity-diagram" => {
            for edge in diagram["edges"].as_array().expect("validated edges") {
                links.push(Link {
                    from: index[str_field(edge, "source")],
                    to: index[str_field(edge, "target")],
                    label: edge.get("label").map(|_| safe_text(str_field(edge, "id"))),
                });
            }
        }
        _ => unreachable!("diagram is validated before rendering"),
    }

    let mut graph = Graph::new();
    for (position, label) in labels.iter().enumerate() {
        graph.add_node(position + 1, label);
    }
    for link in &links {
        graph.add_edge(link.from, link.to, link.label.as_deref());
    }
    let options = if ascii {
        RenderOptions::ascii()
    } else {
        RenderOptions::plain()
    };
    let drawing = graph.compute_layout().render_string(&options);
    let mut output = String::new();
    output.push_str(&format!(
        "{} ({}, {})\n",
        safe_text(str_field(diagram, "title")),
        safe_text(str_field(diagram, "id")),
        str_field(diagram, "kind")
    ));
    if let Some(view) = view {
        output.push_str(&format!(
            "Run {}  revision {}  {}\n",
            view.run_id, view.revision, view.status
        ));
    }
    output.push('\n');
    output.push_str(drawing.trim_end());
    output.push_str("\n\nNodes:\n");
    for node in all_nodes {
        output.push_str(&format!(
            "  {} [{}] {}\n",
            safe_text(str_field(node, "id")),
            str_field(node, "type"),
            safe_text(str_field(node, "title"))
        ));
        if let Some(description) = node.get("description").and_then(Value::as_str) {
            output.push_str(&format!("    {}\n", safe_text(description)));
        }
    }
    let mut choice_lines = Vec::new();
    match str_field(diagram, "kind") {
        "decision-tree" => {
            for node in all_nodes {
                if let Some(choices) = node.get("choices").and_then(Value::as_array) {
                    for choice in choices {
                        choice_lines.push(format!(
                            "  {} / {}: {} -> {}",
                            str_field(node, "id"),
                            str_field(choice, "id"),
                            safe_text(str_field(choice, "label")),
                            str_field(choice, "target")
                        ));
                    }
                }
            }
        }
        "activity-diagram" => {
            for edge in diagram["edges"].as_array().expect("validated edges") {
                if let Some(label) = edge.get("label").and_then(Value::as_str) {
                    choice_lines.push(format!(
                        "  {} / {}: {} -> {}",
                        str_field(edge, "source"),
                        str_field(edge, "id"),
                        safe_text(label),
                        str_field(edge, "target")
                    ));
                }
            }
        }
        _ => (),
    }
    if !choice_lines.is_empty() {
        output.push_str("\nChoices:\n");
        for line in choice_lines {
            output.push_str(&line);
            output.push('\n');
        }
    }
    if let Some(view) = view {
        if !view.available_inputs.is_empty() {
            output.push_str("\nAvailable inputs:\n");
            for input in &view.available_inputs {
                output.push_str(&format!(
                    "  {}  {}  {}\n",
                    input.activation_id, input.operation, input.node_id
                ));
            }
        }
        if !view.waiting_joins.is_empty() {
            output.push_str("\nWaiting joins:\n");
            for join in &view.waiting_joins {
                output.push_str(&format!(
                    "  {}  arrived: {}  pending: {}\n",
                    join.node_id,
                    join.arrived_branch_ids.join(", "),
                    join.pending_branch_ids.join(", ")
                ));
            }
        }
        if let Some(outcome) = &view.outcome {
            output.push_str(&format!("\nOutcome: {}\n", outcome));
        }
    }
    output
}
