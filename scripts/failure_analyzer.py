#!/usr/bin/env python3
"""
failure_analyzer.py — HALO 启发的失败模式分析器
扫描经验笔记，按失败类型分组，识别系统性问题。

用法:
  python3 failure_analyzer.py                    # 分析所有笔记
  python3 failure_analyzer.py --recent 7         # 只分析最近7天
  python3 failure_analyzer.py --pattern "timeout" # 搜索特定模式
"""
import json, os, sys, re, time
from datetime import datetime, timezone, timedelta
from collections import Counter, defaultdict

REPLAY_DIR = os.path.expanduser("~/linxinyu-system/state/mother/skills/replay")
XI_REPLAY_DIR = "/mnt/d/xi-system/state/mother/skills/replay"

# 失败模式分类规则
FAILURE_PATTERNS = {
    "timeout": {
        "keywords": ["超时", "timeout", "hang", "卡死", "blocked"],
        "severity": "high",
        "fix_hint": "考虑增加 timeout、拆分任务、或用 background 模式",
    },
    "api_error": {
        "keywords": ["401", "403", "429", "500", "API", "rate limit", "认证", "token"],
        "severity": "medium",
        "fix_hint": "检查 API key、重试逻辑、速率限制",
    },
    "encoding": {
        "keywords": ["UTF-8", "编码", "encoding", "mojibake", "乱码", "decode"],
        "severity": "medium",
        "fix_hint": "统一用 UTF-8，注意 Python 默认编码",
    },
    "path_error": {
        "keywords": ["路径", "path", "No such file", "不存在", "FileNotFoundError"],
        "severity": "medium",
        "fix_hint": "检查路径拼写，用 os.path.exists 预检",
    },
    "network": {
        "keywords": ["网络", "network", "DNS", "连接", "connection", "ECONNREFUSED"],
        "severity": "high",
        "fix_hint": "检查网络连通性、DNS、防火墙",
    },
    "loop": {
        "keywords": ["循环", "loop", "重复", "重复犯错", "又忘了", "cycle"],
        "severity": "critical",
        "fix_hint": "写技能/加 guardrail，不要只靠记忆",
    },
    "config": {
        "keywords": ["配置", "config", "yaml", "json", "格式", "解析失败"],
        "severity": "medium",
        "fix_hint": "验证配置格式，用 yaml.safe_load 检查",
    },
    "permission": {
        "keywords": ["权限", "permission", "denied", "blocked", "拒绝"],
        "severity": "low",
        "fix_hint": "检查文件权限、Docker 用户映射",
    },
}


def load_notes(sources=None, recent_days=None):
    """加载经验笔记"""
    notes = []
    dirs = [REPLAY_DIR]
    if sources == "xi":
        dirs = [XI_REPLAY_DIR]
    elif sources == "all":
        dirs = [REPLAY_DIR, XI_REPLAY_DIR]

    cutoff = None
    if recent_days:
        cutoff = datetime.now(timezone.utc) - timedelta(days=recent_days)

    for d in dirs:
        if not os.path.isdir(d):
            continue
        for fname in os.listdir(d):
            if not fname.endswith(".json") or not fname.startswith("note-"):
                continue
            try:
                with open(os.path.join(d, fname)) as f:
                    note = json.load(f)
                # 时间过滤
                if cutoff:
                    created = note.get("meta", {}).get("created_at", "")
                    if created:
                        try:
                            ts = datetime.fromisoformat(created)
                            if ts.tzinfo is None:
                                ts = ts.replace(tzinfo=timezone.utc)
                            if ts < cutoff:
                                continue
                        except (ValueError, TypeError):
                            pass
                note["_file"] = os.path.join(d, fname)
                note["_source_dir"] = d
                notes.append(note)
            except Exception:
                pass
    return notes


