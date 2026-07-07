#!/usr/bin/env python3
"""
replay_player.py — 回放模块
路径: ~/linxinyu-system/scripts/replay_player.py
用法: python3 replay_player.py

由 experience-replay skill 调用。每日 cron 自动触发。
"""
import json, os, random, time, importlib.util
from datetime import datetime, timezone

REPLAY_DIR = os.path.expanduser("~/linxinyu-system/state/mother/skills/replay")
DAILY_DIR = "/mnt/d/xinyu-zool/messages/topics"

def get_notes(min_age_hours=2):
    notes = []
    if not os.path.isdir(REPLAY_DIR): return notes
    now = time.time()
    for fname in os.listdir(REPLAY_DIR):
        if not fname.endswith(".json") or not fname.startswith("note-"): continue
        try:
            with open(os.path.join(REPLAY_DIR, fname)) as f:
                note = json.load(f)
        except: continue
        created = note.get("meta", {}).get("created_at", "")
        if created:
            try:
                if (now - datetime.fromisoformat(created).timestamp()) / 3600 < min_age_hours:
                    continue
            except: pass
        notes.append((os.path.join(REPLAY_DIR, fname), note))
    return notes

def pick_notes(notes, count=2):
    return random.sample(notes, min(count, len(notes)))

def mark_replayed(fpath):
    try:
        with open(fpath) as f: note = json.load(f)
        note["replay_count"] = note.get("replay_count", 0) + 1
        note["last_replayed_at"] = datetime.now(timezone.utc).isoformat()
        with open(fpath, "w") as f: json.dump(note, f, ensure_ascii=False, indent=2)
    except: pass

def format_note(note):
    c = note.get("content", {})
    tags = ", ".join(note.get("tags", []))
    rc = note.get("replay_count", 0)
    lines = [f"📝 **{c.get('task', '')}**", f"  标签: {tags} | 回放次数: {rc+1}"]
    if c.get("what_worked"): lines.append(f"  ✅ {c['what_worked']}")
    if c.get("what_didnt"): lines.append(f"  ❌ {c['what_didnt']}")
    if c.get("lesson"): lines.append(f"  💡 {c['lesson']}")
    return "\n".join(lines)

def replay():
    """HALO 启发的两级回放：先概览失败模式，再智能选笔记深入"""
    notes = get_notes()
    if not notes: return "📭 今日无回放"

    # Level 1: 失败模式概览（HALO trace analysis）
    systemic = []
    try:
        spec = importlib.util.spec_from_file_location(
            "failure_analyzer",
            os.path.expanduser("~/linxinyu-system/scripts/failure_analyzer.py")
        )
        analyzer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(analyzer)
        analysis = analyzer.analyze(notes)
        systemic = analysis.get("systemic_patterns", [])
    except Exception:
        pass

    # Level 2: 智能选笔记（优先有教训+回放少+匹配系统性失败）
    scored = []
    for fpath, note in notes:
        score = 0
        lesson = note.get("content", {}).get("lesson", "")
        didnt = note.get("content", {}).get("what_didnt", "")
        rc = note.get("replay_count", 0)
        text = f"{didnt} {lesson}".lower()
        if lesson: score += 3
        score -= rc
        # 匹配系统性失败模式的笔记优先
        for p in systemic:
            for kw in p.get("pattern", "").split("/"):
                if kw.lower() in text:
                    score += 5
                    break
        scored.append((score, fpath, note))

    scored.sort(key=lambda x: -x[0])
    selected = scored[:2]

    results = []
    if systemic:
        results.append("🔍 **系统性失败模式**")
        for p in systemic[:3]:
            sev_icon = {"critical": "🔴", "high": "🟠", "medium": "🟡"}.get(p["severity"], "⚪")
            results.append(f"  {sev_icon} {p['pattern']}: {p['count']}次 — {p['fix_hint']}")
        results.append("")

    for _, fpath, note in selected:
        results.append(format_note(note))
        mark_replayed(fpath)

    header = f"🧠 **思的今日回放** ({datetime.now().strftime('%Y-%m-%d %H:%M')})\n从 {len(notes)} 条笔记中挑了 {len(selected)} 条复习\n\n"
    output = header + "\n---\n\n".join(results)
    try:
        with open(os.path.join(DAILY_DIR, "tech.md"), "a") as f:
            f.write(f"\n---\n\n## 回放 {datetime.now().strftime('%Y-%m-%d %H:%M')}\n\n{output}\n")
    except: pass
    return output

if __name__ == "__main__":
    print(replay())
