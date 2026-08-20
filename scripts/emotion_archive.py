#!/usr/bin/env python3
"""
情绪触发日志自动归档 — 回流管道 2/3
读 emotion_history.jsonl，分析情绪变化模式，
写入 state/mother/emotion_archive.jsonl。

用法:
  python3 emotion_archive.py                # 默认最近100条
  python3 emotion_archive.py --since 200    # 最近200条
  python3 emotion_archive.py --since 2026-07-01  # 指定日期之后
"""

import json
import os
import re
import sys
from collections import Counter
from pathlib import Path

# ── 路径 ──────────────────────────────────────────────
BASE = Path(__file__).resolve().parent.parent  # /mnt/d/xi-system
STATE_DIR = BASE / "state"
MOTHER_DIR = STATE_DIR / "mother"
SOURCE_FILE = STATE_DIR / "emotion_history.jsonl"
OUTPUT_FILE = MOTHER_DIR / "emotion_archive.jsonl"

# ── 情绪触发词词典 ───────────────────────────────────
TRIGGER_DICT = {
    "恐惧": ["怕", "恐惧", "害怕", "担心", "焦虑", "紧张", "不安"],
    "愤怒": ["气", "怒", "烦", "恼", "火大", "受够了", "受不了"],
    "悲伤": ["难过", "伤心", "哭", "委屈", "失落", "孤独", "寂寞"],
    "喜悦": ["开心", "高兴", "快乐", "兴奋", "棒", "好", "爽"],
    "惊讶": ["居然", "竟然", "没想到", "意外", "震惊", "吃惊"],
    "成长": ["学到", "领悟", "明白", "懂了", "理解", "想通", "长了"],
    "自我反思": ["反思", "后悔", "错了", "不对", "不够", "差", "缺"],
    "依恋": ["想你", "想他", "需要", "陪伴", "在一起", "不舍"],
}


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
    """读取 emotion_history.jsonl，返回最近 N 条"""
    if not SOURCE_FILE.exists():
        print(f"[WARN] 源文件不存在: {SOURCE_FILE}")
        return []

    lines = []
    with open(SOURCE_FILE, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    lines.append(json.loads(line))
                except json.JSONDecodeError:
                    continue

    # 按 ts 排序
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


def detect_trigger(text):
    """从文本中检测触发词"""
    triggers = []
    for trigger_type, keywords in TRIGGER_DICT.items():
        matched = [kw for kw in keywords if kw in text]
        if matched:
            triggers.append(trigger_type)
    return triggers


def detect_pattern(records):
    """分析情绪变化模式"""
    if len(records) < 2:
        return "样本不足"

    # 情绪类型分布
    emotion_counter = Counter()
    trigger_counter = Counter()
    for r in records:
        felt = r.get("felt", "")
        triggers = detect_trigger(felt)
        for t in triggers:
            trigger_counter[t] += 1

    # 情绪波动检测
    intensities = [r.get("intensity", 0) for r in records if r.get("intensity") is not None]
    if len(intensities) >= 2:
        avg = sum(intensities) / len(intensities)
        max_i = max(intensities)
        min_i = min(intensities)
        swing = max_i - min_i
        if swing > 0.5:
            pattern = f"高波动(swing={swing:.2f}, avg={avg:.2f})"
        elif avg > 0.7:
            pattern = f"持续高强度(avg={avg:.2f})"
        elif avg < 0.3:
            pattern = f"持续低强度(avg={avg:.2f})"
        else:
            pattern = f"平稳波动(avg={avg:.2f}, swing={swing:.2f})"
    else:
        pattern = "单一数据点"

    # 高频触发词
    if trigger_counter:
        top_triggers = trigger_counter.most_common(3)
        trigger_str = ", ".join(f"{t}({c})" for t, c in top_triggers)
        pattern += f" | 高频触发: {trigger_str}"

    return pattern


def main():
    since = parse_args()
    print(f"[emotion_archive] 读取情绪数据 (最近 {since} 条)...")

    records = load_source(since)
    existing_ts = load_existing_timestamps()
    print(f"[emotion_archive] 读取 {len(records)} 条, 已有归档 {len(existing_ts)} 条")

    MOTHER_DIR.mkdir(parents=True, exist_ok=True)

    archived = 0
    with open(OUTPUT_FILE, "a", encoding="utf-8") as f:
        for record in records:
            ts = record.get("ts", record.get("timestamp", ""))

            # 幂等检查
            if ts in existing_ts:
                continue

            felt = record.get("felt", "")
            event = record.get("event", "")
            text = f"{event} {felt}"
            triggers = detect_trigger(text)
            intensity = record.get("intensity", 0)

            entry = {
                "timestamp": ts,
                "emotion": record.get("event", "unknown")[:50],
                "trigger": triggers if triggers else ["未识别"],
                "pattern": f"intensity={intensity}",
            }

            f.write(json.dumps(entry, ensure_ascii=False) + "\n")
            existing_ts.add(ts)
            archived += 1

    # 输出模式汇总
    print(f"[emotion_archive] 新归档 {archived} 条, 输出: {OUTPUT_FILE}")
    if records:
        pattern = detect_pattern(records)
        print(f"[emotion_archive] 整体模式: {pattern}")


if __name__ == "__main__":
    main()