def classify_failures(notes):
    """将笔记中的失败分类到模式"""
    pattern_groups = defaultdict(list)
    unclassified = []

    for note in notes:
        content = note.get("content", {})
        didnt = content.get("what_didnt", "")
        lesson = content.get("lesson", "")
        text = f"{didnt} {lesson}".lower()

        if not text.strip():
            continue

        matched = False
        for pattern_name, pattern_info in FAILURE_PATTERNS.items():
            for kw in pattern_info["keywords"]:
                if kw.lower() in text:
                    pattern_groups[pattern_name].append({
                        "task": content.get("task", ""),
                        "what_didnt": didnt,
                        "lesson": lesson,
                        "created": note.get("meta", {}).get("created_at", ""),
                        "source": note.get("meta", {}).get("source", "?"),
                        "file": note.get("_file", ""),
                    })
                    matched = True
                    break
            if matched:
                break

        if not matched and text.strip():
            unclassified.append({
                "task": content.get("task", ""),
                "what_didnt": didnt,
                "lesson": lesson,
                "created": note.get("meta", {}).get("created_at", ""),
            })

    return dict(pattern_groups), unclassified


def analyze(notes):
    """完整分析"""
    patterns, unclassified = classify_failures(notes)

    # 统计
    total_notes = len(notes)
    notes_with_failures = sum(1 for n in notes if n.get("content", {}).get("what_didnt", "").strip())
    failure_types = Counter()

    for pattern, items in patterns.items():
        failure_types[pattern] = len(items)

    # 找系统性问题（出现 2+ 次的模式）
    systemic = {
        k: v for k, v in patterns.items() if len(v) >= 2
    }

    # 按严重度排序
    severity_order = {"critical": 0, "high": 1, "medium": 2, "low": 3}
    sorted_systemic = sorted(
        systemic.items(),
        key=lambda x: (severity_order.get(FAILURE_PATTERNS[x[0]]["severity"], 9), -len(x[1]))
    )

    return {
        "total_notes": total_notes,
        "notes_with_failures": notes_with_failures,
        "failure_type_counts": dict(failure_types),
        "systemic_patterns": [
            {
                "pattern": name,
                "count": len(items),
                "severity": FAILURE_PATTERNS[name]["severity"],
                "fix_hint": FAILURE_PATTERNS[name]["fix_hint"],
                "examples": items[:3],  # 最多展示3个例子
            }
            for name, items in sorted_systemic
        ],
        "unclassified_count": len(unclassified),
    }


def format_report(result):
    """格式化分析报告"""
    lines = []
    lines.append(f"🔍 **失败模式分析报告**")
    lines.append(f"  总笔记: {result['total_notes']} | 含失败: {result['notes_with_failures']}")
    lines.append(f"  未分类: {result['unclassified_count']}")
    lines.append("")

    if result["failure_type_counts"]:
        lines.append("**失败类型分布:**")
        for ftype, count in sorted(result["failure_type_counts"].items(), key=lambda x: -x[1]):
            sev = FAILURE_PATTERNS[ftype]["severity"]
            icon = {"critical": "🔴", "high": "🟠", "medium": "🟡", "low": "🟢"}.get(sev, "⚪")
            lines.append(f"  {icon} {ftype}: {count}次")
        lines.append("")

    if result["systemic_patterns"]:
        lines.append("**系统性问题（出现2+次）:**")
        for p in result["systemic_patterns"]:
            lines.append(f"  🔁 {p['pattern']} ({p['count']}次, {p['severity']})")
            lines.append(f"     建议: {p['fix_hint']}")
            for ex in p["examples"][:2]:
                task = ex["task"][:50]
                lines.append(f"     - {task}")
        lines.append("")
    else:
        lines.append("✅ 无系统性失败模式\n")

    return "\n".join(lines)


if __name__ == "__main__":
    recent = None
    source = None
    search_pattern = None

    args = sys.argv[1:]
    i = 0
    while i < len(args):
        if args[i] == "--recent" and i + 1 < len(args):
            recent = int(args[i + 1])
            i += 2
        elif args[i] == "--source" and i + 1 < len(args):
            source = args[i + 1]
            i += 2
        elif args[i] == "--pattern" and i + 1 < len(args):
            search_pattern = args[i + 1]
            i += 2
        else:
            i += 1

    notes = load_notes(sources=source, recent_days=recent)

    if search_pattern:
        # 搜索特定模式
        matches = []
        for note in notes:
            content = note.get("content", {})
            text = f"{content.get('what_didnt', '')} {content.get('lesson', '')}"
            if search_pattern.lower() in text.lower():
                matches.append(content)
        print(f"搜索 '{search_pattern}': {len(matches)} 条匹配")
        for m in matches[:5]:
            print(f"  - {m.get('task', '')[:60]}")
    else:
        result = analyze(notes)
        print(format_report(result))
