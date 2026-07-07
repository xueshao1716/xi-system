/**
 * SkillOS — 技能策展引擎 (STOM v2.1 融合)
 *
 * 架构：三层信息架构（源自 诗/STOM v2.1）
 *   L1 — 执行层：skill manifest（元信息 / 触发条件 / 步骤）
 *   L2 — 参考层：模板 / 脚本 / 引用
 *   L3 — 外部层：API 端点 / 工具注册
 *   gate — CI 门禁：三纯净校验（最小化 / 可复用 / 可测试）
 *
 * 运行时：SkillCurator 策展引擎（V2）
 *   评分向量（标签 35% / 场景 25% / 历史 25% / 新鲜度 15%）
 *   成本感知 + 依赖解析 + 回退链 + 自适应阈值
 *
 * 参考：Google Cloud AI Research — SkillOS 论文
 *       诗/STOM v2.1 方法论（三层信息架构 + CI 门禁 + 三纯净原则）
 * 迭代：V2 → V3: 融合 STOM 信息架构
 */

/** CI 门禁 — 三纯净校验 */
class StomGate {
  static validate(skillDef) {
    const errors = []
    if (!skillDef.name || skillDef.name.length < 2)
      errors.push('技能名太短或缺失')
    if (typeof skillDef.handler !== 'function')
      errors.push('必须有 handler 函数')
    if (!skillDef.meta?.tags?.length)
      errors.push('必须有 tags 标签（可测试性要求）')
    return { ok: errors.length === 0, errors }
  }
}

class SkillCurator {
  constructor(options = {}) {
    this.skills = new Map()
    this.history = []
    this.baseThreshold = options.threshold ?? 0.5
    this.maxHistory = options.maxHistory ?? 200
  }

  /**
   * 注册技能（通过门禁才注册）
   * @param {string} name
   * @param {Function} handler
   * @param {Object} meta
   * @param {string[]} meta.tags
   * @param {string[]} meta.scenarios
   * @param {number} meta.cost
   * @param {string[]} meta.dependsOn
   * @param {string[]} meta.fallback
   */
  register(name, handler, meta = {}) {
    const check = StomGate.validate({ name, handler, meta })
    if (!check.ok) {
      console.warn(`[STOM gate] 技能 ${name} 未通过门禁:`, check.errors.join(', '))
      return this
    }
    this.skills.set(name, {
      handler,
      meta: {
        tags: meta.tags ?? [],
        scenarios: meta.scenarios ?? [],
        cost: meta.cost ?? 0.5,
        dependsOn: meta.dependsOn ?? [],
        fallback: meta.fallback ?? [],
      },
      stats: { calls: 0, hits: 0, fails: 0, totalLatency: 0 },
    })
    return this
  }

  curate(task) {
    const threshold = this._adaptiveThreshold(task)
    const candidates = []

    for (const [name, skill] of this.skills) {
      const score = this._evaluate(name, skill, task)
      if (score.match >= threshold) {
        candidates.push({
          name,
          score: score.match,
          cost: skill.meta.cost,
          value: score.match / Math.max(skill.meta.cost, 0.01),
          handler: skill.handler,
          fallback: skill.meta.fallback,
        })
      }
    }

    candidates.sort((a, b) => b.value - a.value)
    this._record(task, candidates)
    return candidates
  }

  async execute(task) {
    const curated = this.curate(task)
    if (curated.length === 0) return { ok: false, reason: 'no_skill_matched' }

    const resolved = await this._resolveDependencies(curated[0].name, task)
    if (!resolved.ok) return resolved

    return this._runWithFallback(curated, task, 0)
  }

  _evaluate(name, skill, task) {
    const meta = skill.meta
    const taskType = task.type ?? ''
    const keywords = task.keywords ?? []
    const context = task.context ?? {}

    let tagScore = 0
    let scenarioScore = 0
    let historyScore = 0
    let contextScore = 0

    if (meta.tags.length > 0) {
      const matched = meta.tags.filter(t => taskType.includes(t) || keywords.some(k => t.includes(k)))
      tagScore = matched.length / meta.tags.length
    }

    if (meta.scenarios.length > 0 && context.scenario) {
      const matched = meta.scenarios.filter(s =>
        s.toLowerCase().includes(context.scenario.toLowerCase())
      )
      scenarioScore = matched.length / meta.scenarios.length
    }

    const st = skill.stats
    if (st.calls > 0) {
      const hitRate = st.hits / st.calls
      const avgLatency = st.totalLatency / st.calls
      historyScore = hitRate * 0.7 - Math.min(avgLatency / 10000, 0.3)
    }

    const recentCalls = this.history.filter(h => h.skill === name).length
    contextScore = Math.max(0, 1 - recentCalls * 0.05)

    const match = tagScore * 0.35 + scenarioScore * 0.25 + historyScore * 0.25 + contextScore * 0.15
    const confidence = Math.min(1, (st.calls + 1) / 5)

    return { match: Math.round(match * 100) / 100, confidence }
  }

  _adaptiveThreshold(task) {
    const complexity = (task.keywords?.length ?? 0) * 0.05 +
                       (task.context?.scenario ? 0.1 : 0) +
                       (task.type?.length ?? 0) * 0.01
    return Math.max(0.2, this.baseThreshold - complexity)
  }

  async _resolveDependencies(skillName, task) {
    const visited = new Set()
    const queue = [skillName]

    while (queue.length > 0) {
      const current = queue.shift()
      if (visited.has(current)) continue
      visited.add(current)

      const skill = this.skills.get(current)
      if (!skill) return { ok: false, reason: `dependency_not_found:${current}` }

      for (const dep of skill.meta.dependsOn) {
        if (!this.skills.has(dep)) {
          return { ok: false, reason: `dependency_not_registered:${dep}` }
        }
        queue.push(dep)
      }
    }

    return { ok: true }
  }

  async _runWithFallback(candidates, task, index) {
    if (index >= candidates.length) {
      return { ok: false, reason: 'all_fallback_exhausted' }
    }

    const candidate = candidates[index]
    const skill = this.skills.get(candidate.name)
    skill.stats.calls++
    const start = Date.now()

    try {
      const result = await candidate.handler(task)
      skill.stats.hits++
      skill.stats.totalLatency += Date.now() - start
      return { ok: true, skill: candidate.name, result }
    } catch (err) {
      skill.stats.fails++
      skill.stats.totalLatency += Date.now() - start

      for (const fallbackName of candidate.fallback) {
        const fbIndex = candidates.findIndex(c => c.name === fallbackName)
        if (fbIndex > index) {
          return this._runWithFallback(candidates, task, fbIndex)
        }
      }

      return this._runWithFallback(candidates, task, index + 1)
    }
  }

  _record(task, candidates) {
    this.history.push({
      task: task.type,
      skill: candidates[0]?.name ?? 'none',
      count: candidates.length,
      time: Date.now(),
    })
    if (this.history.length > this.maxHistory) this.history.shift()
  }

  stats() {
    const result = {}
    for (const [name, skill] of this.skills) {
      const st = skill.stats
      result[name] = {
        calls: st.calls,
        hits: st.hits,
        fails: st.fails,
        hitRate: st.calls > 0 ? (st.hits / st.calls).toFixed(2) : 0,
        avgLatency: st.calls > 0 ? Math.round(st.totalLatency / st.calls) : 0,
        cost: skill.meta.cost,
        dependsOn: skill.meta.dependsOn,
      }
    }
    return result
  }

  recentHistory(n = 10) {
    return this.history.slice(-n)
  }
}

module.exports = { SkillCurator, StomGate }
