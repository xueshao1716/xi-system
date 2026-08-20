#!/usr/bin/env python3
import io

path = "/mnt/d/xi-system/src/main.rs"
with io.open(path, "r", encoding="utf-8") as f:
    src = f.read()

old = "tier_providers: Vec::new(),"

new = """tier_providers: vec![
                                            agent_loop::TieredProvider {
                                                tier: model_router::ModelTier::Cheap,
                                                provider: agent_loop::LlmProvider {
                                                    model: "sensenova-6.7-flash-lite".to_string(),
                                                    llm_base: "https://token.sensenova.cn/v1".to_string(),
                                                    api_key: "sk-fvIaiJv2vALMK26tQq1FCbqKJ9LXf6qg".to_string(),
                                                    label: "sensenova-free".to_string(),
                                                },
                                            },
                                            agent_loop::TieredProvider {
                                                tier: model_router::ModelTier::Standard,
                                                provider: agent_loop::LlmProvider {
                                                    model: model.to_string(),
                                                    llm_base: llm_base.to_string(),
                                                    api_key: api_key.to_string(),
                                                    label: "deepseek-main".to_string(),
                                                },
                                            },
                                            agent_loop::TieredProvider {
                                                tier: model_router::ModelTier::Smart,
                                                provider: agent_loop::LlmProvider {
                                                    model: "MiniMax-M2".to_string(),
                                                    llm_base: "https://api.minimax.chat/v1".to_string(),
                                                    api_key: "sk-cp-5P7Tew1HaQcoP6mvyoVF7f1s27a5UllC_fNdasxs0pX-O87tF3ymwiC05bGqdSxsNvvLNqxTPTdXXJUYh9zugLi_ENrWegXQPYbAl4rYygpCK-9haR8mb3Y".to_string(),
                                                    label: "minimax-smart".to_string(),
                                                },
                                            },
                                        ],"""

count = src.count(old)
if count == 0:
    print("NOT FOUND")
    raise SystemExit(1)

src = src.replace(old, new)

with io.open(path, "w", encoding="utf-8") as f:
    f.write(src)

print(f"Replaced {count} occurrence(s)")
