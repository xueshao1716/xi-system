#!/usr/bin/env python3
import re

with open('src/ctx2soft.rs.bak', 'r', errors='replace') as f:
    content = f.read()
lines = content.split('\n')
line6 = lines[5]

breaks = set()

# 1. Before standalone // and /// (preceded by ;, }, ], ))
for m in re.finditer(r'//', line6):
    pos = m.start()
    before = line6[max(0,pos-30):pos].rstrip()
    if before.endswith((';', '}', ']', ')')) or pos == 0:
        breaks.add(pos)

# 2. Before pub fn, pub struct, pub enum, impl, use, const, static, #[
#    when preceded by ;, }, ], ), or at start
for kw in ['pub fn ', 'pub async fn ', 'pub struct ', 'pub enum ', 'pub type ',
           'pub trait ', 'impl ', 'use ', 'const ', 'static ', 'mod ', '#[', '///',
           'fn ', 'enum ', 'struct ', 'type ', 'trait ', 'unsafe ', 'extern ']:
    for m in re.finditer(re.escape(kw), line6):
        pos = m.start()
        before = line6[max(0,pos-30):pos].rstrip()
        if before.endswith((';', '}', ']', ')', ',')) or pos == 0:
            breaks.add(pos)

# 3. Before } when preceded by ; or code
for m in re.finditer(r'}', line6):
    pos = m.start()
    before = line6[max(0,pos-30):pos].rstrip()
    if before.endswith((';', ',', '>', ']', ')', '"', 'true', 'false')) or pos == 0:
        breaks.add(pos)

# 4. Before common statement starters with 4+ spaces before them
for kw in ['let ', 'if ', 'for ', 'while ', 'match ', 'return ', 'break;',
           'continue;', 'self.', 'println!', 'eprintln!', 'format!',
           'scores.push', 'failures.push', 'Self {']:
    for m in re.finditer(re.escape(kw), line6):
        pos = m.start()
        if pos > 0:
            sp = 0
            p = pos - 1
            while p >= 0 and line6[p] == ' ':
                sp += 1
                p -= 1
            if sp >= 4:
                breaks.add(pos)

print("Total break positions:", len(breaks))
for b in sorted(breaks)[:30]:
    print("  pos %d: %r" % (b, line6[b:b+30]))
