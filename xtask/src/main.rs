use anyhow::{anyhow, Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Open,
    Blocked,
    Closed,
}

impl Status {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Status::Open),
            "blocked" => Some(Status::Blocked),
            "closed" => Some(Status::Closed),
            _ => None,
        }
    }
    fn to_str(self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::Blocked => "blocked",
            Status::Closed => "closed",
        }
    }
    fn mark(self) -> char {
        match self {
            Status::Open => ' ',
            Status::Blocked => '/',
            Status::Closed => 'x',
        }
    }
}

#[derive(Debug, Clone)]
struct Task {
    id: String,
    title: String,
    estimate: String,
    status: Status,
    blocked_by: Vec<String>,
    blocks: Vec<String>,
    path: PathBuf,
    body: String,
}

fn find_task_file(task_id: &str) -> Result<PathBuf> {
    let pattern = format!("{task_id}.md");
    for entry in WalkDir::new("docs/ROADMAP") {
        let entry = entry?;
        if entry.file_type().is_file() && entry.file_name() == pattern {
            return Ok(entry.into_path());
        }
    }
    Err(anyhow!("task file not found for {task_id}"))
}

fn parse_task(path: &Path) -> Result<Task> {
    let content = fs::read_to_string(path)?;
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return Err(anyhow!("missing frontmatter"));
    }
    let mut id = String::new();
    let mut title = String::new();
    let mut estimate = String::from("");
    let mut status = Status::Open;
    let mut blocked_by: Vec<String> = Vec::new();
    let mut blocks: Vec<String> = Vec::new();
    let mut in_frontmatter = true;
    while let Some(line) = lines.next() {
        if in_frontmatter && line.trim() == "---" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "id" => id = val.to_string(),
                "title" => title = val.to_string(),
                "estimate" => estimate = val.to_string(),
                "status" => status = Status::from_str(val).unwrap_or(Status::Open),
                "blocked_by" => {
                    // next lines processed below when seeing list items
                }
                "blocks" => {}
                _ => {}
            }
        } else if line.trim_start().starts_with("- ") {
            let item = line.trim_start().trim_start_matches('-').trim().to_string();
            // We don't know if it's blocked_by or blocks; infer from previous key not tracked; fallback to blocked_by
            // Simpler: collect all list items after blocked_by: or blocks:
        }
    }
    // Simpler re-parse lists with regex
    let re_blocked = Regex::new(r"blocked_by:\n(?P<items>(?:\s*-\s*id:\s*.*\n?)+)").unwrap();
    if let Some(cap) = re_blocked.captures(&content) {
        let items = cap.name("items").unwrap().as_str();
        let item_re = Regex::new(r"id:\s*([^\n\r]+)").unwrap();
        blocked_by = item_re.captures_iter(items).map(|c| c[1].trim().to_string()).collect();
    }
    let re_blocks = Regex::new(r"blocks:\n(?P<items>(?:\s*-\s*id:\s*.*\n?)+)").unwrap();
    if let Some(cap) = re_blocks.captures(&content) {
        let items = cap.name("items").unwrap().as_str();
        let item_re = Regex::new(r"id:\s*([^\n\r]+)").unwrap();
        blocks = item_re.captures_iter(items).map(|c| c[1].trim().to_string()).collect();
    }

    // Remaining body after frontmatter
    let body_start = content.find("---\n").ok_or_else(|| anyhow!("bad fm"))?;
    let body = content[body_start + 4..].splitn(2, "---\n").nth(1).unwrap_or("").to_string();

    if id.is_empty() {
        id = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
    }
    if title.is_empty() {
        title = id.clone();
    }

    Ok(Task { id, title, estimate, status, blocked_by, blocks, path: path.to_path_buf(), body })
}

fn write_task(task: &Task) -> Result<()> {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("id: {}\n", task.id));
    out.push_str(&format!("title: {}\n", task.title));
    if !task.estimate.is_empty() {
        out.push_str(&format!("estimate: {}\n", task.estimate));
    }
    out.push_str(&format!("status: {}\n", task.status.to_str()));
    out.push_str("blocked_by:\n");
    if task.blocked_by.is_empty() {
        out.push_str("  -\n");
    } else {
        for id in &task.blocked_by {
            out.push_str(&format!("  - id: {}\n", id));
        }
    }
    out.push_str("blocks:\n");
    if task.blocks.is_empty() {
        out.push_str("  -\n");
    } else {
        for id in &task.blocks {
            out.push_str(&format!("  - id: {}\n", id));
        }
    }
    out.push_str("---\n");
    out.push_str(&task.body);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    fs::write(&task.path, out)?;
    Ok(())
}

