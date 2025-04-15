use crate::texture::Texture;

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    let path = std::env::current_dir()?
        .join("res")
        .join("cube")
        .join(file_name);
    println!("load string path = {}", path.to_str().unwrap());
    let txt = std::fs::read_to_string(path)?;

    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    let path = std::env::current_dir()?
        .join("res")
        .join("cube")
        .join(file_name);
    println!("load binary path = {}", path.to_str().unwrap());
    let data = std::fs::read(path)?;

    Ok(data)
}

pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<Texture> {
    let data = load_binary(file_name).await?;
    Texture::from_bytes(device, queue, &data, file_name)
}
