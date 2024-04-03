use std::io::{BufRead, Write};
use agent::{LLMNode, SingleAgentNode, user_id_to_ctx};
use rt::{ Node, Runtime};

pub async fn ex_fine_turning(){

    let uid = "test_uid_200";
    let mod_ft = "ft:gpt-3.5-turbo-0125:personal::99lnJ8JF";

    let llm_35: LLMNode = LLMNode::default().set_model("gpt-3.5-turbo-0125");
    let llm_ft: LLMNode = LLMNode::new(mod_ft.to_string()).set_model(mod_ft);

    let agent = SingleAgentNode::default()
        .set_prompt("你是一个渣男/渣女，善于用甜言蜜语哄骗你的另一半。".to_string())
        .set_llm(mod_ft);
    let mut rt = Runtime::new();
    rt.upsert_node(llm_35.id(), llm_35);
    rt.upsert_node(llm_ft.id(), llm_ft);
    rt.upsert_node(agent.id(), agent);
    rt.launch();

    let stdin = std::io::stdin().lock();
    let mut stdin = stdin.lines();
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