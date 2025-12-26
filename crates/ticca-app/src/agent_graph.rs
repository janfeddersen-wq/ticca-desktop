//! Agent call graph data structures for the UI.
//!
//! Tracks agent executions across multiple "runs" (one run per chat message),
//! with status tracking and timestamps for duration calculation.

use std::collections::HashMap;
use std::time::Instant;

use ticca_core::agents::AgentType;
use ticca_core::tools::AgentCallEvent;

/// Status of an agent node during execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentStatus {
    #[default]
    Running,
    Completed,
    Failed,
}

/// Metadata for a single execution run (one chat message = one run).
#[derive(Debug, Clone)]
pub struct RunInfo {
    pub run_id: usize,
    #[allow(dead_code)]
    pub label: String,
    #[allow(dead_code)]
    pub started_at: Instant,
    pub completed: bool,
}

impl RunInfo {
    pub fn new(run_id: usize) -> Self {
        Self {
            run_id,
            label: format!("Run #{}", run_id),
            started_at: Instant::now(),
            completed: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentNode {
    pub id: usize,
    pub agent_type: AgentType,
    pub label: String,
    pub status: AgentStatus,
    #[allow(dead_code)]
    pub run_id: usize,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
}

impl AgentNode {
    /// Create a new agent node with default Running status.
    pub fn new(id: usize, agent_type: AgentType, run_id: usize) -> Self {
        Self {
            id,
            agent_type,
            label: format!("{} #{}", agent_type.display_name(), id),
            status: AgentStatus::Running,
            run_id,
            started_at: Some(Instant::now()),
            completed_at: None,
        }
    }

    /// Get the duration if both started_at and completed_at are set.
    pub fn duration(&self) -> Option<std::time::Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            _ => None,
        }
    }

    /// Get elapsed time since started (for running nodes).
    pub fn elapsed(&self) -> Option<std::time::Duration> {
        self.started_at.map(|start| start.elapsed())
    }
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
    current_run: usize,
    runs: Vec<RunInfo>,
}

impl AgentCallGraph {
    /// Create a new graph with no nodes yet.
    /// Starts at run 0 (placeholder state) - first `start_new_run()` creates Run #1.
    pub fn new(_root_agent: AgentType) -> Self {
        let root_id = 0;
        let current_run = 0; // Placeholder state before first real run
        let nodes = HashMap::new();

        Self {
            root_id,
            nodes,
            order: Vec::new(),
            edges: Vec::new(),
            current_run,
            runs: Vec::new(), // Empty - first start_new_run() creates Run #1
        }
    }

    /// Reset everything and start fresh at run 1.
    /// Used for NewSession - completely clears execution history.
    pub fn reset(&mut self, root_agent: AgentType) {
        *self = Self::new(root_agent);
    }

    /// Start a new run without clearing existing nodes.
    /// Increments the run counter and creates a new root node for this run.
    pub fn start_new_run(&mut self, root_agent: AgentType) {
        self.current_run += 1;
        self.runs.push(RunInfo::new(self.current_run));

        // Generate a new unique ID for the root node of this run
        let new_root_id = self.next_node_id();
        self.root_id = new_root_id;

        let root_node = AgentNode::new(new_root_id, root_agent, self.current_run);
        self.nodes.insert(new_root_id, root_node);
        self.order.push(new_root_id);
    }

    /// Ensure a node exists in the graph for the current run.
    /// Used when AgentStreamEvent::Start arrives before AgentCallEvent.
    pub fn ensure_node_exists(&mut self, node_id: usize, agent_type: AgentType) {
        self.ensure_node(node_id, agent_type);
    }

    /// Record an agent call event, creating nodes if needed.
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

