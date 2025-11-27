use anyhow::{anyhow, Context, Result};
use gray_matter::{engine::YAML, Matter};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Open,
    Blocked,
    Closed,
}

impl Default for Status {
    fn default() -> Self {
        Status::Open
    }
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Frontmatter {
    id: String,
    title: String,
    estimate: Option<String>,
    #[serde(default)]
    status: Status,
    #[serde(default)]
    blocked_by: Vec<String>,
    #[serde(default)]
    blocks: Vec<String>,
}

#[derive(Debug, Clone)]
struct Task {
    front: Frontmatter,
    body: String,
    path: PathBuf,
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
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);
    let mut front: Frontmatter =
        parsed.data.as_ref().map(|d| d.deserialize()).transpose()?.unwrap_or_default();
    if front.id.is_empty() {
        front.id = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
    }
    if front.title.is_empty() {
        front.title = front.id.clone();
    }
    Ok(Task { front, body: parsed.content, path: path.to_path_buf() })
}

fn write_task(task: &Task) -> Result<()> {
    let matter = Matter::<YAML>::new();
    let rendered = matter.stringify(&task.front, &task.body);
    fs::write(&task.path, rendered)?;
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
                    map.insert(task.front.id.clone(), task);
                }
            }
        }
    }
    Ok(map)
}

fn update_checklist(task: &Task) -> Result<()> {
    let phase = task.front.id.split('.').next().unwrap();
    let readme_path = Path::new("docs/ROADMAP").join(phase).join("README.md");
    let data = fs::read_to_string(&readme_path)?;
    let re = Regex::new(r"^- \[[ x/]\] \[(P\d+\.\d+)\]\(([^)]+)\) (.*)$").unwrap();
    let mut out = String::new();
    for line in data.lines() {
        if let Some(caps) = re.captures(line) {
            let id = &caps[1];
            let link = &caps[2];
            let title = &caps[3];
            if id == task.front.id {
                out.push_str(&format!(
                    "- [{}] [{}]({}) {}\n",
                    task.front.status.mark(),
                    id,
                    link,
                    title
                ));
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
    // rebuild blocks from blocked_by
    for t in tasks.values_mut() {
        t.front.blocks.clear();
    }
    let snapshot = tasks.clone();
    for t in tasks.values_mut() {
        for dep in &t.front.blocked_by {
            if let Some(dep_task) = tasks.get_mut(dep) {
                if !dep_task.front.blocks.contains(&t.front.id) {
                    dep_task.front.blocks.push(t.front.id.clone());
                }
            }
        }
        // recompute status if not closed
        if t.front.status != Status::Closed {
            let mut blocked = false;
            for dep in &t.front.blocked_by {
                if let Some(dep_task) = snapshot.get(dep) {
                    if dep_task.front.status != Status::Closed {
                        blocked = true;
                        break;
                    }
                }
            }
            t.front.status = if blocked { Status::Blocked } else { Status::Open };
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
    task.front.status = status;
    recompute_blocked(tasks);
    save_all(tasks)
}

fn cmd_block(tasks: &mut HashMap<String, Task>, from: &str, to: &str) -> Result<()> {
    let from_task = tasks.get_mut(from).ok_or_else(|| anyhow!("from task not found"))?;
    let to_task = tasks.get_mut(to).ok_or_else(|| anyhow!("to task not found"))?;
    if !to_task.front.blocked_by.contains(&from.to_string()) {
        to_task.front.blocked_by.push(from.to_string());
    }
    if !from_task.front.blocks.contains(&to.to_string()) {
        from_task.front.blocks.push(to.to_string());
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
