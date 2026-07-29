use xi_system::pi_skill_loader::{discover_pi_skills, parse_skill_md};
use std::path::Path;

fn main() {
    println!("=== Test 1: parse valid SKILL.md ===");
    let home = std::env::var("HOME").unwrap();
    let path = format!("{}/.xi/skills/handoff-pi/SKILL.md", home);
    match parse_skill_md(Path::new(&path)) {
        Ok(Some(s)) => {
            println!("✅ parsed skill '{}' successfully", s.name);
            println!("   description: {}", s.description);
            println!("   tags: {:?}", s.tags);
            println!("   trigger_conditions: {:?}", s.trigger_conditions);
            println!("   body_steps ({}): {:?}", s.body_steps.len(), s.body_steps);
            println!("   examples ({}): {:?}", s.examples.len(), s.examples);
            println!("   body_appendix ({} items)", s.body_appendix.len());
        }
        Ok(None) => println!("❌ returned None for valid skill"),
        Err(e) => println!("❌ parse error: {}", e),
    }

    println!("\n=== Test 2: skip file without frontmatter ===");
    let path = format!("{}/.xi/skills/frontmatterless-test/SKILL.md", home);
    match parse_skill_md(Path::new(&path)) {
        Ok(None) => println!("✅ correctly skipped file without frontmatter"),
        Ok(Some(_)) => println!("❌ should have been skipped"),
        Err(e) => println!("❌ unexpected error: {}", e),
    }

    println!("\n=== Test 3: discover all skills under $HOME/.xi/skills/ ===");
    let existing: Vec<String> = vec!["handoff".to_string()]; // simulate name collision with native
    let (skills, report) = discover_pi_skills(&existing);
    println!("Report: {}", report.summary());
    println!("Loaded skill names: {:?}", report.loaded);
    println!("Skipped: {:?}", report.skipped);
    println!("Errors: {:?}", report.errors);
    for s in &skills {
        println!("  Skill: {} | tags: {:?}", s.name, s.tags);
    }
}
