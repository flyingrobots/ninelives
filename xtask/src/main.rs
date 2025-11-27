use anyhow::{anyhow, Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy)]
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
    fn checklist_mark(self) -> char {
        match self {
            Status::Open => ' ',
            Status::Blocked => '/',
            Status::Closed => 'x',
        }
    }
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

fn parse_frontmatter(content: &str) -> Result<(HashMap<String, String>, usize)> {
    let mut lines = content.lines();
    let mut map = HashMap::new();
    if lines.next() != Some("---") {
        return Err(anyhow!("missing frontmatter start"));
    }
    let mut consumed = 1;
    for line in lines.by_ref() {
        consumed += 1;
        if line.trim() == "---" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    Ok((map, consumed))
}

fn update_frontmatter(path: &Path, status: Status) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let (mut fm, fm_lines) = parse_frontmatter(&content)?;
    fm.insert("status".into(), status.to_str().into());

    // rebuild frontmatter preserving dependencies block if present
    let mut out = String::new();
    out.push_str("---\n");
    for key in ["id", "title", "estimate", "status"] {
        if let Some(val) = fm.get(key) {
            out.push_str(&format!("{key}: {val}\n"));
        }
    }
    // keep dependencies lines verbatim from original
    let deps_re = Regex::new(r"^dependencies:\n([\s\S]*)").unwrap();
    if let Some(caps) = deps_re.captures(&content) {
        out.push_str(&format!("{}\n", caps.get(0).unwrap().as_str()));
    } else {
        out.push_str("dependencies:\n  -\n");
    }
    out.push_str("---\n");
    // append rest of content after original frontmatter
    let rest: Vec<&str> = content.lines().skip(fm_lines).collect();
    out.push_str(&rest.join("\n"));
    if !out.ends_with('\n') {
        out.push('\n');
    }
    fs::write(path, out)?;
    Ok(())
}

fn update_checklist(task_id: &str, status: Status) -> Result<()> {
    // determine phase dir
    let phase = task_id.split('.').next().ok_or_else(|| anyhow!("bad id"))?;
    let phase_dir = Path::new("docs/ROADMAP").join(phase);
    let readme = phase_dir.join("README.md");
    let data = fs::read_to_string(&readme)?;
    let mut out = String::new();
    let re = Regex::new(r"^- \[[ x/]\] \[(?P<id>P\d+\.\d+)\]\((?P<link>[^)]+)\) (?P<title>.*)$")
        .unwrap();
    for line in data.lines() {
        if let Some(caps) = re.captures(line) {
            let id = &caps["id"];
            if id == task_id {
                let link = &caps["link"];
                let title = &caps["title"];
                out.push_str(&format!(
                    "- [{}] [{}]({}) {}\n",
                    status.checklist_mark(),
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
    fs::write(readme, out)?;
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        eprintln!("Usage: cargo run --bin tasks <TASK_ID> <open|blocked|closed>");
        std::process::exit(1);
    }
    let task_id = args.remove(0);
    let status = Status::from_str(&args[0]).ok_or_else(|| anyhow!("invalid status"))?;

    let path = find_task_file(&task_id)?;
    update_frontmatter(&path, status).context("update frontmatter")?;
    update_checklist(&task_id, status).context("update checklist")?;
    println!("Updated {task_id} -> {}", status.to_str());
    Ok(())
}
