use async_openai::Client;
use async_openai::types::{CreateImageRequestArgs, ImageModel, ImageSize, ResponseFormat};

pub async fn test2image(prompt:impl Into<String>)->anyhow::Result<()>{
    let req = CreateImageRequestArgs::default()
        .prompt(prompt.into())
        .n(1)
        .response_format(ResponseFormat::B64Json)
        // .size(ImageSize::S512x512)
        .user("test")
        .model(ImageModel::DallE3)
        .build()?;
    let resp = Client::new().images().create(req).await?;
    let paths = resp.save("./img").await?;
    paths
        .iter()
        .for_each(|path| println!("Image file path: {}", path.display()));

    Ok(())
}

#[cfg(test)]
mod test{
    use crate::test2image;

    #[tokio::test]
    async fn test_text_2_image(){
        test2image("图片类似：表情包。风格：Q版。人物：熊猫。画风：黑白色。表情：贱兮兮并且坏笑。动作：捂嘴笑").await.unwrap();
    }
}