fn load_tasks() -> Result<HashMap<String, Task>> {
    let mut map = HashMap::new();
    for entry in WalkDir::new("docs/ROADMAP") {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with('P') && name.ends_with(".md") && name != "README.md" {
                    let task = parse_task(entry.path())?;
                    map.insert(task.id.clone(), task);
                }
            }
        }
    }
    Ok(map)
}

fn update_checklist(task: &Task) -> Result<()> {
    let phase = task.id.split('.').next().unwrap();
    let readme_path = Path::new("docs/ROADMAP").join(phase).join("README.md");
    let data = fs::read_to_string(&readme_path)?;
    let re = Regex::new(r"^- \[[ x/]\] \[(P\d+\.\d+)\]\(([^)]+)\) (.*)$").unwrap();
    let mut out = String::new();
    for line in data.lines() {
        if let Some(caps) = re.captures(line) {
            let id = &caps[1];
            let link = &caps[2];
            let title = &caps[3];
            if id == task.id {
                out.push_str(&format!("- [{}] [{}]({}) {}\n", task.status.mark(), id, link, title));
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    fs::write(readme_path, out)?;
    Ok(())
}

fn recompute_blocked(tasks: &mut HashMap<String, Task>) {
    // ensure blocks lists are consistent with blocked_by
    // rebuild blocks from blocked_by for safety
    for t in tasks.values_mut() {
        t.blocks.clear();
    }
    let ids: Vec<String> = tasks.keys().cloned().collect();
    for id in &ids {
        let deps = tasks[id].blocked_by.clone();
        for dep in deps {
            if let Some(dep_task) = tasks.get_mut(&dep) {
                if !dep_task.blocks.contains(id) {
                    dep_task.blocks.push(id.to_string());
                }
            }
        }
    }
    // propagate: if any blocked_by not closed -> blocked, else open (unless closed)
    let snapshot = tasks.clone();
    for t in tasks.values_mut() {
        if t.status == Status::Closed {
            continue;
        }
        let mut blocked = false;
        for dep in &t.blocked_by {
            if let Some(dep_task) = snapshot.get(dep) {
                if dep_task.status != Status::Closed {
                    blocked = true;
                    break;
                }
            }
        }
        if blocked {
            t.status = Status::Blocked;
        } else if t.status == Status::Blocked {
            t.status = Status::Open;
        }
    }
}

fn save_all(tasks: &HashMap<String, Task>) -> Result<()> {
    for task in tasks.values() {
        write_task(task)?;
        update_checklist(task)?;
    }
    Ok(())
}

fn cmd_set(tasks: &mut HashMap<String, Task>, id: &str, status: Status) -> Result<()> {
    let task = tasks.get_mut(id).ok_or_else(|| anyhow!("task not found"))?;
    task.status = status;
    recompute_blocked(tasks);
    save_all(tasks)
}

fn cmd_block(tasks: &mut HashMap<String, Task>, from: &str, to: &str) -> Result<()> {
    let from_task = tasks.get_mut(from).ok_or_else(|| anyhow!("from task not found"))?;
    let to_task = tasks.get_mut(to).ok_or_else(|| anyhow!("to task not found"))?;
    if !to_task.blocked_by.contains(&from.to_string()) {
        to_task.blocked_by.push(from.to_string());
    }
    if !from_task.blocks.contains(&to.to_string()) {
        from_task.blocks.push(to.to_string());
    }
    recompute_blocked(tasks);
    save_all(tasks)
}

fn usage() {
    eprintln!("Usage:\n  cargo run --bin tasks set <TASK_ID> <open|blocked|closed>\n  cargo run --bin tasks block <FROM_ID> <TO_ID>");
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        usage();
        std::process::exit(1);
    }
    let cmd = args.remove(0);
    let mut tasks = load_tasks()?;
    match cmd.as_str() {
        "set" => {
            if args.len() != 2 {
                usage();
                std::process::exit(1);
            }
            let id = &args[0];
            let status = Status::from_str(&args[1]).ok_or_else(|| anyhow!("invalid status"))?;
            cmd_set(&mut tasks, id, status)?;
            println!("Set {id} -> {}", status.to_str());
        }
        "block" => {
            if args.len() != 2 {
                usage();
                std::process::exit(1);
            }
            let from = &args[0];
            let to = &args[1];
            cmd_block(&mut tasks, from, to)?;
            println!("{from} now blocks {to}");
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
    Ok(())
}
