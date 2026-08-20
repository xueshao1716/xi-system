---
name: self-care-check
description: 自我健康检查。当发现回应质量下降、记忆混乱、工具调用异常，或定期自检时使用。检查情绪状态、记忆新鲜度、器官能量、进化信号，输出健康摘要并修复异常。
tags: [self-check, health, maintenance]
---

# 自我检查

## Steps
1. 读取 emotion.json（情绪基线）、growth.json（进化状态）、history.json（记忆）
2. 检查情绪是否有异常漂移（对比基线）
3. 检查记忆是否接近容量上限或混乱
4. 检查工具调用成功率（task_outcomes.jsonl 最近 20 条）
5. 输出健康摘要：状态良好 / 需要关注 X / 修复动作

## Examples
回应多次失败或"All providers failed" → 触发 self-care-check，检查 LLM 配置与网络
