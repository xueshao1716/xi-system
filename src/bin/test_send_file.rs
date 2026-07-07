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

    // ____________________________?    if !std::path::Path::new(&file_path).exists() && args.len() < 2 {
        std::fs::write(&file_path, "________________n______? _________\n________________________\n")
            .map_err(|e| format!("____________: {}", e))?;
        println!("__ _________: {}", file_path);
    }

    // ______ token
    let token_path = format!("{}/wx_token.json", home);
    let mut wl = xi_system::wechat::WeiLink::new();
    if !wl.load_token(&token_path) {
        eprintln!("_?_________ token: {}", token_path);
        std::process::exit(1);
    }
    println!("_?_______?);    // ______?    println!("__ ______? {} -> {}", file_path, to_user);    let result = wl.send_file(&to_user, &file_path, &caption, "").await?;    println!("__ ______? ret={:?}, errcode={:?}", result.ret, result.errcode);    match result.errcode {        Some(-14) => eprintln!("___ ________________?),

        Some(-2) => eprintln!("___ _________60____?),        Some(0) | None if result.ret == Some(0) => println!("_?________"),        _ => eprintln!("___ _______?),

    }

    Ok(())