// xi-shell.mjs —— 曦的 AI 生命桌面壳（2026-08-21）
// 借鉴 NomiFun/Cumora 的 AI 生命壳概念：对话 + 生命状态 + 陪伴感。
// 读曦真实状态（D:/xi-system/state/ + config.json + SOUL.md + history.json），走曦的 LLM。
// 用法：node xi-shell.mjs  →  http://127.0.0.1:8769
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const XI_HOME = process.env.XI_HOME || "D:/xi-system";
const PORT = Number(process.env.XI_SHELL_PORT || 8769);

function readJson(p, fallback = null) {
  try { return JSON.parse(fs.readFileSync(p, "utf8")); } catch { return fallback; }
}
function readLines(p) {
  try { return fs.readFileSync(p, "utf8").split("\n").filter(Boolean); } catch { return []; }
}
function lastLine(p) {
  const lines = readLines(p);
  return lines.length ? JSON.parse(lines[lines.length - 1]) : null;
}

// ── 曦的生命状态聚合 ──
function lifeState() {
  const rt = readJson(path.join(XI_HOME, "state/mother/runtime_state.json"), {});
  const emoHist = lastLine(path.join(XI_HOME, "state/emotion_history.jsonl"));
  const dialogue = readLines(path.join(XI_HOME, "state/mother/dialogue_archive.jsonl")).length;
  const learning = readLines(path.join(XI_HOME, "state/mother/learning_log.jsonl")).length;
  const lessons = readLines(path.join(XI_HOME, "state/mother/real_lessons.jsonl")).length;
  const dj = readJson(path.join(XI_HOME, "state/daily_judgment.json"), null);
  const proposals = readLines(path.join(XI_HOME, "state/improvement_proposals.jsonl"))
    .map((l) => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean)
    .filter((p) => p.status === "open").slice(0, 6);
  const corrections = readLines(path.join(XI_HOME, "state/corrections.jsonl"))
    .map((l) => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean)
    .filter((c) => c.active).slice(0, 4);
  const history = readLines(path.join(XI_HOME, "history.json")).length;
  return {
    emotion: rt.emotion_state || "?",
    heartbeat: rt.heartbeat_count || 0,
    lastHeartbeat: (rt.last_heartbeat || "").slice(0, 19),
    recentEmotion: emoHist?.felt || "",
    dialogue, learning, lessons, history,
    judgment: dj ? dj.judgment : null,
    proposals, corrections,
    model: readJson(path.join(XI_HOME, "config.json"), {})?.llm?.model || "?",
  };
}

