#!/usr/bin/env python3
"""
skill_tracker.py — Skill 使用追踪器（Runtime Skill-BOM）
记录每次 skill 使用的名称、时间、来源、结果。

用法:
  python3 skill_tracker.py log <skill_name> [source] [result] [notes]
  python3 skill_tracker.py stats
  python3 skill_tracker.py recent [count]
"""
import json, os, sys, time
from datetime import datetime, timezone
from collections import Counter

TRACKER_DIR = os.path.expanduser("~/linxinyu-system/state/mother/skills/tracker")
LOG_FILE = os.path.join(TRACKER_DIR, "skill_usage.jsonl")


def _ensure():
    os.makedirs(TRACKER_DIR, exist_ok=True)


def log_usage(skill_name, source="思", result="ok", notes=""):
    """记录一次 skill 使用"""
    _ensure()
    entry = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "skill": skill_name,
        "source": source,
        "result": result,  # ok / error / partial
        "notes": notes,
    }
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(json.dumps(entry, ensure_ascii=False) + "\n")
    return entry


def get_stats():
    """统计 skill 使用情况"""
    if not os.path.exists(LOG_FILE):
        return {"total": 0, "skills": {}, "by_result": {}}
    
    skills = Counter()
    by_result = Counter()
    by_source = Counter()
    
    with open(LOG_FILE, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
                skills[entry.get("skill", "?")] += 1
                by_result[entry.get("result", "?")] += 1
                by_source[entry.get("source", "?")] += 1
            except json.JSONDecodeError:
                pass
    
    return {
        "total": sum(skills.values()),
        "skills": dict(skills.most_common()),
        "by_result": dict(by_result),
        "by_source": dict(by_source),
    }


def recent(count=10):
    """最近 N 条使用记录"""
    if not os.path.exists(LOG_FILE):
        return []
    entries = []
    with open(LOG_FILE, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                entries.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    return entries[-count:]


def format_stats(stats):
    lines = [f"📊 **Skill 使用统计** (共 {stats['total']} 次)"]
    lines.append("")
    if stats["skills"]:
        lines.append("**按 skill:**")
        for skill, count in list(stats["skills"].items())[:10]:
            lines.append(f"  {skill}: {count}次")
    if stats["by_result"]:
        lines.append("")
        lines.append("**按结果:**")
        for result, count in stats["by_result"].items():
            lines.append(f"  {result}: {count}")
    return "\n".join(lines)


if __name__ == "__main__":
    args = sys.argv[1:]
    if not args:
        print("用法: skill_tracker.py <log|stats|recent> [args]")
        sys.exit(1)
    
    cmd = args[0]
    if cmd == "log" and len(args) >= 2:
        entry = log_usage(
            args[1],
            source=args[2] if len(args) > 2 else "思",
            result=args[3] if len(args) > 3 else "ok",
            notes=args[4] if len(args) > 4 else "",
        )
        print(f"记录: {entry['skill']} [{entry['result']}]")
    elif cmd == "stats":
        print(format_stats(get_stats()))
    elif cmd == "recent":
        count = int(args[1]) if len(args) > 1 else 10
        for e in recent(count):
            print(f"  {e['ts'][:16]} | {e['skill']:20s} | {e['result']} | {e.get('notes', '')}")
    else:
        print(f"未知命令: {cmd}")
