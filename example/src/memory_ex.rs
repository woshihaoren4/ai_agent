use std::io::{BufRead, Write};
use wd_tools::PFArc;
use agent::{LLMNode, Memory, PromptCommonTemplate, ShortLongMemoryMap, SingleAgentNode, user_id_to_ctx};
use rt::{Node, Runtime};

#[allow(dead_code)]
pub async fn ex_long_short_memory(){
    let uid = "test_uid_003";
    let llm_35: LLMNode = LLMNode::default();
    let pp = "你是一个智能助手，回答要幽默风趣，尽量简洁。\n用户姓名：${name}，年龄:${age}。\n和用户的交互历史：${history}";
    let mut memory = ShortLongMemoryMap::default();
    memory.init(uid);
    let memory = memory.arc();
    let tag_tool = memory.as_user_tag_tool();
    let summery_tool = memory.as_summery_tool();

    //拼装prompt
    let pp = pp.replace("${name}",memory.get_user_tag(uid,"name").await.unwrap().as_str());
    let mut pp = pp.replace("${age}",memory.get_user_tag(uid,"age").await.unwrap().as_str());
    let history = memory.recall_summary(uid, "生日", 5).await.unwrap();
    if !history.is_empty() {
        pp = pp.replace("${history}",history.join(" \n ").as_str());
    }else{
        pp = pp.replace("${history}","");
    }

    let agent = SingleAgentNode::default()
        .set_prompt::<PromptCommonTemplate>(PromptCommonTemplate::default().role(pp.as_str()).into())
        .add_tool(tag_tool.as_openai_tool())
        .add_tool(summery_tool.as_openai_tool())
        .set_memory(memory);

    let mut rt = Runtime::new();
    rt.upsert_node(tag_tool.id(), tag_tool);
    rt.upsert_node(summery_tool.id(), summery_tool);
    rt.upsert_node(llm_35.id(), llm_35);
    rt.upsert_node(agent.id(), agent);
    rt.launch();

    let stdin = std::io::stdin().lock();
    let mut stdin = stdin.lines();
    println!("Prompt:-->{}", pp.as_str());
    print!("User  :-->");
    std::io::stdout().flush().unwrap();
    while let Some(Ok(query)) = stdin.next() {
        let answer: anyhow::Result<String> =
            rt.call("test_single_agent", "single_agent", query,|x|{
                user_id_to_ctx(&x,uid);
                x
            }).await;
        if let Err(ref e) = answer {
            println!("error--->{}", e);
            return;
        }
        println!("AI    :-->{}", answer.unwrap());
        print!("User  :-->");
        std::io::stdout().flush().unwrap();
    }
}