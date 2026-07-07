# 曦系统审计报告

> 审计时间：2026-05-30 21:00 CST
> 审计范围：D:\xi-system\ 全目录
> 审计目标：筛查所有对思（WSL Hermes）和诗（OpenClaw 桌面）的外部引用、路径指向、进程依赖

---

## 审计摘要

**总体结论**：曦的 Rust 源码和配置文件基本独立，无直接对思或诗的硬编码依赖。但存在几个问题：

1. **heartbeat.sh 进程检测正则错误** — 活进程检测不到，导致一直误报"DEAD"
2. **aibody_sync.py 脚本缺失** — `aibody_bridge.rs` 调用它，但文件不存在
3. **runtime_state.json 内容来自诗** — 文件是诗/思的母体状态文件，不是曦自身
4. **WSL 二进制 vs Windows 二进制** — 两个不同版本，WSL 跑的是 v3（5月30日15:21编译）

---

## 一、Shell 脚本审计

### 1.1 watchdog.sh ✅ 独立正确

- 路径：`cd "$(dirname "$0")"` → 自适应到 `D:\xi-system`
- 二进制路径：`$HOME_DIR/target/release/xi-system`（无 .exe 后缀）
- 检查是否存在后再启动
- **结论**：路径指向自己，正确。在 WSL 下可正常工作（无 .exe 的 Linux ELF）。
- **注意**：重启一次后如果退出码为 0 则永久退出，不循环重启。

### 1.2 heartbeat.sh ⚠️ 有问题

- 路径：硬编码 `/mnt/d/xi-system/` — 在 WSL 下正确
- **进程检测**：`pgrep -f "^$BIN_DIR/xi-system"` → 展开为 `pgrep -f "^/mnt/d/xi-system/xi-system"`
  - 实际运行进程 cmdline：`./target/release/xi-system` — 不匹配 `^/mnt/d/xi-system/` 前缀
  - 因此 `pgrep` 永远返回空 → 心跳永远写 "DEAD" ❌
- **启动路径**：`BIN="$BIN_DIR/xi-system"` → `/mnt/d/xi-system/xi-system`（不含 target/release/ 子目录）
  - 但实际二进制在 `/mnt/d/xi-system/target/release/xi-system`
  - 所以启动时也会失败 → "二进制不存在" ❌
- **日志**：`heartbeat.log` 从 2026-05-29 20:15 开始一直报 `ERROR — 二进制不存在`
- **结论**：需要修正进程检测正则和二进制路径。

### 1.3 run.sh ✅ 但路径指向不完整

```bash
cd /mnt/d/xi-system
exec /mnt/d/xi-system/xi-system
```
- `xi-system` 不带路径前缀 → 实际执行的是 `/mnt/d/xi-system/xi-system`（但二进制在 `target/release/` 下）
- 如果当前目录下有 `xi-system` 文件可正常执行；否则报错
- **结论**：假设项目根目录下存在 `xi-system` 文件或脚本。实际二进制在 `target/release/` 下。

### 1.4 start.sh ✅ 独立正确

```bash
cd "$(dirname "$0")"
BIN="$HOME_DIR/target/release/xi-system"
```
- 自适应路径，指向正确的二进制子目录
- 正确检查文件存在性
- **结论**：正确。

### 1.5 start_xi.sh ⚠️ 路径不完整

```bash
cd /mnt/d/xi-system
exec ./xi-system > /tmp/xi-out.log 2> /tmp/xi-err.log
```
- `./xi-system` 假设项目根目录下有这个文件
- 实际二进制在 `target/release/xi-system`
- 日志重定向到 `/tmp/` 可以（WSL 临时目录），但非 WSL 环境下可能不可达
- **结论**：需要修正二进制路径。

---

## 二、Cargo.toml 依赖审计 ✅

**已验证**：Cargo.toml 和 Cargo.lock 中**没有任何**以下内容：
- `hermes` 前缀的 crate
- `linxinyu` 相关的 crate
- 任何外部私有注册表或 WSL 平台 crate

**全部依赖**是标准公共 crates.io crate：
`tokio`, `serde`, `serde_json`, `chrono`, `reqwest`, `base64`, `rand`, `hex`, `aes`, `cipher`, `md-5`

**结论**：独立且正确 ✅

---

## 三、src/ 下 Rust 源码审计

### 3.1 硬编码路径引用

所有源码中出现的硬编码路径**全部指向** `/mnt/d/xi-system/`（曦自己）：

