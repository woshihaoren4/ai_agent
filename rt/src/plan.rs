use crate::{Context, NextNodeResult, Plan, END_NODE_CODE, START_NODE_CODE};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use wd_tools::{PFErr, PFOk};

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub code: String,         //当前节点的编码
    pub node_type_id: String, //类型节点id
    pub node_config: String,  //类型节点配置
}

#[derive(Debug, Default, Clone)]
pub struct PlanNode {
    ready: Vec<String>,
    go: Vec<String>,
    cfg: Option<Node>,
}

#[derive(Debug, Default)]
pub struct LockPlan {
    map: Mutex<HashMap<String, PlanNode>>,
}

#[derive(Debug, Clone)]
pub struct PlanBuilder {
    map: HashMap<String, PlanNode>,
}

impl From<(Vec<String>, Node, Vec<String>)> for PlanNode {
    fn from((ready, cfg, go): (Vec<String>, Node, Vec<String>)) -> Self {
        Self {
            ready,
            go,
            cfg: Some(cfg),
        }
    }
}
impl From<(Node, Vec<String>)> for PlanNode {
    fn from((cfg, go): (Node, Vec<String>)) -> Self {
        Self {
            ready: vec![],
            go,
            cfg: Some(cfg),
        }
    }
}
impl From<(Node, String)> for PlanNode {
    fn from((cfg, go): (Node, String)) -> Self {
        Self {
            ready: vec![],
            go: vec![go],
            cfg: Some(cfg),
        }
    }
}
impl From<(Node, &str)> for PlanNode {
    fn from((cfg, go): (Node, &str)) -> Self {
        (cfg, go.to_string()).into()
    }
}

impl LockPlan {
    pub fn string(&self) -> String {
        let mut res = String::new();
        let lock = self.map.lock().unwrap();
        PlanBuilder::str_tree(START_NODE_CODE, &mut res, lock.deref());
        return res;
    }
}

impl PlanBuilder {
    pub fn start<N: Into<Node>, S: Into<String>>(node: N, next_nodes: Vec<S>) -> Self {
        let node = node.into();
        let mut map = HashMap::new();
        let go = next_nodes
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<String>>();

        let first_node_code = node.code.clone();
        let start = (Node::default(), vec![first_node_code.clone()]).into();
        let plan = (node, go).into();

        map.insert(START_NODE_CODE.to_string(), start);
        map.insert(first_node_code, plan);
        Self { map }
    }
    pub fn start_new_branch<N: Into<Node>, S: Into<String>>(
        &mut self,
        node: N,
        next_nodes: Vec<S>,
    ) -> &mut Self {
        let node = node.into();
        let code = node.code.clone();
        if let Some(s) = self.map.get_mut(START_NODE_CODE) {
            s.go.push(code.clone());
        }
        let go = next_nodes
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<String>>();
        let plan = (node, go).into();
        self.map.insert(code, plan);
        self
    }
    pub fn insert_node<N: Into<PlanNode>>(&mut self, node: N) -> &mut Self {
        let node = node.into();
        let code = if let Some(ref s) = node.cfg {
            s.code.clone()
        } else {
            "".into()
        };
        self.map.insert(code, node);
        self
    }
    pub fn sequence<N: Into<Node>, S: Into<String>>(
        &mut self,
        nodes: Vec<N>,
        next_node_code: S,
    ) -> &mut Self {
        let mut nodes = nodes.into_iter().map(|x| x.into()).collect::<Vec<Node>>();
        for _ in 0..nodes.len() {
            if nodes.is_empty() {
                break;
            } else if nodes.len() == 1 {
                let node = nodes.pop().unwrap();
                let go = vec![next_node_code.into()];
                self.insert_node((node, go));
                break;
            } else {
                let node = nodes.remove(0);
                let code = nodes.get(0).unwrap().code.clone();
                self.insert_node((node, vec![code]));
            }
        }
        self
    }
    pub fn fission<S: Into<String>, N: Into<Node>>(&mut self, node: N, go: Vec<S>) -> &mut Self {
        let go = go.into_iter().map(|x| x.into()).collect::<Vec<String>>();
        self.insert_node((node.into(), go))
    }
    //在ready_codes中指定需要等待那些节点
    pub fn merged<R: Into<String>, S: Into<String>, N: Into<Node>>(
        &mut self,
        ready_codes: Vec<R>,
        node: N,
        next_node_code: S,
    ) -> &mut Self {
        let ready_codes = ready_codes
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<String>>();
        self.insert_node((ready_codes, node.into(), vec![next_node_code.into()]))
    }
    pub fn end<R: Into<String>, N: Into<Node>>(
        &mut self,
        ready_codes: Vec<R>,
        node: N,
    ) -> &mut Self {
        let ready_codes = ready_codes
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<String>>();
        self.insert_node((ready_codes, node.into(), vec![]))
    }
    pub fn single_node<T: Into<String>, C: Into<String>>(node_type_id: T, cfg: C) -> Self {
        Self::start((END_NODE_CODE, node_type_id.into(), cfg.into()), vec![""])
    }

