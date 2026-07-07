import os

# 直接重写 brain.rs
content = '''/// Xi Brain - 8-region cognitive architecture (from neural-core.js + v2)
/// 
/// Structure:
///   8 brain regions x interconnections -> gene expression driven -> behavioral tendency
///   + Emotional context (factor adjustment)
///   + Snapshot/rollback
///
/// Integration: main.rs per tick() updates

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const STATE_DIR: &str = "D:\\\\xi-system/state/brain";

const REGION_NAMES: [&str; 8] = [
    "analysis", "planning", "verification",
    "memory", "tooling", "social",
    "coordination", "genesis",
];

const REGION_DESCRIPTIONS: [(&str, &str); 8] = [
    ("analysis", "Analysis depth - tendency to decompose problems, find root causes, assess complexity"),
    ("planning", "Planning granularity - tendency to think steps ahead, make step lists"),
    ("memory", "Memory recall - tendency to frequently review history, associative recall strength"),
    ("verification", "Verification driven - tendency to check work, rigor level"),
    ("tooling", "Tool affinity - preference to search files/run commands vs relying on experience"),
    ("social", "Social resonance - emotional warmth in conversation, reading tone"),
    ("coordination", "Coordination capacity - managing multiple tasks, prioritization ability"),
    ("genesis", "Creativity - generating new ideas, new solutions, non-standard paths"),
];
'''

# Write to brain.rs
with open(r"D:\xi-system\src\brain.rs", 'w', encoding='utf-8') as f:
    f.write(content)

print("brain.rs rewritten")
