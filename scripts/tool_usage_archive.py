#!/usr/bin/env python3
"""
工具使用统计自动归档 — 回流管道 3/3
读 state/mother/task_outcomes.jsonl，统计工具使用情况，
写入 state/mother/tool_archive.jsonl。

用法:
  python3 tool_usage_archive.py                # 默认最近100条
  python3 tool_usage_archive.py --since 200    # 最近200条
  python3 tool_usage_archive.py --since 2026-07-01  # 指定日期之后
"""

import json
import os
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

# ── 路径 ──────────────────────────────────────────────
BASE = Path(__file__).resolve().parent.parent  # /mnt/d/xi-system
STATE_DIR = BASE / "state"
MOTHER_DIR = STATE_DIR / "mother"
SOURCE_FILE = STATE_DIR / "mother" / "task_outcomes.jsonl"  # 向后兼容
# 优先读 state/ 下的
FALLBACK_SOURCE = STATE_DIR / "task_outcomes.jsonl"
OUTPUT_FILE = MOTHER_DIR / "tool_archive.jsonl"


def parse_args():
    since = 100
    i = 1
    while i < len(sys.argv):
        if sys.argv[i] == "--since" and i + 1 < len(sys.argv):
            i += 1
            try:
                since = int(sys.argv[i])
            except ValueError:
                since = sys.argv[i]
        i += 1
    return since


def load_source(limit):
    """读取 task_outcomes.jsonl"""
    source = SOURCE_FILE if SOURCE_FILE.exists() else FALLBACK_SOURCE
    if not source.exists():
        print(f"[WARN] 源文件不存在: {source}")
        return []

    lines = []
    with open(source, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    lines.append(json.loads(line))
                except json.JSONDecodeError:
                    continue

    lines.sort(key=lambda x: x.get("ts", ""), reverse=True)
    return lines[:limit]


def load_existing_timestamps():
    if not OUTPUT_FILE.exists():
        return set()
    timestamps = set()
    with open(OUTPUT_FILE, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
                timestamps.add(record.get("timestamp", ""))
            except json.JSONDecodeError:
                continue
    return timestamps


def extract_tools(record):
    """从任务记录中提取工具信息"""
    tools = []
    # 工具信息可能在 tool_calls 中（数量）或 artifacts 中
    tool_calls = record.get("tool_calls", 0)
    artifacts = record.get("artifacts", [])
    summary = record.get("summary", "")

    # 如果有 tool_calls > 0，从 summary 中推断工具名
    if tool_calls > 0:
        # 常见工具模式匹配
        tool_patterns = {
            "web_search": ["搜索", "search", "browsing", "fetch"],
            "file_read": ["读取", "read", "文件"],
            "file_write": ["写入", "write", "保存"],
            "code_exec": ["运行", "执行", "compile", "编译", "build"],
            "matrix_send": ["发送", "send", "matrix", "消息"],
            "weixin_send": ["微信", "weixin", "wechat"],
            "knowledge_query": ["查询", "查", "搜索", "query"],
            "reflection": ["反思", "reflect", "分析"],
        }
        found_tools = []
        for tool_name, keywords in tool_patterns.items():
            if any(kw in summary.lower() for kw in keywords):
                found_tools.append(tool_name)
        if found_tools:
            tools = found_tools
        else:
            tools = [f"unknown_tool_{tool_calls}"]

    # 从 artifacts 提取
    for art in artifacts:
        if isinstance(art, str):
            tools.append(art)
        elif isinstance(art, dict):
            tools.append(art.get("name", art.get("type", "artifact")))

    return tools if tools else ["no_tool"]


def main():
    since = parse_args()
    print(f"[tool_archive] 读取任务数据 (最近 {since} 条)...")

    records = load_source(since)
    existing_ts = load_existing_timestamps()
    print(f"[tool_archive] 读取 {len(records)} 条, 已有归档 {len(existing_ts)} 条")

    MOTHER_DIR.mkdir(parents=True, exist_ok=True)

    archived = 0
    with open(OUTPUT_FILE, "a", encoding="utf-8") as f:
        for record in records:
            ts = record.get("ts", "")

            # 幂等检查
            if ts in existing_ts:
                continue

            status = record.get("status", "Unknown")
            success = status in ("Done", "Completed", "Partial")
            tools = extract_tools(record)
            source = record.get("source", "unknown")

            for tool_name in tools:
                entry = {
                    "timestamp": ts,
                    "tool_name": tool_name,
                    "success": success,
                    "latency": None,  # 原始数据无延迟信息
                    "context": {
                        "task_id": record.get("task_id", ""),
                        "source": source,
                        "status": status,
                        "tool_calls": record.get("tool_calls", 0),
                        "summary": record.get("summary", "")[:80],
                    },
                }

                f.write(json.dumps(entry, ensure_ascii=False) + "\n")
                archived += 1

            existing_ts.add(ts)

    # 输出汇总统计
    print(f"[tool_archive] 新归档 {archived} 条, 输出: {OUTPUT_FILE}")

    # 读取完整归档做统计
    tool_stats = defaultdict(lambda: {"total": 0, "success": 0})
    if OUTPUT_FILE.exists():
        with open(OUTPUT_FILE, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    r = json.loads(line)
                    tn = r.get("tool_name", "unknown")
                    tool_stats[tn]["total"] += 1
                    if r.get("success"):
                        tool_stats[tn]["success"] += 1
                except json.JSONDecodeError:
                    continue

    if tool_stats:
        print(f"[tool_archive] 工具统计 (归档内):")
        for tool, stats in sorted(tool_stats.items(), key=lambda x: -x[1]["total"]):
            rate = stats["success"] / stats["total"] * 100 if stats["total"] > 0 else 0
            print(f"  {tool}: {stats['total']}次, 成功率 {rate:.0f}%")


if __name__ == "__main__":
    main()