    pub fn check(&self, code: &str, count: &mut i32) -> anyhow::Result<()> {
        if *count > 1000 {
            return anyhow::anyhow!("The number of cycles exceeded 1000").err();
        }
        *count += 1;
        if let Some(s) = self.map.get(code) {
            for i in s.go.iter() {
                if i.as_str() == END_NODE_CODE {
                    continue;
                }
                let e = self.check(i.as_str(), count);
                if e.is_err() {
                    return e;
                }
            }
        } else {
            if code != END_NODE_CODE {
                return anyhow::anyhow!("end node is illogical").err();
            }
        };
        Ok(())
    }
    pub fn build(&mut self) -> LockPlan {
        let map = std::mem::take(&mut self.map);
        let map = Mutex::new(map);
        LockPlan { map }
    }
    pub fn check_and_build(&mut self) -> anyhow::Result<LockPlan> {
        let mut index = 0;
        self.check(START_NODE_CODE, &mut index)?;
        self.build().ok()
    }
    pub fn string(&self) -> String {
        let mut res = String::new();
        Self::str_tree(START_NODE_CODE, &mut res, &self.map);
        return res;
    }
    fn str_tree(code: &str, out: &mut String, map: &HashMap<String, PlanNode>) {
        let ns = if let Some(opt) = map.get(code) {
            opt
        } else {
            return;
        };
        for i in ns.go.iter() {
            if let Some(ref cfg) = ns.cfg {
                out.push_str(&format!(
                    "\n{}->{}: [{}] {}",
                    code, i, cfg.node_type_id, cfg.node_config
                ));
            } else {
                out.push_str(&format!("\n{}->{}: [] ", code, i));
            }
            Self::str_tree(i.as_str(), out, map);
        }
    }
}

impl<S1, S2, S3> From<(S1, S2, S3)> for Node
where
    S1: Into<String>,
    S2: Into<String>,
    S3: Into<String>,
{
    fn from((code, id, cfg): (S1, S2, S3)) -> Self {
        Node {
            code: code.into(),
            node_type_id: id.into(),
            node_config: cfg.into(),
        }
    }
}
impl<S1, S2> From<(S1, S2)> for Node
where
    S1: Into<String>,
    S2: Into<String>,
{
    fn from((code, id): (S1, S2)) -> Self {
        Node {
            code: code.into(),
            node_type_id: id.into(),
            node_config: "".into(),
        }
    }
}

impl Plan for LockPlan {
    fn next(&self, _ctx: Arc<Context>, node_code: &str) -> NextNodeResult {
        if node_code == END_NODE_CODE {
            return NextNodeResult::Over;
        }
        let mut lock = match self.map.lock() {
            Ok(o) => o,
            Err(e) => {
                return NextNodeResult::Error(e.to_string());
            }
        };
        let p = if let Some(p) = lock.get(node_code) {
            p.go.clone()
        } else {
            return NextNodeResult::Error(format!("node[{}] not found", node_code));
        };
        let mut list = vec![];
        for i in p {
            let node = if let Some(n) = lock.get_mut(i.as_str()) {
                n
            } else {
                return NextNodeResult::Error(format!("node[{}] not found", i));
            };
            if node.cfg.is_none() {
                continue;
            }
            for i in 0..node.ready.len() {
                if node.ready[i].as_str() == node_code {
                    node.ready.remove(i);
                    break;
                }
            }
            if node.ready.is_empty() {
                let node = std::mem::take(&mut node.cfg).unwrap();
                list.push(node)
            }
        }
        NextNodeResult::Nodes(list)
    }

    fn set(&self, nodes: Vec<PlanNode>) {
        let mut lock = self.map.lock().unwrap();
        for i in nodes {
            if let Some(ref cfg) = i.cfg {
                lock.insert(cfg.code.clone(), i);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::plan::PlanBuilder;
    use crate::{Node, END_NODE_CODE, START_NODE_CODE};

    #[test]
    fn test_plan_builder() {
        let planer = PlanBuilder::start((START_NODE_CODE, "1", ""), vec!["B"])
            .insert_node((Node::from(("B", "2", "")), "C"))
            .sequence(vec![("C", "3", ""), ("E", "4", "")], "F")
            .fission(("F", "3", ""), vec!["G", "H"])
            .sequence(vec![("G", "3", "")], "M")
            .sequence(vec![("H", "3", "")], "M")
            .merged(vec!["G", "H"], ("M", "4", ""), END_NODE_CODE)
            .end(vec!["M"], (END_NODE_CODE, "5", ""))
            .check_and_build()
            .unwrap();

        println!("{}", planer.string());
    }
}
