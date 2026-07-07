# 曦进化引擎 v3 改动手册

> 基于 Meta-Harness 论文（arXiv:2603.28052）和 darwin-loop 赛博达尔文5维AND门控的升级。
> 改动日期：2026-05-30 | 文件：`src/evolution.rs`

---

## 改动总览

| 改动项 | 原状态 | 新状态 |
|--------|--------|--------|
| 提案存储 | 只有内存 Vec<Proposal> | 每次提案自动建 `evolution_traces/iteration_NNNN/{code, scores, traces/}` 目录 |
| 评估方式 | 单分阈值（随机或外部输入） | 5 维 AND 门控（format/content/behavior/performance/safety） |
| 回滚机制 | 无显式回滚（拒绝后基线不动） | 真·回滚 + 拒绝失败 trace 保留到 filesystem |
| 历史可查 | 只能读 proposals 数组 | 新增 7 个 filesystem 操作 API，支持 grep/cat 主动查询迭代目录 |
| 提案 ID | `prop-N` 递增 | `prop-iteration_counter` 对齐目录编号 |

---

## 改动 1：Proposal 结构体新增字段

```rust
// 新增字段
pub gate_scores: HashMap<String, f64>,    // 5 维门控评分
pub iteration_dir: Option<String>,         // trace 目录路径
pub failure_trace: Option<String>,          // 拒绝后的失败 trace
```

**兼容性**：旧 JSON 读取时这些字段自动为 `None`/空 HashMap，`load()` 的 `unwrap_or_else` 容错。

---

## 改动 2：EvolutionState 新增字段

```rust
pub iteration_counter: usize,  // 迭代计数器（用于 trace 目录编号）
```

旧状态反序列化时这个字段缺省为 0，从 1 开始对外创建。

---

## 改动 3：trace 目录系统（新增 7 个方法）

### `traces_root() -> PathBuf`
- 取 `XI_EVOLUTION_TRACES_DIR` 环境变量，默认 `evolution_traces/`
- 所有 trace 目录建在此根目录下

### `create_iteration_dir(iteration) -> io::Result<PathBuf>`
创建目录树：
```
evolution_traces/
  iteration_0001/
    code/
      baseline_snapshot.json
    scores/
      gate_scores.json
    traces/
      20260530_203000_propose.log
      20260530_203005_gate_eval.log
      20260530_203010_failure_reason.log  (拒绝时才有)
  iteration_0002/
    ...
```

### `snapshot_baseline_to_iteration(dir)`
写当前完整基因状态快照（baseline + adjustments + signals + generation + growth）到 `code/baseline_snapshot.json`

### `write_gate_scores(dir, gate_scores, verdict)`
写门控评分 + 判决结果到 `scores/gate_scores.json`

### `write_trace_entry(dir, step_name, content)`
写任意内容到 `traces/{timestamp}_{step_name}.log`

### `load_iteration_snapshot(iteration)`（静态方法）
从某轮迭代目录读取 `code/baseline_snapshot.json`，返回 `Option<Value>`

### `list_iteration_dirs()`（静态方法）
扫描 `traces_root/` 下所有 `iteration_NNNN` 目录，返回排序后的编号数组

### `load_previous_gate_scores(iteration)`（静态方法）
读取某轮迭代的 `scores/gate_scores.json`，用于 diagnostic 参考

---

## 改动 4：5 维 AND 门控（替代单分评估）

```rust
pub fn multi_gate_evaluate(&self, proposal_id: &str) -> (HashMap<String, f64>, bool)
```

| 维度 | 门控规则 | 满分条件 |
|------|---------|---------|
| format（格式） | proposed_value 在 [0, 1] 内 | 1.0 |
| content（内容） | direction 为 up/down，delta ∈ (0.005, 0.5] | 1.0 |
| behavior（行为） | direction 与 proposed>old / <old 一致 | 1.0 |
| performance（性能） | delta ≤ 0.15（防过度变异） | 1.0 |
| safety（安全） | proposed ∈ [0.1, 0.95] 且 delta ≤ 0.2 | 1.0 |

**全部 5 维得分 = 1.0 才通过**，任一 Fail 自动拒绝 + 回滚。

---

## 改动 5：真·回滚机制

**原行为**：拒绝提案后不碰基线，不记录任何回滚信息。

**新行为**：
1. 拒绝：基线不动（隐式回滚到 `old_value`）
2. 在 `gene_adjustments` 中标记"尝试过但被拒绝"
3. 失败 trace 写入 `traces/failure_reason.log`
4. 保留在 filesystem 中供后续诊断 grep 学习

---

## 改动 6：API 签名变更

### `propose_mutation(gene_key, direction, reason) -> String`
- **新增行为**：自动递增 `iteration_counter`，调用 `create_iteration_dir` 建目录
- **新增行为**：写入 `baseline_snapshot.json` + propose trace log
- 返回 `prop-N` 的形式不变，但 N 现在对齐 `iteration_counter`

### `evaluate_proposal(proposal_id) -> (bool, HashMap<String, f64>)`
- **签名变更**：原 `(proposal_id, score: f64) -> bool` → 移除 score 入参，改为内部自动运行门控
- 返回从 `bool` 变为 `(bool, HashMap<String, f64>)`——第二个值为门控各维度得分

### `resolve_proposal(proposal_id) -> bool`
- **新增行为**：拒绝时显式打印"回滚到 old_value"
- 返回 boolean 不变

---

## 调用方升级指南

如果其他模块调用了以下函数，需要更新：

### `evaluate_proposal` 调用方

**旧代码**：
```rust
let ok = state.evaluate_proposal("prop-1", 0.85);
```

**新代码**：
```rust
// 不再传 score，内部自动 5 维门控
let (ok, gates) = state.evaluate_proposal("prop-1");
// gates = {"format": 1.0, "content": 1.0, "behavior": 1.0, "performance": 0.0, "safety": 1.0}
```

### 如需灵活阈值

`RATCHET_THRESHOLD` 常量（0.7）是综合门控均值阈值。如需外部配置：

```rust
// 读取环境变量
let threshold: f64 = std::env::var("XI_RATCHET_THRESHOLD")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(Self::RATCHET_THRESHOLD);
```

---

## 文件改动检查清单

- [x] `src/evolution.rs` — 核心实现（已改写）
- [ ] 验证 `cargo build` 通过
- [ ] 验证 `cargo test` 通过（如果存在测试）
- [ ] 验证 `state.load()` 兼容旧 JSON（新增字段可选）
- [ ] 验证 `evaluate_proposal` 调用方更新
- [ ] 检查是否需要在 `.gitignore` 中添加 `evolution_traces/`
- [ ] 生成初始证明：`mkdir evolution_traces/iteration_0000/`

---

## 回滚方案

如果升级后有问题，恢复旧版：

```bash
cd D:\xi-system
copy src\evolution.rs.bak src\evolution.rs
# 然后 cargo build 重建
```

备份文件位置：`D:\xi-system\src\evolution.rs.bak`

---

## 来源

- Meta-Harness 论文 (arXiv:2603.28052): 完整 trace filesystem + proposer 主动查询
- darwin-loop 5 维 AND 门控: format/content/behavior/performance/safety
- Trace 目录结构: 论文表 1 的 `iteration_N/{code, scores, traces/}` 设计
