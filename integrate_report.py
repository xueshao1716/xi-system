#!/usr/bin/env python3
"""把 report_protocol 接入 main.rs"""

with open('src/main.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

changes = []
# ── 改动1: 加 mod 声明 ──
# 在 `mod grid_distill;` 后面
for i, line in enumerate(lines):
    if 'mod grid_distill;' in line:
        changes.append((i+1, 'mod report_protocol;\n'))
        break

# ── 改动2: 初始化 reporter ──
# 在 `let mut distiller = ...` 后面
for i, line in enumerate(lines):
    if 'let mut distiller = grid_distill::GridDistiller' in line:
        changes.append((i+1, '    let mut reporter = report_protocol::ReportProtocol::new();\n'))
        break

# ── 改动3: 包裹微信文章抓取 ──
# 在 `let article = tools::fetch_wechat_article` 前后加报告
for i, line in enumerate(lines):
    stripped = line.strip()
    if stripped.startswith('let article = tools::fetch_wechat_article'):
        # 在这行前面加 reporter.start
        indent = line[:len(line) - len(line.lstrip())]
        changes.append((i, f'{indent}reporter.start("扒取公众号文章");\n'))
        # 在这行后面加 checkpoint + done
        changes.append((i+2, f'{indent}reporter.checkpoint("文章内容已获取");\n'))
        changes.append((i+3, f'{indent}reporter.done();\n'))
        break

# ── 改动4: 在 /recall 命令后加 /report 命令 ──
# 找到 /recall 处理块的 continue; 那一行，在后面加 /report 处理
for i, line in enumerate(lines):
    if '/recall' in line and '命令' in line and '查状态' in line:
        # 找到这个块的 continue;
        for j in range(i, min(i+40, len(lines))):
            if lines[j].strip() == 'continue;' and '/recall' in ''.join(lines[i:j+1]):
                # 在 continue; 后面插入 /report 处理
                indent = lines[j][:len(lines[j]) - len(lines[j].lstrip())]
                report_block = f"""\n                        // ── /report 命令：查报告协议状态 ──
                        if lower == "/report" || lower == "报告" {{
                            let status = reporter.status_line();
                            let history: Vec<String> = reporter.recent_history(5).iter()
                                .map(|s| s.summary())
                                .collect();
                            let report_msg = format!(
                                "📋 报告协议状态\\n\
                                 ═══════════════\\n\
                                 {{}}\\n\
                                 📜 最近记录:\\n  - {{}}",
                                status,
                                history.join("\\n  - "),
                            );
                            println!("💬 /report ->\\n{{}}", report_msg);
                            memory.add("assistant", &report_msg);
                            if let Ok(_) = wl.send_text(&user_id, &report_msg, &msg.context_token.unwrap_or_default()).await {{
                                println!("✅ 已发送 (/report)");
                            }}
                            continue;
                        }}
"""
                changes.append((j+1, report_block))
                break
        break

# ── 按行号逆序应用改动 ──
changes.sort(key=lambda x: x[0], reverse=True)
for idx, new_content in changes:
    lines.insert(idx, new_content)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.writelines(lines)

print(f"✅ 完成，共 {len(changes)} 处改动")
