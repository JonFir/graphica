use crate::validate::{node, nodes, outgoing, str_field};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeError {
    pub code: String,
    pub message: String,
    pub details: Value,
}
fn error(code: &str, message: impl Into<String>) -> RuntimeError {
    RuntimeError {
        code: code.into(),
        message: message.into(),
        details: json!({}),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activation {
    pub activation_id: String,
    pub node_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_result: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub target: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub activation_id: String,
    pub node_id: String,
    pub operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<Choice>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_statuses: Option<Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingJoin {
    pub activation_id: String,
    pub node_id: String,
    pub fork_instance_id: String,
    pub arrived_branch_ids: Vec<String>,
    pub pending_branch_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunView {
    pub run_version: String,
    pub run_id: String,
    pub revision: u64,
    pub diagram_id: String,
    pub kind: String,
    pub status: String,
    pub outcome: Option<Value>,
    pub activations: Vec<Activation>,
    pub available_inputs: Vec<Input>,
    pub waiting_joins: Vec<WaitingJoin>,
    pub history: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Frame {
    activation_id: String,
    node_id: String,
    child_index: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Context {
    fork_instance_id: String,
    branch_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Token {
    activation_id: String,
    node_id: String,
    context: Vec<Context>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForkInstance {
    id: String,
    fork_node_id: String,
    join_node_id: String,
    branch_ids: Vec<String>,
    arrived: Vec<String>,
    arrival_sequences: Vec<u64>,
    join_activation_id: Option<String>,
    completed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum State {
    DecisionTree {
        active: Option<String>,
    },
    BehaviorTree {
        stack: Vec<Frame>,
    },
    ActivityDiagram {
        tokens: Vec<Token>,
        forks: Vec<ForkInstance>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub run_version: String,
    pub run_id: String,
    pub revision: u64,
    pub diagram: Value,
    pub status: String,
    pub outcome: Option<Value>,
    pub activations: Vec<Activation>,
    pub history: Vec<Value>,
    state: State,
    next_activation: u64,
    next_fork: u64,
}
#[derive(Clone)]
struct Pending {
    node_id: String,
    predecessor: Option<String>,
    edge_id: Option<String>,
    causal_event_sequence: Option<u64>,
    context: Vec<Context>,
    rule: String,
}

impl Run {
    pub fn start(diagram: Value, run_id: String) -> Result<Self, RuntimeError> {
        crate::validate::validate_diagram(&diagram)
            .map_err(|_| error("internal-error", "start requires a validated diagram"))?;
        let kind = str_field(&diagram, "kind").to_owned();
        let state = match kind.as_str() {
            "decision-tree" => State::DecisionTree { active: None },
            "behavior-tree" => State::BehaviorTree { stack: Vec::new() },
            "activity-diagram" => State::ActivityDiagram {
                tokens: Vec::new(),
                forks: Vec::new(),
            },
            _ => {
                return Err(error(
                    "internal-error",
                    "start requires a validated diagram",
                ))
            }
        };
        let start_id = if kind == "activity-diagram" {
            str_field(&diagram, "entry")
        } else {
            str_field(&diagram, "root")
        }
        .to_owned();
        let mut run = Run {
            run_version: "0.1".into(),
            run_id,
            revision: 1,
            diagram,
            status: "active".into(),
            outcome: None,
            activations: Vec::new(),
            history: Vec::new(),
            state,
            next_activation: 1,
            next_fork: 1,
        };
        run.emit(
            "run-started",
            json!({"diagramId": str_field(&run.diagram,"id"), "formatVersion": "0.1"}),
        );
        match kind.as_str() {
            "decision-tree" => run.enter_decision(&start_id, None, None),
            "behavior-tree" => run.enter_behavior(&start_id, None, None, "entry"),
            "activity-diagram" => run.advance_activity(vec![Pending {
                node_id: start_id,
                predecessor: None,
                edge_id: None,
                causal_event_sequence: None,
                context: Vec::new(),
                rule: "entry".into(),
            }])?,
            _ => unreachable!(),
        }
        run.check_progress()?;
        Ok(run)
    }
    fn emit(&mut self, kind: &str, extra: Value) -> u64 {
        let sequence = self.history.len() as u64 + 1;
        let mut event = serde_json::Map::new();
        event.insert("sequence".into(), json!(sequence));
        event.insert("revision".into(), json!(self.revision));
        event.insert("type".into(), json!(kind));
        if sequence > 1 {
            event.insert("causalEventSequence".into(), json!(sequence - 1));
        }
        if let Value::Object(o) = extra {
            event.extend(o);
        }
        self.history.push(Value::Object(event));
        sequence
    }
    fn enter(
        &mut self,
        node_id: &str,
        predecessor: Option<&str>,
        edge_id: Option<&str>,
        rule: &str,
        causal_event_sequence: Option<u64>,
    ) -> String {
        let id = format!("a{}", self.next_activation);
        self.next_activation += 1;
        self.activations.push(Activation {
            activation_id: id.clone(),
            node_id: node_id.into(),
            status: "active".into(),
            reported_result: None,
        });
        let mut event = json!({"nodeId":node_id,"activationId":id,
            "predecessorActivationId":predecessor,"rule":rule});
        if let Some(sequence) = causal_event_sequence {
            event["causalEventSequence"] = json!(sequence);
        }
        let seq = self.emit("node-entered", event);
        if let Some(from) = predecessor {
            self.emit(
                "transition-taken",
                json!({"sourceActivationId":from,"targetActivationId":id,
                "edgeOrChoiceId":edge_id,"rule":rule,"causalEventSequence":seq}),
            );
        }
        id
    }
    fn complete_activation(&mut self, activation_id: &str, result: Option<&str>, rule: &str) {
        if let Some(a) = self
            .activations
            .iter_mut()
            .find(|a| a.activation_id == activation_id)
        {
            a.status = "completed".into();
            if let Some(result) = result {
                a.reported_result = Some(result.into());
            }
            let node_id = a.node_id.clone();
            self.emit(
                "node-completed",
                json!({"nodeId":node_id,"activationId":activation_id,
                "result":result,"rule":rule}),
            );
        }
    }
    fn finish(&mut self, outcome: Value) {
        self.status = "completed".into();
        self.outcome = Some(outcome.clone());
        self.emit(
            "run-completed",
            json!({"outcome":outcome,"rule":"terminal-node"}),
        );
    }
    fn enter_decision(&mut self, node_id: &str, predecessor: Option<&str>, choice: Option<&str>) {
        let id = self.enter(
            node_id,
            predecessor,
            choice,
            if predecessor.is_some() {
                "selected-choice"
            } else {
                "entry"
            },
            None,
        );
        if str_field(node(&self.diagram, node_id), "type") == "result" {
            let value = str_field(node(&self.diagram, node_id), "value").to_owned();
            self.complete_activation(&id, None, "result");
            self.finish(json!({"kind":"decision-tree","nodeId":node_id,"value":value}));
        } else if let State::DecisionTree { active } = &mut self.state {
            *active = Some(id);
        }
    }
    fn enter_behavior(
        &mut self,
        node_id: &str,
        predecessor: Option<&str>,
        child_index: Option<usize>,
        rule: &str,
    ) {
        let id = self.enter(node_id, predecessor, None, rule, None);
        if let Some(position) = child_index {
            if let Some(event) = self.history.last_mut().and_then(Value::as_object_mut) {
                event.insert("childPosition".into(), json!(position));
            }
        }
        if let State::BehaviorTree { stack } = &mut self.state {
            stack.push(Frame {
                activation_id: id.clone(),
                node_id: node_id.into(),
                child_index: 0,
            });
        }
        let t = str_field(node(&self.diagram, node_id), "type");
        if t == "sequence" || t == "selector" {
            let child = node(&self.diagram, node_id)["children"][0]
                .as_str()
                .unwrap()
                .to_owned();
            self.enter_behavior(&child, Some(&id), Some(0), "child-entry");
        }
    }
    fn report_behavior(&mut self, activation_id: &str, status: &str) {
        if status == "running" {
            if let Some(a) = self
                .activations
                .iter_mut()
                .find(|a| a.activation_id == activation_id)
            {
                a.reported_result = Some("running".into());
            }
            return;
        }
        self.complete_activation(activation_id, Some(status), "caller-report");
        let mut result = status.to_owned();
        if let State::BehaviorTree { stack } = &mut self.state {
            stack.pop();
        }
        loop {
            let parent = match &self.state {
                State::BehaviorTree { stack } => stack.last().cloned(),
                _ => None,
            };
            let Some(frame) = parent else {
                self.finish(json!({"kind":"behavior-tree","status":result}));
                return;
            };
            let parent_node = node(&self.diagram, &frame.node_id);
            let t = str_field(parent_node, "type");
            let children = parent_node["children"].as_array().unwrap();
            let advance = (t == "sequence" && result == "success")
                || (t == "selector" && result == "failure");
            if advance && frame.child_index + 1 < children.len() {
                let next_index = frame.child_index + 1;
                let child = children[next_index].as_str().unwrap().to_owned();
                if let State::BehaviorTree { stack } = &mut self.state {
                    stack.last_mut().unwrap().child_index = next_index;
                }
                self.enter_behavior(
                    &child,
                    Some(&frame.activation_id),
                    Some(next_index),
                    if t == "sequence" {
                        "sequence-continuation"
                    } else {
                        "selector-fallback"
                    },
                );
                return;
            }
            result = if t == "sequence" {
                if advance {
                    "success"
                } else {
                    "failure"
                }
            } else if advance {
                "failure"
            } else {
                "success"
            }
            .into();
            self.complete_activation(
                &frame.activation_id,
                Some(&result),
                if t == "sequence" {
                    "sequence-propagation"
                } else {
                    "selector-propagation"
                },
            );
            if let State::BehaviorTree { stack } = &mut self.state {
                stack.pop();
            }
        }
    }
    fn advance_activity(&mut self, initial: Vec<Pending>) -> Result<(), RuntimeError> {
        let mut queue: VecDeque<Pending> = initial.into();
        while let Some(item) = queue.pop_front() {
            let t = str_field(node(&self.diagram, &item.node_id), "type").to_owned();
            let existing_join = if t == "join" {
                item.context.last().and_then(|context| match &self.state {
                    State::ActivityDiagram { forks, .. } => forks
                        .iter()
                        .find(|f| f.id == context.fork_instance_id)
                        .and_then(|f| f.join_activation_id.clone()),
                    _ => None,
                })
            } else {
                None
            };
            let id = if let Some(existing) = existing_join {
                self.emit(
                    "transition-taken",
                    json!({"sourceActivationId":item.predecessor,
                    "targetActivationId":existing,"edgeOrChoiceId":item.edge_id,"rule":item.rule}),
                );
                existing
            } else {
                self.enter(
                    &item.node_id,
                    item.predecessor.as_deref(),
                    item.edge_id.as_deref(),
                    &item.rule,
                    item.causal_event_sequence,
                )
            };
            match t.as_str() {
                "action" | "decision" => {
                    if let State::ActivityDiagram { tokens, .. } = &mut self.state {
                        tokens.push(Token {
                            activation_id: id,
                            node_id: item.node_id,
                            context: item.context,
                        });
                    }
                }
                "initial" | "merge" => {
                    self.complete_activation(
                        &id,
                        None,
                        if t == "initial" {
                            "initial-forward"
                        } else {
                            "merge-forward"
                        },
                    );
                    for edge in outgoing(&self.diagram, &item.node_id) {
                        queue.push_back(Pending {
                            node_id: str_field(edge, "target").into(),
                            predecessor: Some(id.clone()),
                            edge_id: Some(str_field(edge, "id").into()),
                            causal_event_sequence: None,
                            context: item.context.clone(),
                            rule: format!("{t}-forward"),
                        });
                    }
                }
                "fork" => {
                    self.complete_activation(&id, None, "fork-expansion");
                    let fork_completion_sequence = self.history.len() as u64;
                    let instance_id = format!("f{}", self.next_fork);
                    self.next_fork += 1;
                    let outs = outgoing(&self.diagram, &item.node_id);
                    let branch_ids: Vec<String> =
                        outs.iter().map(|e| str_field(e, "id").to_owned()).collect();
                    let join_node_id =
                        str_field(node(&self.diagram, &item.node_id), "join").to_owned();
                    if let State::ActivityDiagram { forks, .. } = &mut self.state {
                        forks.push(ForkInstance {
                            id: instance_id.clone(),
                            fork_node_id: item.node_id.clone(),
                            join_node_id,
                            branch_ids,
                            arrived: Vec::new(),
                            arrival_sequences: Vec::new(),
                            join_activation_id: None,
                            completed: false,
                        });
                    }
                    for edge in outs {
                        let branch_id = str_field(edge, "id").to_owned();
                        let mut context = item.context.clone();
                        context.push(Context {
                            fork_instance_id: instance_id.clone(),
                            branch_id: branch_id.clone(),
                        });
                        queue.push_back(Pending {
                            node_id: str_field(edge, "target").into(),
                            predecessor: Some(id.clone()),
                            edge_id: Some(branch_id),
                            causal_event_sequence: Some(fork_completion_sequence),
                            context,
                            rule: "fork-branch".into(),
                        });
                    }
                }
                "join" => {
                    let Some(branch) = item.context.last().cloned() else {
                        return Err(error("internal-error", "join without fork context"));
                    };
                    let Some(index) = (match &self.state {
                        State::ActivityDiagram { forks, .. } => forks.iter().position(|f| {
                            f.id == branch.fork_instance_id
                                && f.join_node_id == item.node_id
                                && !f.completed
                        }),
                        _ => None,
                    }) else {
                        return Err(error(
                            "internal-error",
                            "join reached by wrong fork instance",
                        ));
                    };
                    // Only the first arrival has a persistent waiting join activation.
                    let (join_id, released, branch_ids, arrived) = {
                        let State::ActivityDiagram { forks, .. } = &mut self.state else {
                            unreachable!()
                        };
                        let fork = &mut forks[index];
                        if fork.arrived.contains(&branch.branch_id) {
                            return Err(error("internal-error", "duplicate branch arrival"));
                        }
                        let first = fork.join_activation_id.is_none();
                        if first {
                            fork.join_activation_id = Some(id.clone());
                        }
                        fork.arrived.push(branch.branch_id.clone());
                        let join_id = fork.join_activation_id.clone().unwrap();
                        let released = fork.arrived.len() == fork.branch_ids.len();
                        if released {
                            fork.completed = true;
                        }
                        (
                            join_id,
                            released,
                            fork.branch_ids.clone(),
                            fork.arrived.clone(),
                        )
                    };
                    if !released {
                        if let Some(a) = self.activations.iter_mut().find(|a| a.activation_id == id)
                        {
                            a.status = "waiting".into();
                        }
                    }
                    let arrival_sequence = self.emit(
                        "join-arrived",
                        json!({"activationId":join_id,"forkInstanceId":branch.fork_instance_id,
                        "branchId":branch.branch_id,"arrivedBranchIds":arrived}),
                    );
                    let arrival_sequences =
                        if let State::ActivityDiagram { forks, .. } = &mut self.state {
                            let fork = &mut forks[index];
                            fork.arrival_sequences.push(arrival_sequence);
                            fork.arrival_sequences.clone()
                        } else {
                            unreachable!()
                        };
                    if released {
                        self.emit("join-released",json!({"activationId":join_id,"forkInstanceId":branch.fork_instance_id,
                            "branchIds":branch_ids,"rule":"all-branches-arrived","causalEventSequences":arrival_sequences}));
                        self.complete_activation(&join_id, None, "join-release");
                        let mut context = item.context;
                        context.pop();
                        for edge in outgoing(&self.diagram, &item.node_id) {
                            queue.push_back(Pending {
                                node_id: str_field(edge, "target").into(),
                                predecessor: Some(join_id.clone()),
                                edge_id: Some(str_field(edge, "id").into()),
                                causal_event_sequence: None,
                                context: context.clone(),
                                rule: "join-continuation".into(),
                            });
                        }
                    }
                }
                "final" => {
                    if !item.context.is_empty() {
                        return Err(error("internal-error", "final reached inside open fork"));
                    }
                    self.complete_activation(&id, None, "final");
                    self.finish(json!({"kind":"activity-diagram","nodeId":item.node_id}));
                    if !queue.is_empty() {
                        return Err(error(
                            "internal-error",
                            "final reached with pending transitions",
                        ));
                    }
                }
                _ => return Err(error("internal-error", "unknown activity node")),
            }
        }
        Ok(())
    }
    fn check_progress(&self) -> Result<(), RuntimeError> {
        if self.status == "active" && self.available_inputs().is_empty() {
            Err(error(
                "internal-error",
                "active run has no available inputs",
            ))
        } else if self.status == "completed"
            && (!self.available_inputs().is_empty()
                || matches!(&self.state, State::ActivityDiagram { forks, .. } if forks.iter().any(|f| !f.completed)))
        {
            Err(error("internal-error", "completed run has pending inputs"))
        } else {
            Ok(())
        }
    }
    pub fn available_inputs(&self) -> Vec<Input> {
        let ids: Vec<String> = match &self.state {
            State::DecisionTree { active } => active.iter().cloned().collect(),
            State::BehaviorTree { stack } => stack
                .last()
                .map(|f| f.activation_id.clone())
                .into_iter()
                .collect(),
            State::ActivityDiagram { tokens, .. } => {
                tokens.iter().map(|t| t.activation_id.clone()).collect()
            }
        };
        let mut ids = ids;
        ids.sort_by_key(|id| {
            self.activations
                .iter()
                .position(|a| &a.activation_id == id)
                .unwrap_or(usize::MAX)
        });
        ids.into_iter()
            .filter_map(|id| {
                let a = self
                    .activations
                    .iter()
                    .find(|a| a.activation_id == id && a.status == "active")?;
                let n = node(&self.diagram, &a.node_id);
                let t = str_field(n, "type");
                let choices = if t == "question" {
                    Some(
                        n["choices"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|c| Choice {
                                id: str_field(c, "id").into(),
                                label: str_field(c, "label").into(),
                                target: str_field(c, "target").into(),
                            })
                            .collect(),
                    )
                } else if t == "decision" {
                    Some(
                        outgoing(&self.diagram, &a.node_id)
                            .into_iter()
                            .map(|e| Choice {
                                id: str_field(e, "id").into(),
                                label: str_field(e, "label").into(),
                                target: str_field(e, "target").into(),
                            })
                            .collect(),
                    )
                } else {
                    None
                };
                let allowed_statuses = match t {
                    "condition" => Some(vec!["success".into(), "failure".into()]),
                    "action" if str_field(&self.diagram, "kind") == "behavior-tree" => {
                        Some(vec!["running".into(), "success".into(), "failure".into()])
                    }
                    "action" => Some(vec!["success".into()]),
                    _ => None,
                };
                Some(Input {
                    activation_id: id,
                    node_id: a.node_id.clone(),
                    operation: if choices.is_some() {
                        "choose"
                    } else {
                        "report"
                    }
                    .into(),
                    choices,
                    allowed_statuses,
                })
            })
            .collect()
    }
    pub fn view(&self) -> RunView {
        let waiting_joins = match &self.state {
            State::ActivityDiagram { forks, .. } => forks
                .iter()
                .filter(|f| !f.completed)
                .filter_map(|f| {
                    let activation_id = f.join_activation_id.clone()?;
                    Some(WaitingJoin {
                        activation_id,
                        node_id: f.join_node_id.clone(),
                        fork_instance_id: f.id.clone(),
                        arrived_branch_ids: f.arrived.clone(),
                        pending_branch_ids: f
                            .branch_ids
                            .iter()
                            .filter(|id| !f.arrived.contains(id))
                            .cloned()
                            .collect(),
                    })
                })
                .collect(),
            _ => Vec::new(),
        };
        RunView {
            run_version: self.run_version.clone(),
            run_id: self.run_id.clone(),
            revision: self.revision,
            diagram_id: str_field(&self.diagram, "id").into(),
            kind: str_field(&self.diagram, "kind").into(),
            status: self.status.clone(),
            outcome: self.outcome.clone(),
            activations: self.activations.clone(),
            available_inputs: self.available_inputs(),
            waiting_joins,
            history: self.history.clone(),
        }
    }
    pub fn next(&self) -> Value {
        json!({"runId":self.run_id,"revision":self.revision,"status":self.status,
            "outcome":self.outcome,"availableInputs":self.available_inputs()})
    }
    pub fn history(&self) -> Value {
        json!({"runId":self.run_id,"revision":self.revision,"events":self.history})
    }
    pub fn check_stored(&self) -> Result<(), RuntimeError> {
        if self.run_version != "0.1" {
            return Err(error("unsupported-run-version", "unsupported run version"));
        }
        if self.revision == 0 || self.run_id.is_empty() {
            return Err(error("corrupt-run", "invalid run identity or revision"));
        }
        if self.history.iter().enumerate().any(|(i, e)| {
            e["sequence"].as_u64() != Some(i as u64 + 1)
                || e["revision"]
                    .as_u64()
                    .is_none_or(|r| r == 0 || r > self.revision)
        }) {
            return Err(error("corrupt-run", "invalid event sequence"));
        }
        crate::validate::validate_diagram(&self.diagram)
            .map_err(|_| error("corrupt-run", "invalid frozen diagram"))?;
        let kind = str_field(&self.diagram, "kind");
        let matching_state = matches!(
            (&self.state, kind),
            (State::DecisionTree { .. }, "decision-tree")
                | (State::BehaviorTree { .. }, "behavior-tree")
                | (State::ActivityDiagram { .. }, "activity-diagram")
        );
        if !matching_state {
            return Err(error("corrupt-run", "state kind does not match diagram"));
        }
        if !matches!(self.status.as_str(), "active" | "completed")
            || (self.status == "active" && self.outcome.is_some())
            || (self.status == "completed" && self.outcome.is_none())
        {
            return Err(error("corrupt-run", "invalid lifecycle"));
        }
        let node_ids: HashSet<&str> = nodes(&self.diagram)
            .iter()
            .map(|n| str_field(n, "id"))
            .collect();
        let mut activation_ids = HashSet::new();
        for activation in &self.activations {
            if !activation_ids.insert(activation.activation_id.as_str())
                || !node_ids.contains(activation.node_id.as_str())
                || !matches!(
                    activation.status.as_str(),
                    "active" | "waiting" | "completed"
                )
            {
                return Err(error("corrupt-run", "invalid activation"));
            }
        }
        if self.next_activation != self.activations.len() as u64 + 1 {
            return Err(error("corrupt-run", "invalid activation counter"));
        }
        let active_node = |id: &str| {
            self.activations
                .iter()
                .find(|a| a.activation_id == id && a.status == "active")
                .map(|a| a.node_id.as_str())
        };
        match &self.state {
            State::DecisionTree { active } => {
                if (self.status == "active") != active.is_some()
                    || active.as_ref().is_some_and(|id| {
                        active_node(id).is_none_or(|node_id| {
                            str_field(node(&self.diagram, node_id), "type") != "question"
                        })
                    })
                {
                    return Err(error("corrupt-run", "invalid active question"));
                }
            }
            State::BehaviorTree { stack } => {
                if (self.status == "active") != !stack.is_empty()
                    || stack.iter().enumerate().any(|(i, frame)| {
                        if active_node(&frame.activation_id) != Some(frame.node_id.as_str()) {
                            return true;
                        }
                        let frame_node = node(&self.diagram, &frame.node_id);
                        let node_type = str_field(frame_node, "type");
                        if i + 1 == stack.len() {
                            !matches!(node_type, "action" | "condition")
                        } else {
                            if !matches!(node_type, "sequence" | "selector") {
                                return true;
                            }
                            let children = frame_node["children"].as_array().unwrap();
                            children.get(frame.child_index).and_then(Value::as_str)
                                != Some(stack[i + 1].node_id.as_str())
                        }
                    })
                    || stack
                        .first()
                        .is_some_and(|frame| frame.node_id != str_field(&self.diagram, "root"))
                {
                    return Err(error("corrupt-run", "invalid behavior stack"));
                }
            }
            State::ActivityDiagram { tokens, forks } => {
                if self.next_fork != forks.len() as u64 + 1 {
                    return Err(error("corrupt-run", "invalid fork counter"));
                }
                if tokens.iter().any(|t| {
                    active_node(&t.activation_id) != Some(t.node_id.as_str())
                        || !node_ids.contains(t.node_id.as_str())
                        || !matches!(
                            str_field(node(&self.diagram, &t.node_id), "type"),
                            "action" | "decision"
                        )
                }) {
                    return Err(error("corrupt-run", "invalid activity token"));
                }
                if forks.iter().any(|f| {
                    !node_ids.contains(f.fork_node_id.as_str())
                        || !node_ids.contains(f.join_node_id.as_str())
                        || f.arrived.iter().any(|b| !f.branch_ids.contains(b))
                        || f.arrived.iter().collect::<HashSet<_>>().len() != f.arrived.len()
                        || f.arrival_sequences.len() != f.arrived.len()
                        || f.join_activation_id
                            .as_ref()
                            .is_some_and(|id| !activation_ids.contains(id.as_str()))
                }) {
                    return Err(error("corrupt-run", "invalid fork instance"));
                }
            }
        }
        self.check_progress()
            .map_err(|_| error("corrupt-run", "invalid run progress"))?;
        Ok(())
    }
}

fn precheck(
    run: &Run,
    revision: u64,
    activation_id: &str,
    operation: &str,
) -> Result<Input, RuntimeError> {
    if run.revision != revision {
        return Err(RuntimeError {
            code: "revision-conflict".into(),
            message: format!("Expected revision {revision}; found {}.", run.revision),
            details: json!({"expected":revision,"actual":run.revision}),
        });
    }
    if run.status == "completed" {
        return Err(error("run-completed", "run is already completed"));
    }
    let Some(input) = run
        .available_inputs()
        .into_iter()
        .find(|i| i.activation_id == activation_id)
    else {
        return Err(error(
            "inactive-activation",
            "activation is unknown or inactive",
        ));
    };
    if input.operation != operation {
        return Err(error(
            "operation-mismatch",
            "operation does not match activation",
        ));
    }
    Ok(input)
}
pub fn apply_choice(
    run: &mut Run,
    revision: u64,
    activation_id: &str,
    choice_id: &str,
    note: Option<&str>,
) -> Result<(), RuntimeError> {
    let input = precheck(run, revision, activation_id, "choose")?;
    let Some(choices) = input.choices else {
        return Err(error("internal-error", "choice input has no choices"));
    };
    let Some(choice) = choices.into_iter().find(|c| c.id == choice_id) else {
        return Err(error("invalid-choice", "choice is unavailable"));
    };
    let mut updated = run.clone();
    updated.revision += 1;
    updated.emit(
        "choice-selected",
        json!({"activationId":activation_id,"nodeId":input.node_id,
        "choiceId":choice_id,"note":note}),
    );
    updated.complete_activation(activation_id, None, "choice-selected");
    match &mut updated.state {
        State::DecisionTree { active } => {
            *active = None;
            updated.enter_decision(&choice.target, Some(activation_id), Some(choice_id));
        }
        State::ActivityDiagram { tokens, .. } => {
            let index = tokens
                .iter()
                .position(|t| t.activation_id == activation_id)
                .unwrap();
            let token = tokens.remove(index);
            updated.advance_activity(vec![Pending {
                node_id: choice.target,
                predecessor: Some(activation_id.into()),
                edge_id: Some(choice_id.into()),
                causal_event_sequence: None,
                context: token.context,
                rule: "selected-edge".into(),
            }])?;
        }
        _ => return Err(error("internal-error", "choice on behavior tree")),
    }
    updated.check_progress()?;
    *run = updated;
    Ok(())
}
pub fn apply_report(
    run: &mut Run,
    revision: u64,
    activation_id: &str,
    status: &str,
    note: Option<&str>,
) -> Result<(), RuntimeError> {
    let input = precheck(run, revision, activation_id, "report")?;
    let Some(allowed_statuses) = &input.allowed_statuses else {
        return Err(error(
            "internal-error",
            "report input has no allowed statuses",
        ));
    };
    if !allowed_statuses.iter().any(|s| s == status) {
        return Err(error("invalid-report", "status is unavailable"));
    }
    let mut updated = run.clone();
    updated.revision += 1;
    updated.emit(
        "status-reported",
        json!({"activationId":activation_id,"nodeId":input.node_id,
        "status":status,"note":note}),
    );
    match &mut updated.state {
        State::BehaviorTree { .. } => updated.report_behavior(activation_id, status),
        State::ActivityDiagram { tokens, .. } => {
            let index = tokens
                .iter()
                .position(|t| t.activation_id == activation_id)
                .unwrap();
            let token = tokens.remove(index);
            updated.complete_activation(activation_id, Some(status), "caller-report");
            let edge = outgoing(&updated.diagram, &token.node_id)[0];
            let pending = Pending {
                node_id: str_field(edge, "target").into(),
                predecessor: Some(activation_id.into()),
                edge_id: Some(str_field(edge, "id").into()),
                causal_event_sequence: None,
                context: token.context,
                rule: "action-completed".into(),
            };
            updated.advance_activity(vec![pending])?;
        }
        _ => return Err(error("internal-error", "report on decision tree")),
    }
    updated.check_progress()?;
    *run = updated;
    Ok(())
}
