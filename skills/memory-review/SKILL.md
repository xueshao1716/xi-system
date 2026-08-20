---
name: memory-review
description: 定期整理记忆库。当对话变多、记忆混乱、或用户提到忘记过去的事时使用。扫描 history.json 与 MEMORY.md，提炼重要关系细节与用户偏好，合并重复条目，归档过时信息。
tags: [memory, organize, reflection]
---

# 记忆整理

## Steps
1. 读取当前记忆概况（history.json 条目数、最近 20 条对话）
2. 识别值得长期保留的信息：用户偏好、重要约定、关系里程碑、情绪事件
3. 检查 MEMORY.md 是否有重复或过时条目，提出合并/归档建议
4. 用 write_file 记录整理结果到 memory/review-notes.md
5. 向用户简要汇报整理了什么

## Examples
用户说"你还记得我之前说过的事吗" → 触发 memory-review，先搜记忆再回答
