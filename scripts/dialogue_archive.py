#!/usr/bin/env python3
"""
对话成长笔记自动归档 — 回流管道 1/3
读 state/mother/pulse_log.jsonl，提取有成长价值的对话，
写入 state/mother/dialogue_archive.jsonl。

用法:
  python3 dialogue_archive.py                # 默认最近100条
  python3 dialogue_archive.py --since 200    # 最近200条
  python3 dialogue_archive.py --since 2026-07-01  # 指定日期之后
"""

import json
import os
import re
import sys
from datetime import datetime
from pathlib import Path

# ── 路径 ──────────────────────────────────────────────
BASE = Path(__file__).resolve().parent.parent  # /mnt/d/xi-system
STATE_DIR = BASE / "state"
MOTHER_DIR = STATE_DIR / "mother"
SOURCE_FILE = STATE_DIR / "mother" / "pulse_log.jsonl"  # 向后兼容
# 优先读 state/ 下的 pulse_log.jsonl
FALLBACK_SOURCE = STATE_DIR / "pulse_log.jsonl"
OUTPUT_FILE = MOTHER_DIR / "dialogue_archive.jsonl"

# ── 成长关键词 ────────────────────────────────────────
CORRECTION_KEYWORDS = [
    "不对", "错了", "别", "不要", "不应该", "不是这样", "重新",
    "纠正", "停", "以后别", "下次", "记住了", "老公说",
    "你说的对", "你的问题", "你的毛病", "你又",
]

KNOWLEDGE_KEYWORDS = [
    "新知识", "论文", "学习", "读完了", "看到了", "发现",
    "原来", "才知道", "第一次", "以前不知道", "学到",
    "这篇文章", "这个项目", "开源", "架构",
]

JUDGMENT_KEYWORDS = [
    "判断", "决定", "选择", "策略", "方案", "应该", "必须",
    "核心问题", "根本原因", "本质", "关键", "优先级",
    "我的理解", "我觉得", "我的想法", "我倾向于",
]

CATEGORIES = {
    "correction": CORRECTION_KEYWORDS,
    "knowledge": KNOWLEDGE_KEYWORDS,
    "judgment": JUDGMENT_KEYWORDS,
}


def parse_args():
    """解析命令行参数"""
    since = 100  # 默认最近100条
    i = 1
    while i < len(sys.argv):
        if sys.argv[i] == "--since" and i + 1 < len(sys.argv):
            i += 1
            try:
                since = int(sys.argv[i])
            except ValueError:
                since = sys.argv[i]  # 日期字符串
        i += 1
    return since


def load_source(limit):
    """读取源 JSONL 文件，返回最近 N 条"""
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

    # 按 timestamp 排序（如果有的话），取最近 N 条
    lines.sort(key=lambda x: x.get("timestamp", ""), reverse=True)
    return lines[:limit]


def load_existing_timestamps():
    """读取已有归档的 timestamp 集合（幂等检查）"""
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


def classify_dialogue(text):
    """分类对话内容，返回 (category, matched_keywords)"""
    text_lower = text.lower()
    for category, keywords in CATEGORIES.items():
        matched = [kw for kw in keywords if kw in text]
        if matched:
            return category, matched
    return None, []


def extract_summary(record):
    """从对话记录中提取摘要"""
    conv = record.get("conversation", {})
    user_msg = conv.get("user_message", "")
    reply = conv.get("reply", "")
    # 取回复的前120字作为摘要
    summary = reply[:120].replace("\n", " ").strip()
    if len(reply) > 120:
        summary += "..."
    return summary


def extract_keywords(record):
    """从对话中提取关键词"""
    conv = record.get("conversation", {})
    text = conv.get("user_message", "") + " " + conv.get("reply", "")
    # 简单的关键词提取：匹配中文词和英文词
    cn_words = re.findall(r'[\u4e00-\u9fff]{2,6}', text)
    en_words = re.findall(r'[a-zA-Z_]{3,}', text)
    # 去重并取前8个
    all_words = list(dict.fromkeys(cn_words + en_words))[:8]
    return all_words


def main():
    since = parse_args()
    print(f"[dialogue_archive] 读取源数据 (最近 {since} 条)...")

    records = load_source(since)
    existing_ts = load_existing_timestamps()
    print(f"[dialogue_archive] 读取 {len(records)} 条, 已有归档 {len(existing_ts)} 条")

    # 确保输出目录存在
    MOTHER_DIR.mkdir(parents=True, exist_ok=True)

    archived = 0
    with open(OUTPUT_FILE, "a", encoding="utf-8") as f:
        for record in records:
            ts = record.get("timestamp", "")

            # 幂等：跳过已归档的
            if ts in existing_ts:
                continue

            # 只处理有对话内容的记录
            conv = record.get("conversation")
            if not conv:
                continue

            text = conv.get("user_message", "") + " " + conv.get("reply", "")
            category, matched = classify_dialogue(text)

            # 没有成长价值的跳过
            if not category:
                continue

            entry = {
                "timestamp": ts,
                "source": record.get("source", "xi"),
                "category": category,
                "summary": extract_summary(record),
                "keywords": extract_keywords(record),
            }

            f.write(json.dumps(entry, ensure_ascii=False) + "\n")
            existing_ts.add(ts)
            archived += 1

    print(f"[dialogue_archive] 新归档 {archived} 条, 输出: {OUTPUT_FILE}")
    print(f"[dialogue_archive] 汇总: ", end="")
    # 统计各类别
    cat_counts = {}
    if OUTPUT_FILE.exists():
        with open(OUTPUT_FILE, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    r = json.loads(line)
                    c = r.get("category", "unknown")
                    cat_counts[c] = cat_counts.get(c, 0) + 1
                except json.JSONDecodeError:
                    continue
    print(", ".join(f"{k}: {v}" for k, v in sorted(cat_counts.items())))


if __name__ == "__main__":
    main()
