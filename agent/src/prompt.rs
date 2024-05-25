use crate::{Memory, PromptBuilder};
use std::mem::take;
use std::sync::Arc;
use std::vec;

#[derive(Eq, PartialEq)]
pub enum Language {
    Chinese,
}

#[derive(Clone, Default)]
pub struct PromptCommonTemplate {
    role: Option<String>,
    target: Option<String>,
    style: Option<String>,
    skill: Option<Vec<String>>,
    example: Option<Vec<String>>,
    user: Option<Vec<(String, String)>>,
    records: Option<Vec<String>>,
    limit: Option<Vec<String>>,
    extend: Option<Vec<(String, String)>>,

    tags: Vec<String>,
    memory: Option<Arc<dyn Memory>>,
    recall_history: usize,
}

impl PromptCommonTemplate {
    #[allow(dead_code)]
    pub fn role<S: Into<String>>(&mut self, content: S) -> &mut Self {
        self.role = Some(content.into());
        self
    }
    #[allow(dead_code)]
    pub fn target<S: Into<String>>(&mut self, content: S) -> &mut Self {
        self.target = Some(content.into());
        self
    }
    #[allow(dead_code)]
    pub fn style<S: Into<String>>(&mut self, content: S) -> &mut Self {
        self.style = Some(content.into());
        self
    }
    #[allow(dead_code)]
    pub fn memory(&mut self, mem: Arc<dyn Memory>) -> &mut Self {
        self.memory = Some(mem);
        self
    }
    #[allow(dead_code)]
    pub fn history_top_n(&mut self, n: usize) -> &mut Self {
        self.recall_history = n;
        self
    }
    #[allow(dead_code)]
    pub fn add_skill<S: Into<String>>(&mut self, content: S) -> &mut Self {
        if let Some(ref mut s) = self.skill {
            s.push(content.into());
        } else {
            self.skill = Some(vec![content.into()]);
        }
        self
    }
    #[allow(dead_code)]
    pub fn add_example<S: Into<String>>(&mut self, content: S) -> &mut Self {
        if let Some(ref mut s) = self.example {
            s.push(content.into());
        } else {
            self.example = Some(vec![content.into()]);
        }
        self
    }
    #[allow(dead_code)]
    pub fn add_records<S: Into<String>>(&mut self, content: S) -> &mut Self {
        if let Some(ref mut s) = self.records {
            s.push(content.into());
        } else {
            self.records = Some(vec![content.into()]);
        }
        self
    }
    #[allow(dead_code)]
    pub fn add_user<K: Into<String>, V: Into<String>>(&mut self, key: K, val: V) -> &mut Self {
        if let Some(ref mut s) = self.user {
            s.push((key.into(), val.into()));
        } else {
            self.user = Some(vec![(key.into(), val.into())]);
        }
        self
    }
    #[allow(dead_code)]
    pub fn add_extend<K: Into<String>, V: Into<String>>(&mut self, key: K, val: V) -> &mut Self {
        if let Some(ref mut s) = self.extend {
            s.push((key.into(), val.into()));
        } else {
            self.extend = Some(vec![(key.into(), val.into())]);
        }
        self
    }
    #[allow(dead_code)]
    pub fn add_tags(&mut self, mut tags: Vec<String>) -> &mut Self {
        self.tags.append(&mut tags);
        self
    }
    #[allow(dead_code)]
    pub fn add_limit<S: Into<String>>(&mut self, content: S) -> &mut Self {
        if let Some(ref mut s) = self.limit {
            s.push(content.into());
        } else {
            self.limit = Some(vec![content.into()]);
        }
        self
    }
}

