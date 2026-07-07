# 曦 (xi-system) 体检报告 2026-06-09

**检查目录**: `D:\xi-system`

**总文件数**: 1881

**总大小**: 1436.8 MB

**Rust 代码行数**: 11668

## 1. 目录结构

| 模块 | 文件数 | 大小(MB) |
|------|--------|----------|
| target | 4239 | 1647.7 |
| memory | 203 | 0.3 |
| node_modules | 99 | 12.9 |
| src | 48 | 0.5 |
| projects | 36 | 0.2 |
| scripts | 12 | 14.7 |
| state | 7 | 0.1 |
| tools | 3 | 0.0 |
| docs | 2 | 0.0 |
| tests | 2 | 0.0 |

## 2. 核心文件

- ✅ **config.json** (1,752 bytes)
- ✅ **brain.json** (1,351 bytes)
- ✅ **heartbeat.log** (28,106 bytes)
- ✅ **emotion.json** (5,998 bytes)
- ❌ **reflexion.json**: 缺失
- ✅ **history.json** (592,255 bytes)
- ✅ **emotion_history.jsonl** (390 bytes)
- ✅ **proactive.json** (152 bytes)
- ✅ **growth.json** (33,917 bytes)
- ✅ **dreams.json** (4,796 bytes)
- ✅ **IDENTITY.md** (472 bytes)
- ✅ **SOUL.md** (4,376 bytes)
- ✅ **MEMORY.md** (4,757 bytes)
- ✅ **Cargo.toml** (576 bytes)

## 3. Rust 源码

- Rust 文件: 36

- 总代码行数: 11,668

| 模块 | 行数 |
|------|------|
| aesthetics.rs | 155 |
| agent_loop.rs | 477 |
| aibody_bridge.rs | 249 |
| assets.rs | 154 |
| brain.rs | 89 |
| broker.rs | 150 |
| ctx2soft.rs | 1574 |
| curl_http.rs | 91 |
| direct_llm.rs | 47 |
| disgust.rs | 154 |
| dream.rs | 278 |
| emotion.rs | 305 |
| evidence_first.rs | 311 |
| evolution.rs | 734 |
| eyes.rs | 386 |
| grid_distill.rs | 147 |
| grn.rs | 300 |
| heartbeat.rs | 34 |
| lib.rs | 27 |
| main.rs | 541 |
| matrix_bridge.rs | 352 |
| memory.rs | 479 |
| organs.rs | 465 |
| poller.rs | 211 |
| proactive.rs | 127 |
| reflexion.rs | 342 |
| repair.rs | 260 |
| report_protocol.rs | 191 |
| router.rs | 187 |
| rules.rs | 22 |
| scenario.rs | 501 |
| soul.rs | 179 |
| throat.rs | 218 |
| token_budget.rs | 282 |
| tools.rs | 905 |
| wechat.rs | 705 |

## 4. 数据盘点

- **history.json**: 2 条记录 (592,255 bytes)
- **emotion_history.jsonl**: 2 条
- **memory/**: 203 文件

## 5. 配置与心跳

- config.json: 9 个配置项
  - LLM provider: agnes
  - LLM endpoint: N/A
  - API key: ✅ 有
- .heartbeat: 2026-06-08 11:50:04 OK pid=52157
- .last_sync: 2026-05-27T16:00:00+08:00

## 6. 启动文件

- ✅ start.sh (700 bytes)
- ✅ start_xi.sh (101 bytes)
- ✅ run.sh (64 bytes)
- ✅ heartbeat.sh (1,205 bytes)
- ✅ watchdog.sh (941 bytes)
- ❌ launch_xi.py
- ❌ ensure_xi_running.py

## 7. 日志

- xi-err.log: 112 bytes
- xi-out.log: 232,515 bytes
- heartbeat.log: 28,106 bytes

## 8. 综合评分

| 维度 | 评分 | 说明 |
|------|------|------|
| 陪伴 | ? | 需分析 emotion + history |

| 理解 | ? | 需分析 router + reflexion |

| 判断力 | ? | 需分析 config + behavior |

| 自主性 | ? | 需分析 proactive + heartbeat |


---
*由 OpenClaw 自动生成*