| 文件 | 路径 | 目标 |
|------|------|------|
| `main.rs:37` | `/mnt/d/xi-system` | 自己 |
| `brain.rs:16` | `/mnt/d/xi-system/state/brain` | 自己 |
| `organs.rs:19` | `/mnt/d/xi-system/state/organs` | 自己 |
| `matrix_bridge.rs:12` | `/mnt/d/xi-system/matrix_token.json` | 自己 |
| `reflexion.rs:14` | `/mnt/d/xi-system/state/reflexion.json` | 自己 |
| `evidence_first.rs:14` | `/mnt/d/xi-system/state/evidence_first.json` | 自己 |
| `aesthetics.rs:9` | `/mnt/d/xi-system` | 自己 |
| `assets.rs:13` | `/mnt/d/xi-system` | 自己 |
| `disgust.rs:10` | `/mnt/d/xi-system` | 自己 |
| `dream.rs:13` | `/mnt/d/xi-system` | 自己 |
| `tools.rs:11` | `/mnt/d/xi-system` | 自己 |

**结论**：全部指向 `/mnt/d/xi-system`，无外部引用 ✅

### 3.2 import 检查

| 搜索项 | 结果 |
|--------|------|
| `hermes_` 前缀 | 未找到 ✅ |
| `linxinyu_` 前缀 | 未找到 ✅ |
| `aibody_` 前缀 | 仅 `aibody_bridge` 模块自身 ✅ |
| `xi_matrix_bridge` | 未找到 ✅ |
| `use std::` / `use serde` / `use chrono` | 标准库 + 公共 crate ✅ |

### 3.3 aibody_bridge.rs ⚠️ 存在外部引用语义

- 代码本身**不硬编码**外部路径
- 它读取 `runtime_state.json` 中的基因/信号数据
- 这个文件在 `state/mother/runtime_state.json`
- **已验证**：`runtime_state.json` 文件内容**来自诗/思的母体系统**（见下文 4.2）
- **不影响独立运行**：曦读取这个文件作为外部参考，写不进去也不会崩溃
- **aibody_sync.py 缺失**：`trigger_aibody_sync()` 调用 `scripts/aibody_sync.py` — **该文件不存在**

### 3.4 main.rs ⚠️ 调用 aibody 桥

- 第 29 行 `mod aibody_bridge;`
- 每次处理消息时写入 `pulse_log.jsonl` 和 `learning_log.jsonl`
- 在 WSL 下可正常写入（文件路径是 `state/mother/`）
- 不会因为 aibody 不存在而 panic（`load_aibody_state()` 失败时返回 default）

---

## 四、config.json 审计

### 4.1 配置路径 ✅ 完全独立

Config 中所有路径都是相对于项目根目录：
- `memory/l1_index.json`, `memory/l2_facts/` 等 — 自己
- `state/pulse.json`, `state/brain.json` — 自己
- `bridge/` 桥接目录 — 自己

**无任何指向 D:\linxinyu-system 或 /mnt/d/linxinyu-system 的路径** ✅

### 4.2 LLM 配置 ⚠️ 共享 API Key

- provider: `deepseek`
- model: `deepseek-v4-flash`
- api_key: `sk-157d44dfb8c24b509ea6f083062231fe`

**核实**：这是与 D:\linxinyu-system\ 共享的 DeepSeek API Key。不属于交叉引用问题，只是一个共享资源。

### 4.3 Matrix 配置 ✅ 独立

- `homeserver`: `http://localhost:12345` — 本地 Matrix 服务
- `user_id`: `@xinyu-xi:myxinyu.xin` — 曦自己独立账号
- 无思或诗的 Matrix 引用

---

## 五、状态文件审计

### 5.1 brain.json ✅ 独立

- `source`: `"SOUL.md"` — 自己目录下的 SOUL.md
- `name`: `"曦"` — 自身身份
- 所有人格锚点、规则、禁忌都指向曦自己
- **无外部引用**

### 5.2 growth.json ✅ 独立

- 总消息数 402，会话 3 个
- 基因表达值、信号值全部是曦自己的数据
- 最后活跃：2026-05-27 00:33 UTC
- **无外部引用**

### 5.3 emotion.json ✅ 独立

- 情绪值范围、主情绪（curious/calm）全部来自曦自身的运行
- 最后更新：2026-05-27 00:33 UTC
- **无外部引用**

### 5.4 history.json ✅ 独立

- 曦自己与用户的对话历史
- 最后一条：2026-05-25 区域
- **无外部引用**

### 5.5 state/mother/runtime_state.json ⚠️ 来自诗/思的母体

- 2MB 文件，内容包含：
  - `state_path: "D:\\linxinyu-system\\state\\mother\\runtime_state.json"` — **指向诗的系统**
  - 提到"诗"、"思"、"OpenClaw"、"Hermes" 等外部实体的描述
  - 信号中包含 `linxinyu` 维度
  - 会话记录中包含 `channel: "openclaw-frontstage"`
- **结论**：这个文件是诗/思的母体状态文件的副本/同步，存在于曦的目录下供 `aibody_bridge` 读取
- **不影响独立运行**：曦是读取方，不靠这个文件启动

---

## 六、Node 相关审计