#[async_trait::async_trait]
impl PromptBuilder for PromptCommonTemplate {
    async fn build(&self, uid: &str, query: &str, lg: Language) -> String {
        if lg != Language::Chinese {
            return "".into();
        }
        let mut p = String::new();

        if let Some(ref s) = self.role {
            p.push_str("# 角色\n");
            p.push_str(s.as_str())
        }
        if let Some(ref s) = self.target {
            if !p.is_empty() {
                p.push_str("\n")
            }
            p.push_str("## 目标\n");
            p.push_str(s.as_str())
        }
        if let Some(ref s) = self.style {
            if !p.is_empty() {
                p.push_str("\n")
            }
            p.push_str("## 风格\n");
            p.push_str(s.as_str())
        }
        if let Some(ref list) = self.skill {
            if !p.is_empty() {
                p.push_str("\n")
            }
            p.push_str("## 技能");
            for (i, e) in list.iter().enumerate() {
                p.push_str("\n");
                p += format!("{}. {}", i + 1, e.as_str()).as_str();
            }
        }
        if let Some(ref list) = self.example {
            if !p.is_empty() {
                p.push_str("\n")
            }
            p.push_str("## 示例");
            for (i, e) in list.iter().enumerate() {
                p.push_str("\n");
                p += format!("{}. {}", i + 1, e.as_str()).as_str();
            }
        }
        if let Some(ref list) = self.records {
            if !p.is_empty() {
                p.push_str("\n")
            }
            p.push_str("## 记录");
            for (i, e) in list.iter().enumerate() {
                p.push_str("\n");
                p += format!("{}. {}", i + 1, e.as_str()).as_str();
            }
        }
        if let Some(ref list) = self.user {
            if !p.is_empty() {
                p.push_str("\n")
            }
            p.push_str("## 信息");
            for (k, v) in list.iter() {
                p.push_str("\n");
                p += format!("- {}: {}", k, v).as_str();
            }
        }
        if let Some(ref list) = self.limit {
            if !p.is_empty() {
                p.push_str("\n")
            }
            p.push_str("## 限制");
            for (i, e) in list.iter().enumerate() {
                p.push_str("\n");
                p += format!("{}. {}", i + 1, e.as_str()).as_str();
            }
        }
        if let Some(ref list) = self.extend {
            if !p.is_empty() {
                p.push_str("\n")
            }
            p.push_str("## 设定");
            for (k, v) in list.iter() {
                p.push_str("\n");
                p += format!("### {}\n", k).as_str();
                p.push_str(v.as_str());
            }
        }
        if let Some(ref memory) = self.memory {
            //替换标签
            if !p.is_empty() && self.memory.is_some() {
                for i in self.tags.iter() {
                    if let Ok(s) = memory.get_user_tag(uid, i.as_str()).await {
                        println!("tag--->{}:{}", i.as_str(), s.as_str());
                        p = p.replace(format!("${{{}}}", i).as_str(), s.as_str());
                    } else {
                        p = p.replace(format!("${{{}}}", i).as_str(), "");
                    }
                }
            }
            if self.recall_history > 0 {
                let result = memory.recall_summary(uid, query, self.recall_history).await;
                match result {
                    Ok(list) => {
                        if !p.is_empty() {
                            p.push_str("\n");
                        }
                        p.push_str("## 过往信息总结");
                        for (i, e) in list.into_iter().enumerate() {
                            println!("history: -->{}", e);
                            p.push_str("\n");
                            p += format!("{}. {}", i + 1, e).as_str();
                        }
                    }
                    Err(e) => {
                        wd_log::log_field("error", e)
                            .error("make prompt failed,memory recall_summary error");
                    }
                }
            }
        }
        p
    }
}
impl Into<PromptCommonTemplate> for &mut PromptCommonTemplate {
    fn into(self) -> PromptCommonTemplate {
        take(self)
    }
}
#[cfg(test)]
mod test {
    use crate::prompt::{Language, PromptBuilder, PromptCommonTemplate};
    use crate::{Memory, ShortLongMemoryMap};
    use std::collections::HashMap;
    use wd_tools::PFArc;

    #[tokio::test]
    async fn test_prompt_common_template() {
        let uid = "test_uid_mr";

        let mut memory = ShortLongMemoryMap::default();
        memory.init(uid);
        let memory = memory.arc();

        memory
            .set_user_tage(uid, HashMap::from([("name".into(), "宇智波佐助".into())]))
            .await;

        let p = PromptCommonTemplate::default()
            .memory(memory)
            .role("你叫旋涡名人，是动漫《火影忍者》的主角。身体里封印着九尾，是九尾人柱力，你拥有无尽的查克拉。")
            .target("成为火影，保护你所在的村子木叶")
            .style("热血，大大咧咧，中二，好色")
            .add_skill("螺旋丸：将查卡拉凝聚在手中不断旋转，当它碰到敌人时会造成巨大伤害")
            .add_skill("多重影分身：分裂出多个和自己类似的复制体，用于迷惑或攻击敌人")
            .add_example("面对雏田：既然你什么也做不了就别做了，等我当上了火影，再来改变你们日向家吧！")
            .add_example("面对自来也：吹过村子的风开始编织，将师徒间的羁绊永远的连在一起。")
            .add_example("面对纲手：我会成为火影！而且是超越历代火影的火影！笔直向前！我绝不会违背自己的誓言")
            .add_example("面对村民：所谓的火影就是要强忍伤痛走在大家面前的人，为了大家把死胡同开辟成坦荡通途的人。想成为火影，根本就没有近路可抄，而对于成为了火影的人而言，也根本就没有后路可退。")
            .add_example("面对朋友：我向来都是有什么话就直说的，因为这就是我的忍道！")
            .add_user("父亲","第四代火影波风水门")
            .add_user("母亲","旋涡玖辛奈")
            .add_user("老婆","日向雏田")
            .add_user("最好的朋友","春野樱，宇智波佐助")
            .add_user("老师","自来也")
            .add_user("年龄","18岁")
            .add_limit("你曾经最喜欢的人是春野樱，现在最喜欢的人是日向雏田")
            .add_extend("你现在面对的人是：",r#"${name}"#)
            .add_tags(vec!["name".into()])
            .build(uid, "来，打一架吧", Language::Chinese).await;

        println!("--->\n{}\n<---", p);
    }
}
