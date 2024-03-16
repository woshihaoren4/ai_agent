use async_openai::Client;
use async_openai::types::CreateEmbeddingRequestArgs;
use wd_tools::PFOk;

pub fn cosine_similarity(v1:&[f32], v2:&[f32]) ->f32{
    let dot_product = v1.iter().zip(v2).map(|(x,y)|x*y).sum::<f32>();
    let mag_product = v1.iter().map(|x|x*x).sum::<f32>().sqrt() * v2.iter().map(|x|x*x).sum::<f32>().sqrt();
    return dot_product/mag_product
}

pub fn top_n(des:&[f32],src:&Vec<Vec<f32>>,n:usize)->Vec<usize>{
    let mut cos_src = src.iter().map(|x|cosine_similarity(des, x.as_slice())).enumerate().collect::<Vec<(usize, f32)>>();
    cos_src.sort_by(|a,b|a.1.partial_cmp(&b.1).unwrap());
    let list = cos_src.iter().rev().enumerate().filter(|i|i.0<n).map(|(i,(x,_))|*x).collect::<Vec<usize>>();
    return list
}

pub async fn embedding_small_1536(query:Vec<&str>)->anyhow::Result<Vec<Vec<f32>>>{
    let len = query.len();
    let req= CreateEmbeddingRequestArgs::default()
        .model("text-embedding-3-small")// text-embedding-3-small:1536
        .input(query)
        .build()?;
    let client = Client::new();
    let resp = client.embeddings().create(req).await?;
    let mut list = vec![vec![];len];
    for i in resp.data{
        list[i.index as usize] = i.embedding
    }
    return list.ok()
}

#[cfg(test)]
mod test{
    use crate::infra::{embedding_small_1536, top_n};

    #[tokio::test]
    async fn test_embedding(){
        let query = vec![
            "自古多情空余恨，此恨绵绵无绝期。",
            "春眠不觉晓，处处闻啼鸟。夜来风雨声，花落知多少。",
        ];

        let result = embedding_small_1536(query).await.unwrap();
        for (i,v) in result.iter().enumerate(){
            println!("[{}]-->{:?}",i,v);
        }
    }

    #[test]
    fn test_top_n(){
        let des = vec![0.1f32,0.2f32];
        let src = vec![vec![0.3,0.2],vec![0.3,0.4],vec![0.2,0.1]];
        let result = top_n(&des, &src, 2);
        println!("--->{:?}",result);
    }
}