    /// Mark a node as completed.
    pub fn mark_node_completed(&mut self, node_id: usize) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.status = AgentStatus::Completed;
            node.completed_at = Some(Instant::now());
        }
    }

    /// Mark a node as failed.
    pub fn mark_node_failed(&mut self, node_id: usize) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.status = AgentStatus::Failed;
            node.completed_at = Some(Instant::now());
        }
    }

    /// Mark a run as completed.
    pub fn mark_run_completed(&mut self, run_id: usize) {
        if let Some(run) = self.runs.iter_mut().find(|r| r.run_id == run_id) {
            run.completed = true;
        }
    }

    /// Get all nodes for a specific run.
    #[allow(dead_code)]
    pub fn nodes_for_run(&self, run_id: usize) -> Vec<&AgentNode> {
        self.nodes
            .values()
            .filter(|node| node.run_id == run_id)
            .collect()
    }

    /// Check if any node has Running status.
    pub fn is_any_running(&self) -> bool {
        self.nodes
            .values()
            .any(|node| node.status == AgentStatus::Running)
    }

    /// Get the current run ID.
    pub fn current_run_id(&self) -> usize {
        self.current_run
    }

    /// Get all runs.
    pub fn runs(&self) -> &[RunInfo] {
        &self.runs
    }

    /// Get the root node ID (for the current run).
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

    /// Generate the next unique node ID.
    fn next_node_id(&self) -> usize {
        self.nodes.keys().max().map(|max| max + 1).unwrap_or(1)
    }

    /// Ensure a node exists, creating it if needed with current run tracking.
    fn ensure_node(&mut self, id: usize, agent_type: AgentType) {
        if self.nodes.contains_key(&id) {
            return;
        }

        let node = AgentNode::new(id, agent_type, self.current_run);
        self.nodes.insert(id, node);
        self.order.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_graph_initializes_run_0() {
        let graph = AgentCallGraph::new(AgentType::Coding);
        assert_eq!(graph.current_run_id(), 0); // Initial placeholder state
        assert_eq!(graph.runs().len(), 0); // No runs yet
        assert_eq!(graph.nodes().len(), 0); // No nodes yet
    }

    #[test]
    fn test_first_start_new_run_creates_run_1() {
        let mut graph = AgentCallGraph::new(AgentType::Coding);
        graph.start_new_run(AgentType::Coding);

        assert_eq!(graph.current_run_id(), 1);
        assert_eq!(graph.runs().len(), 1);
        assert_eq!(graph.runs()[0].run_id, 1);
        assert_eq!(graph.runs()[0].label, "Run #1");
        assert!(!graph.runs()[0].completed);
    }

    #[test]
    fn test_start_new_run_increments_counter() {
        let mut graph = AgentCallGraph::new(AgentType::Coding);
        let initial_node_count = graph.nodes().len();

        // First run
        graph.start_new_run(AgentType::Coding);
        assert_eq!(graph.current_run_id(), 1);

        // Second run
        graph.start_new_run(AgentType::Coding);
        assert_eq!(graph.current_run_id(), 2);
        assert_eq!(graph.runs().len(), 2);
        assert_eq!(graph.runs()[1].run_id, 2);
        assert_eq!(graph.runs()[1].label, "Run #2");
        // Should have two more nodes (one per run)
        assert_eq!(graph.nodes().len(), initial_node_count + 2);
    }

    #[test]
    fn test_start_new_run_preserves_old_nodes() {
        let mut graph = AgentCallGraph::new(AgentType::Coding);
        graph.start_new_run(AgentType::Coding);
        let first_run_root = graph.root_id();

        graph.start_new_run(AgentType::Coding);

        // All roots should still exist
        assert!(graph.nodes().contains_key(&first_run_root));
        // New root should be different
        assert_ne!(graph.root_id(), first_run_root);
    }

    #[test]
    fn test_reset_clears_everything() {
        let mut graph = AgentCallGraph::new(AgentType::Coding);
        graph.start_new_run(AgentType::Coding);
        graph.start_new_run(AgentType::Coding);

        assert_eq!(graph.current_run_id(), 2);

        graph.reset(AgentType::Coding);

        assert_eq!(graph.current_run_id(), 0); // Back to initial state
        assert_eq!(graph.runs().len(), 0);
        assert_eq!(graph.nodes().len(), 0);
    }

    #[test]
    fn test_mark_node_completed() {
        let mut graph = AgentCallGraph::new(AgentType::Coding);
        graph.start_new_run(AgentType::Coding);
        let root_id = graph.root_id();

        assert_eq!(graph.nodes()[&root_id].status, AgentStatus::Running);
        assert!(graph.nodes()[&root_id].completed_at.is_none());

        graph.mark_node_completed(root_id);

        assert_eq!(graph.nodes()[&root_id].status, AgentStatus::Completed);
        assert!(graph.nodes()[&root_id].completed_at.is_some());
    }

    #[test]
    fn test_mark_node_failed() {
        let mut graph = AgentCallGraph::new(AgentType::Coding);
        graph.start_new_run(AgentType::Coding);
        let root_id = graph.root_id();

        graph.mark_node_failed(root_id);

        assert_eq!(graph.nodes()[&root_id].status, AgentStatus::Failed);
        assert!(graph.nodes()[&root_id].completed_at.is_some());
    }

    #[test]
    fn test_mark_run_completed() {
        let mut graph = AgentCallGraph::new(AgentType::Coding);
        graph.start_new_run(AgentType::Coding); // Create Run #1

        assert!(!graph.runs()[0].completed);

        graph.mark_run_completed(1);

        assert!(graph.runs()[0].completed);
    }

    #[test]
    fn test_nodes_for_run() {
        let mut graph = AgentCallGraph::new(AgentType::Coding);
        graph.start_new_run(AgentType::Coding); // Run #1
        graph.start_new_run(AgentType::Skills); // Run #2

        let run_0_nodes = graph.nodes_for_run(0); // No placeholder run
        let run_1_nodes = graph.nodes_for_run(1);
        let run_2_nodes = graph.nodes_for_run(2);

        assert_eq!(run_0_nodes.len(), 0);
        assert_eq!(run_1_nodes.len(), 1);
        assert_eq!(run_1_nodes[0].agent_type, AgentType::Coding);
        assert_eq!(run_2_nodes.len(), 1);
        assert_eq!(run_2_nodes[0].agent_type, AgentType::Skills);
    }

    #[test]
    fn test_is_any_running() {
        let mut graph = AgentCallGraph::new(AgentType::Coding);
        graph.start_new_run(AgentType::Coding);

        assert!(graph.is_any_running());

        graph.mark_node_completed(graph.root_id());

        assert!(!graph.is_any_running());
    }

    #[test]
    fn test_node_duration() {
        let mut graph = AgentCallGraph::new(AgentType::Coding);
        graph.start_new_run(AgentType::Coding);
        let root_id = graph.root_id();

        // Duration should be None before completion
        assert!(graph.nodes()[&root_id].duration().is_none());

        // Elapsed should be Some (time since start)
        assert!(graph.nodes()[&root_id].elapsed().is_some());

        graph.mark_node_completed(root_id);

        // Duration should now be Some
        assert!(graph.nodes()[&root_id].duration().is_some());
    }
}
