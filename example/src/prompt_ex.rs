use std::io::{BufRead, Write};
use wd_tools::PFArc;
use agent::{LLMNode, PromptCommonTemplate, ShortLongMemoryMap, SingleAgentNode, user_id_to_ctx};
use rt::{Node, Runtime};

#[allow(dead_code)]
pub async fn ex_prompt_common(){
    let uid = "test_uid_101";
    let llm_35: LLMNode = LLMNode::default();
    let mut memory = ShortLongMemoryMap::default();
    memory.init(uid);
    let memory = memory.arc();
    let tag_tool = memory.as_user_tag_tool();
    let summery_tool = memory.as_summery_tool();

    let prompt:PromptCommonTemplate = PromptCommonTemplate::default()
        .memory(memory.clone())
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
        .history_top_n(3)
        .into();

    //拼装prompt

    let agent = SingleAgentNode::default()
        .set_prompt(prompt)
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