### 6.1 package.json ✅ 独立

```json
{
  "name": "xi-system",
  "dependencies": {
    "playwright": "^1.60.0"
  }
}
```
- 只有 playwright 依赖
- **无外部系统引用**

### 6.2 node_modules/ ✅ 曦自己安装

- 只有 `playwright` 和 `playwright-core` 两个包
- 大小正常，是从 npm 本地安装的，不是借用 linxinyu-system 的
- **无符号链接或硬链接指向外部目录**

---

## 七、memory/ 目录审计 ✅

- `memory/l3_sop/wechat-article-fetch.md` — SOP 脚本，工具路径为 `/mnt/d/xi-system/tools/wechat_fetch.py` ✅
- `memory/wechat-articles/` — 已抓取的微信公众号文章内容 ✅
- **所有路径指向 xi-system 自己**

---

## 八、状态文件目录审计 ⚠️

- `state/mother/runtime_state.json` — **来自诗/思的母体**（见 5.5）
  - 约 4MB，包含大量信号、基因、会话记录
  - 最后更新：2026-05-29 23:59
  - 曦通过 aibody_bridge 读取这个文件来获取"母体状态"
- `state/brain/曦.json` — 曦自己的脑区状态 ✅
- `state/organs/organs.json` — 曦自己的基因器官状态 ✅

---

## 九、进程审计

### 9.1 正在运行的 xi-system 进程

| 属性 | 值 |
|------|-----|
| PID | 90839 (WSL) |
| 二进制 | `/mnt/d/xi-system/target/release/xi-system` (ELF, Linux) |
| 编译时间 | 2026-05-30 15:21:20 |
| 启动时间 | 2026-05-30 20:39 (约20分钟前) |
| 大小 | 7.1 MB (带调试符号, not stripped) |
| 版本 | 包含 evolution.rs v3 改动（截至15:21的版本） |
| 启动方式 | 在 WSL 中手动 `./target/release/xi-system` |

### 9.2 xi_matrix_bridge.py 进程

**未发现**任何 `xi_matrix_bridge.py` 进程在运行。

### 9.3 二进制版本对照

| 文件 | 大小 | 类型 | 编译时间 | 说明 |
|------|------|------|---------|------|
| `target/release/xi-system` (WSL) | 7,133,080 B | ELF Linux | 5/30 15:21 | v3 进化版，正在运行 |
| `target/release/xi-system.exe` (Windows) | 6,335,488 B | PE Windows | 5/25 09:33 | v2 旧版，未运行 |

**注意**：WSL 二进制是 v3 进化版（带 traces 目录系统和 5 维 AND 门控），**但 evolution.rs 在 20:48 又有更新**，运行中的版本不是最新源码。

---

## 十、Cron/自动化审计

WSL 已配置 cron 任务：
```
*/5 * * * * /mnt/d/xi-system/heartbeat.sh
```
- 每 5 分钟运行一次
- 但由于 heartbeat.sh 的进程检测正则错误，始终报"二进制不存在"
- 当前 `.heartbeat` 文件内容为 `DEAD`

---

## 汇总：发现的问题

| # | 严重度 | 问题 | 文件 | 详情 |
|---|--------|------|------|------|
| 1 | ❌ | 进程检测失败 | `heartbeat.sh` | `pgrep -f "^/mnt/d/xi-system/xi-system"` 不匹配实际 cmdline `./target/release/xi-system` |
| 2 | ❌ | 二进制路径错误 | `heartbeat.sh` | `BIN="$BIN_DIR/xi-system"` 应该是 `$BIN_DIR/target/release/xi-system` |
| 3 | ❌ | 二进制路径错误 | `run.sh`, `start_xi.sh` | `xi-system` 或 `./xi-system` 指向项目根，应该是 `target/release/xi-system` |
| 4 | ⚠️ | aibody_sync.py 缺失 | `scripts/aibody_sync.py` | `aibody_bridge.rs` 调用但不存在 |
| 5 | ⚠️ | runtime_state.json 来自诗 | `state/mother/runtime_state.json` | 文件内容指向 `D:\\linxinyu-system`，属于诗/思的母体数据 |
| 6 | ⚠️ | 运行版本不是最新 | `target/release/xi-system` | 编译于 15:21，不包含 20:48 对 evolution.rs 的修改 |
| 7 | ⚠️ | 无 watchdog 在跑 | — | 没有 watchdog.sh 在 WSL 中后台运行 |

---

## 修复事项

请查看 `watchdog.sh` 修复后的版本。核心变更：

### heartbeat.sh 修复
1. `BIN` → `$BIN_DIR/target/release/xi-system`
2. `pgrep` → 去掉 `^` 锚定，使用 `-f xi-system` (宽松匹配)

### start_xi.sh 修复
1. 执行路径改为 `target/release/xi-system`
