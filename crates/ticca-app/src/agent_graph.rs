//! Agent call graph data structures for the UI.

use std::collections::HashMap;

use ticca_core::agents::AgentType;
use ticca_core::tools::AgentCallEvent;

#[derive(Debug, Clone)]
pub struct AgentNode {
    pub id: usize,
    pub agent_type: AgentType,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct AgentEdge {
    pub from: usize,
    pub to: usize,
    #[allow(dead_code)]
    pub prompt: String,
}

#[derive(Debug, Clone)]
pub struct AgentCallGraph {
    root_id: usize,
    nodes: HashMap<usize, AgentNode>,
    order: Vec<usize>,
    edges: Vec<AgentEdge>,
}

impl AgentCallGraph {
    pub fn new(root_agent: AgentType) -> Self {
        let root_id = 0;
        let mut nodes = HashMap::new();
        nodes.insert(
            root_id,
            AgentNode {
                id: root_id,
                agent_type: root_agent,
                label: format!("{} #{}", root_agent.display_name(), root_id),
            },
        );
        Self {
            root_id,
            nodes,
            order: vec![root_id],
            edges: Vec::new(),
        }
    }

    pub fn reset(&mut self, root_agent: AgentType) {
        *self = Self::new(root_agent);
    }

    pub fn record_call(&mut self, event: &AgentCallEvent) {
        self.ensure_node(event.parent_id, event.parent);
        self.ensure_node(event.child_id, event.child);

        let prompt_preview: String = event.prompt.chars().take(120).collect();
        let prompt_suffix = if event.prompt.chars().count() > 120 {
            "..."
        } else {
            ""
        };
        let prompt = format!("{}{}", prompt_preview, prompt_suffix);

        self.edges.push(AgentEdge {
            from: event.parent_id,
            to: event.child_id,
            prompt,
        });
    }

    pub fn root_id(&self) -> usize {
        self.root_id
    }

    pub fn nodes(&self) -> &HashMap<usize, AgentNode> {
        &self.nodes
    }

    pub fn order(&self) -> &[usize] {
        &self.order
    }

    pub fn edges(&self) -> &[AgentEdge] {
        &self.edges
    }

    fn ensure_node(&mut self, id: usize, agent_type: AgentType) {
        if self.nodes.contains_key(&id) {
            return;
        }

        self.nodes.insert(
            id,
            AgentNode {
                id,
                agent_type,
                label: format!("{} #{}", agent_type.display_name(), id),
            },
        );
        self.order.push(id);
    }
}
