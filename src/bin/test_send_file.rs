/// __________?/// ___: cargo run --bin test_send_file <file_path> [caption] [to_user_id]
///
/// ______ o9cq805lqZDMAa3l2RnVXxc71Q8U@im.wechat (___)
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let home = "D:\\xi-system";

    // ___
    let file_path = args.get(1)
        .cloned()
        .unwrap_or_else(|| format!("{}/test_send_file.txt", home));
    let caption = args.get(2).cloned().unwrap_or_default();
    let to_user = args.get(3).cloned()
        .unwrap_or_else(|| "o9cq805lqZDMAa3l2RnVXxc71Q8U@im.wechat".to_string());

    // 文件不存在时创建示例文件
    if !std::path::Path::new(&file_path).exists() && args.len() < 2 {
        std::fs::write(&file_path, "________________n______? _________\n________________________\n")
            .map_err(|e| format!("创建失败: {}", e))?;
        println!("文件路径: {}", file_path);
    }

    // ______ token
    let token_path = format!("{}/wx_token.json", home);
    let mut wl = xi_system::wechat::WeiLink::new();
    if !wl.load_token(&token_path) {
        eprintln!("错误: 读取 token: {}", token_path);
        std::process::exit(1);
    }
    println!("发送文件到 {} -> {}", file_path, to_user);    let result = wl.send_file(&to_user, &file_path, &caption, "").await?;    println!("发送结果 ret={:?}, errcode={:?}", result.ret, result.errcode);    match result.errcode {
        Some(-14) => eprintln!("错误: 文件不存在或过大"),
        Some(-2) => eprintln!("错误: 超时 (60s)"),
        Some(0) | None if result.ret == Some(0) => println!("发送成功"),
        _ => eprintln!("未知错误"),
    }

    Ok(())
}