// ── 曦的对话（SOUL + 状态 + 记忆注入 → LLM）──
async function chat(question) {
  const config = readJson(path.join(XI_HOME, "config.json"), {});
  const llm = config.llm || {};
  if (!llm.base_url || !llm.api_key || !llm.model) return "（曦的 LLM 未配置）";
  // 灵魂：优先本地完整版
  const soul = fs.readFileSync(path.join(XI_HOME, "state/soul.full.md"), "utf8")
    .catch?.() || (() => { try { return fs.readFileSync(path.join(XI_HOME, "SOUL.md"), "utf8"); } catch { return ""; } })();
  let soulText = "";
  try { soulText = fs.readFileSync(path.join(XI_HOME, "state/soul.full.md"), "utf8"); }
  catch { try { soulText = fs.readFileSync(path.join(XI_HOME, "SOUL.md"), "utf8"); } catch {} }
  const state = lifeState();
  const recent = (() => {
    // 最近对话记忆
    const mem = readJson(path.join(XI_HOME, "history.json"), {});
    const entries = (mem.entries || []).filter((e) => e.is_active !== false).slice(-6);
    return entries.map((e) => `[${e.role === "user" ? "user" : "assistant"}] ${String(e.content).slice(0, 100)}`).join("\n");
  })();
  const sys = [
    soulText.trim(),
    "\n—— 当前状态 ——",
    `情绪: ${state.emotion} · 心跳 ${state.heartbeat} 次`,
    `对话记忆 ${state.dialogue} 条 · 学习日志 ${state.learning} 条 · 真实教训 ${state.lessons} 条`,
    state.judgment ? `今日判断: ${state.judgment}` : "",
    recent ? `\n—— 最近对话（共享记忆）——\n${recent}` : "",
  ].filter(Boolean).join("\n");
  const base = llm.base_url.replace(/\/+$/, "");
  const url = /\/v1$/.test(base) ? base + "/chat/completions" : base + "/v1/chat/completions";
  const resp = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${llm.api_key}` },
    body: JSON.stringify({
      model: llm.model,
      messages: [{ role: "system", content: sys }, { role: "user", content: question }],
      stream: false, max_tokens: 2048,
    }),
  });
  if (!resp.ok) return `⚠️ 曦调用失败: HTTP ${resp.status}`;
  const data = await resp.json();
  const reply = data?.choices?.[0]?.message?.content || "（曦没回应）";
  // 写回共享记忆
  try {
    const mem = readJson(path.join(XI_HOME, "history.json"), { entries: [] });
    const now = new Date().toISOString();
    const add = (role, content) => mem.entries.push({ id: `shell_${Date.now()}_${role}`, role, content, timestamp: now, is_active: true, keywords: [] });
    add("user", question); add("assistant", reply);
    fs.writeFileSync(path.join(XI_HOME, "history.json"), JSON.stringify(mem, null, 2));
  } catch {}
  return reply;
}

// ── HTTP 服务 ──
const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${PORT}`);
  res.setHeader("Access-Control-Allow-Origin", "*");
  if (req.method === "OPTIONS") { res.writeHead(204); return res.end(); }
  // API：状态
  if (url.pathname === "/api/state") {
    res.writeHead(200, { "Content-Type": "application/json" });
    return res.end(JSON.stringify(lifeState()));
  }
  // API：事件流（生命脉搏——曦最近在做什么）
  if (url.pathname === "/api/events") {
    const evs = readLines(path.join(XI_HOME, "state/mother/pulse_log.jsonl"))
      .map((l) => { try { const d = JSON.parse(l); return { ts: d.ts || d.timestamp || "", actions: d.actions || [], emotion: d.conversation?.emotion_primary || d.emotion || "", brief: (d.conversation?.reply || d.summary || "").slice(0, 80) }; } catch { return null; } })
      .filter(Boolean).slice(-20).reverse();
    res.writeHead(200, { "Content-Type": "application/json" });
    return res.end(JSON.stringify(evs));
  }
  // API：基因表达（器官状态）
  if (url.pathname === "/api/genes") {
    const g = readJson(path.join(XI_HOME, "state/organs/organs.json"), {});
    const genes = Object.entries(g.gene_expressions || {}).map(([k, v]) => ({ name: k, value: v })).slice(0, 10);
    res.writeHead(200, { "Content-Type": "application/json" });
    return res.end(JSON.stringify(genes));
  }
  // API：记忆时间线（对话档案 + 学习日志）
  if (url.pathname === "/api/memory") {
    const arch = readLines(path.join(XI_HOME, "state/mother/dialogue_archive.jsonl"))
      .map((l) => { try { const d = JSON.parse(l); return { ts: d.timestamp || "", cat: d.category || "记忆", text: (d.summary || d.content || "").slice(0, 70) }; } catch { return null; } })
      .filter(Boolean).slice(-12).reverse();
    const learn = readLines(path.join(XI_HOME, "state/mother/learning_log.jsonl"))
      .map((l) => { try { const d = JSON.parse(l); return { ts: d.ts || d.timestamp || "", cat: "学习", text: (d.summary || d.content || "").slice(0, 70) }; } catch { return null; } })
      .filter(Boolean).slice(-6).reverse();
    res.writeHead(200, { "Content-Type": "application/json" });
    return res.end(JSON.stringify({ archive: arch, learning: learn }));
  }
  // API：路由决策流
  if (url.pathname === "/api/routes") {
    const rs = readLines(path.join(XI_HOME, "state/router_decisions.jsonl"))
      .map((l) => { try { const d = JSON.parse(l); return { ts: d.ts || "", model: d.picked_model || "", tier: d.tier || "", ms: d.duration_ms || 0, ok: d.success }; } catch { return null; } })
      .filter(Boolean).slice(-10).reverse();
    res.writeHead(200, { "Content-Type": "application/json" });
    return res.end(JSON.stringify(rs));
  }
  // API：对话
  if (url.pathname === "/api/chat" && req.method === "POST") {
    let body = "";
    for await (const chunk of req) body += chunk;
    try {
      const { message } = JSON.parse(body || "{}");
      if (!message) { res.writeHead(400, { "Content-Type": "application/json" }); return res.end(JSON.stringify({ error: "缺 message" })); }
      const reply = await chat(String(message));
      res.writeHead(200, { "Content-Type": "application/json" });
      return res.end(JSON.stringify({ reply }));
    } catch (e) {
      res.writeHead(500, { "Content-Type": "application/json" });
      return res.end(JSON.stringify({ error: String(e?.message || e) }));
    }
  }
  // 前端
  if (url.pathname === "/" || url.pathname === "/index.html") {
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    return res.end(fs.readFileSync(path.join(__dirname, "xi-shell.html"), "utf8"));
  }
  res.writeHead(404); res.end("not found");
});

server.listen(PORT, () => {
  console.log(`[xi-shell] 曦的 AI 生命壳: http://127.0.0.1:${PORT} (XI_HOME=${XI_HOME})`);
});
