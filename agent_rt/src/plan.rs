use crate::{Node, PlanResult};
use wd_tools::PFOk;

pub struct GraphPlan {
    start: Node,
    end: Node,
}

impl GraphPlan {
    pub fn test() -> Self {
        let start = Node::new("start");
        let end = Node::new("end");
        Self { start, end }
    }
}

impl super::Plan for GraphPlan {
    fn next(&mut self, name: &str) -> anyhow::Result<PlanResult> {
        if name == self.start_node_name() {
            return Ok(PlanResult::Nodes(vec![self.start.clone()]));
        }
        if name == self.end_node_name() {
            return PlanResult::End.ok();
        }
        Ok(PlanResult::Nodes(vec![self.end.clone()]))
    }
    fn remove(&mut self, name: &str) -> Option<Node> {
        return None;
    }
    fn insert(&mut self, name: &str, node: Node) {}
}
