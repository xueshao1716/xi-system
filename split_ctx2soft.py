#!/usr/bin/env python3
"""
Split merged Rust source file back into proper lines.
Handles strings, line comments, block comments correctly.
"""
import re
import sys

def find_non_code_ranges(content):
    """Find all ranges inside strings, line comments, or block comments."""
    ranges = []
    n = len(content)
    i = 0
    
    while i < n:
        ch = content[i]
        next_ch = content[i+1] if i+1 < n else ''
        
        # Line comment // ...
        if ch == '/' and next_ch == '/':
            start = i
            i += 2
            # In merged content, line comments run until we find a pattern
            # that indicates the start of code. We look for: use/pub/fn/impl/const/static/#[/{/}
            # preceded by spaces (indentation)
            while i < n:
                # Look ahead for code patterns
                rest = content[i:]
                # Check for known code patterns that indicate end of comment
                if re.match(r'(use |pub |impl |const |static |fn |enum |struct |mod |type |trait |unsafe |extern |\{|\}|\#[\[])', rest):
                    # But verify it's not part of comment text by checking indentation
                    # Count spaces before this position
                    sp = 0
                    p = i - 1
                    while p >= start and content[p] == ' ':
                        sp += 1
                        p -= 1
                    # If preceded by 0 spaces from start of comment, it's comment text
                    # If preceded by spaces, it could be indentation of next line
                    if sp > 0:
                        # This looks like code after the comment
                        break
                    else:
                        i += 1
                elif i > start + 2 and content[i] == '/' and i+1 < n and content[i+1] == '/':
                    # Another // comment
                    break
                else:
                    i += 1
            ranges.append((start, i, 'line_comment'))
            continue
        
        # Block comment /* ... */
        if ch == '/' and next_ch == '*':
            start = i
            i += 2
            depth = 1
            while i < n and depth > 0:
                if content[i] == '/' and i + 1 < n and content[i+1] == '*':
                    depth += 1
                    i += 2
                elif content[i] == '*' and i + 1 < n and content[i+1] == '/':
                    depth -= 1
                    i += 2
                else:
                    i += 1
            ranges.append((start, i, 'block_comment'))
            continue
        
        # String literal
        if ch == '"' and (i == 0 or content[i-1] != '\\'):
            start = i
            i += 1
            while i < n:
                if content[i] == '\\' and i + 1 < n:
                    i += 2
                    continue
                if content[i] == '"':
                    i += 1
                    break
                i += 1
            ranges.append((start, i, 'string'))
            continue
        
        # Char literal
        if ch == "'":
            if i + 2 < n and content[i+2] == "'":
                ranges.append((i, i+3, 'char'))
                i += 3
                continue
            # Could be lifetime - skip
        
        i += 1
    
    return ranges

def in_non_code(pos, ranges):
    for s, e, _ in ranges:
        if s <= pos < e:
            return True
    return False

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
        raw = f.read()
    
    lines_raw = raw.split('\n')
    print("Original file has %d lines" % len(lines_raw), file=sys.stderr)
    
    header_lines = []
    merged_content = ''
    
    for idx, line in enumerate(lines_raw):
        if idx < 5:
            header_lines.append(line)
        elif idx == 5:
            merged_content = line
    
    print("Merged content length: %d chars" % len(merged_content), file=sys.stderr)
    
    # Find non-code ranges
    non_code_ranges = find_non_code_ranges(merged_content)
    print("Found %d non-code ranges" % len(non_code_ranges), file=sys.stderr)
    
    # Collect all break positions
    breaks = set()
    n = len(merged_content)
    
    # Patterns to break before (must not be inside strings/comments)
    break_before = [
        'pub fn ', 'pub async fn ', 'pub struct ', 'pub enum ',
        'pub type ', 'pub trait ', 'impl ', 'use ', 'const ',
        'static ', 'mod ', 'type ', 'trait ', 'unsafe ',
        'extern ', 'fn ', 'enum ', 'struct ',
    ]
    
    for pat in break_before:
        for m in re.finditer(re.escape(pat), merged_content):
            pos = m.start()
            if not in_non_code(pos, non_code_ranges):
                breaks.add(pos)
    
    # Break before #[ (attributes)
    for m in re.finditer(r'#\[', merged_content):
        pos = m.start()
        if not in_non_code(pos, non_code_ranges):
            breaks.add(pos)
    
    # Break before // and /// (comments) - these are already handled by non_code_ranges
    # but we want to ensure they're on separate lines
    for m in re.finditer(r'//', merged_content):
        pos = m.start()
        if not in_non_code(pos, non_code_ranges):
            # This // is NOT inside another comment or string
            # It's the start of a comment - add break before it
            breaks.add(pos)
    
    # Break before } when preceded by certain tokens
    for m in re.finditer(r'}', merged_content):
        pos = m.start()
        if not in_non_code(pos, non_code_ranges) and pos > 0:
            before = merged_content[max(0, pos-20):pos].rstrip()
            if before.endswith((';', ',', '"', 'true', 'false', ')', ']', '>')):
                breaks.add(pos)
    
    # Break before statement starters with 4+ spaces indent
    for kw in ['let ', 'if ', 'for ', 'while ', 'match ', 'return ',
               'break;', 'continue;', 'self.', 'println!', 'eprintln!',
               'format!', 'scores.push', 'failures.push', 'Self {',
               '} else', '} else if']:
        for m in re.finditer(re.escape(kw), merged_content):
            pos = m.start()
            if not in_non_code(pos, non_code_ranges) and pos > 0:
                sp = 0
                p = pos - 1
                while p >= 0 and merged_content[p] == ' ':
                    sp += 1
                    p -= 1
                if sp >= 4:
                    breaks.add(pos)
    
    print("Total break positions: %d" % len(breaks), file=sys.stderr)
    
    # Sort breaks
    sorted_breaks = sorted(breaks)
    
    # Build result
    result = []
    prev = 0
    for pos in sorted_breaks:
        segment = merged_content[prev:pos]
        if segment:
            result.append(segment.rstrip())
        prev = pos
    # Add remaining
    remaining = merged_content[prev:]
    if remaining.strip():
        result.append(remaining.strip())
    
    print("After splitting: %d lines" % len(result), file=sys.stderr)
    
    # Combine
    all_lines = header_lines + result
    
    # Write
    output = '\n'.join(all_lines) + '\n'
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(output)
    
    print("Final output: %d lines" % len(all_lines), file=sys.stderr)

if __name__ == '__main__':
    process_file('/mnt/d/xi-system/src/ctx2soft.rs')
