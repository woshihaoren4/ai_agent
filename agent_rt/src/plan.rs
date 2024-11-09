use wd_tools::PFOk;
use crate::{Node, PlanResult};

pub struct GraphPlan{
    start : Node,
    end :Node
}

impl GraphPlan{
    pub fn test()->Self{
        let start = Node::new("start");
        let end = Node::new("end");
        Self{
            start,end
        }
    }
}

impl super::Plan for GraphPlan{
    fn get(&self, name: &str) -> Option<&Node> {
        Some(&self.start)
    }

    fn next(&mut self, name: &str) -> anyhow::Result<PlanResult> {
        if name == "end"{
            return PlanResult::End.ok();
        }
        Ok(PlanResult::Nodes(vec![self.end.clone()]))
    }

    fn remove(&mut self, name: &str) -> Option<Node> {
        return None
    }

    fn insert(&mut self, name: &str, node: Node) {

    }
}