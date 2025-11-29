use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use walkdir::WalkDir;

mod plans;

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
    #[serde(default)]
    value: Option<String>, // e.g., H/M/L or numeric
}

#[derive(Debug, Clone)]
struct Task {
    front: Frontmatter,
    body: String,
    path: PathBuf, // phase README path
}

fn strip_tasks_section(readme_text: &str) -> String {
    let mut out = Vec::new();
    let mut in_tasks = false;
    for line in readme_text.lines() {
        if line.trim().eq_ignore_ascii_case("## Tasks") {
            in_tasks = true;
            continue;
        }
        if in_tasks && line.starts_with("## ") {
            in_tasks = false;
        }
        if !in_tasks {
            out.push(line);
        }
    }
    while out.last().map(|l| l.is_empty()).unwrap_or(false) {
        out.pop();
    }
    out.push("");
    out.join("\n")
}

fn parse_tasks_from_readme(path: &Path) -> Result<Vec<Task>> {
    let text = fs::read_to_string(path)?;
    let lines = text.lines().collect::<Vec<_>>();
    let mut idx = None;
    for (i, l) in lines.iter().enumerate() {
        if l.trim().eq_ignore_ascii_case("## Tasks") {
            idx = Some(i + 1);
            break;
        }
    }
    let mut tasks = Vec::new();
    let mut i = idx.unwrap_or(lines.len());
    while i < lines.len() {
        if !lines[i].starts_with("### ") {
            i += 1;
            continue;
        }
        let heading = lines[i].trim_start_matches("### ").trim();
        let mut parts = heading.splitn(2, ' ');
        let id = parts.next().unwrap_or("").to_string();
        let title = parts.next().unwrap_or(heading).to_string();
        i += 1;
        // parse table
        let mut meta = Frontmatter { id: id.clone(), title: title.clone(), ..Default::default() };
        let mut table_started = false;
        while i < lines.len() {
            let l = lines[i].trim();
            if l.starts_with("### ") {
                break;
            }
            if l.starts_with("## ") {
                break;
            }
            if l.starts_with("| id |") {
                table_started = true;
            }
            if table_started && l.starts_with('|') && l.contains('|') {
                let cols: Vec<_> = l.trim_matches('|').split('|').map(|s| s.trim()).collect();
                if cols.len() >= 2 {
                    let key = cols[0];
                    let val = cols[1];
                    match key {
                        "id" => meta.id = val.to_string(),
                        "title" => meta.title = val.to_string(),
                        "estimate" => meta.estimate = Some(val.to_string()),
                        "status" => {
                            meta.status =
                                Status::from_str(&val.to_lowercase()).unwrap_or(Status::Open)
                        }
                        "blocked_by" => {
                            meta.blocked_by = if val == "-" || val.is_empty() {
                                vec![]
                            } else {
                                val.split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect()
                            }
                        }
                        "blocks" => {
                            meta.blocks = if val == "-" || val.is_empty() {
                                vec![]
                            } else {
                                val.split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect()
                            }
                        }
                        "value" => {
                            meta.value = if val == "-" || val.is_empty() {
                                None
                            } else {
                                Some(val.to_string())
                            }
                        }
                        _ => {}
                    }
                }
            }
            i += 1;
            if table_started && l.is_empty() {
                break;
            }
        }
        // body until next ### or ##
        let mut body_lines = Vec::new();
        while i < lines.len() {
            let l = lines[i];
            if l.starts_with("### ") || l.starts_with("## ") {
                break;
            }
            body_lines.push(l);
            i += 1;
        }
        let body = body_lines.join("\n").trim().to_string();
        tasks.push(Task { front: meta, body, path: path.to_path_buf() });
    }
    Ok(tasks)
}

fn render_tasks(tasks: &[Task]) -> String {
    let mut out = String::from("## Tasks\n\n");
    if tasks.is_empty() {
        out.push_str("No open tasks.\n");
        return out;
    }
    for t in tasks {
        out.push_str(&format!("### {} {}\n\n", t.front.id, t.front.title));
        out.push_str("| field | value |\n| --- | --- |\n");
        let fmt_list = |v: &Vec<String>| if v.is_empty() { "-".to_string() } else { v.join(", ") };
        out.push_str(&format!("| id | {} |\n", t.front.id));
        out.push_str(&format!("| title | {} |\n", t.front.title));
        out.push_str(&format!(
            "| estimate | {} |\n",
            t.front.estimate.clone().unwrap_or_else(|| "-".to_string())
        ));
        out.push_str(&format!("| status | {} |\n", t.front.status.to_str()));
        out.push_str(&format!("| blocked_by | {} |\n", fmt_list(&t.front.blocked_by)));
        out.push_str(&format!("| blocks | {} |\n", fmt_list(&t.front.blocks)));
        out.push_str(&format!(
            "| value | {} |\n\n",
            t.front.value.clone().unwrap_or_else(|| "-".to_string())
        ));
        if !t.body.is_empty() {
            out.push_str(&t.body);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

fn write_phase(readme: &Path, tasks: &[Task]) -> Result<()> {
    let base = strip_tasks_section(&fs::read_to_string(readme)?);
    let rendered =
        format!("{}\n\n{}", base.trim_end(), render_tasks(tasks)).trim_end().to_string() + "\n";
    fs::write(readme, rendered)?;
    Ok(())
}

fn load_tasks() -> Result<HashMap<String, Task>> {
    let mut map = HashMap::new();
    for entry in WalkDir::new("docs/ROADMAP") {
        let entry = entry?;
        if entry.file_type().is_file() && entry.file_name() == "README.md" {
            let tasks = parse_tasks_from_readme(entry.path())?;
            for t in tasks {
                map.insert(t.front.id.clone(), t);
            }
        }
    }
    Ok(map)
}

fn recompute_blocked(tasks: &mut HashMap<String, Task>) {
    // rebuild blocks from blocked_by
    for t in tasks.values_mut() {
        t.front.blocks.clear();
    }
    // build snapshot of statuses for dependency checks
    let snapshot: HashMap<String, Status> =
        tasks.iter().map(|(id, t)| (id.clone(), t.front.status)).collect();

    // rebuild blocks lists
    let ids: Vec<String> = tasks.keys().cloned().collect();
    for id in &ids {
        let deps = tasks[id].front.blocked_by.clone();
        for dep in deps {
            if let Some(dep_task) = tasks.get_mut(&dep) {
                if !dep_task.front.blocks.contains(id) {
                    dep_task.front.blocks.push(id.clone());
                }
            }
        }
    }

    // enforce deterministic ordering for stable diffs
    for t in tasks.values_mut() {
        t.front.blocked_by.sort();
        t.front.blocks.sort();
    }

    // recompute blocked/open based on snapshot
    for t in tasks.values_mut() {
        if t.front.status == Status::Closed {
            continue;
        }
        let mut blocked = false;
        for dep in &t.front.blocked_by {
            if let Some(dep_status) = snapshot.get(dep) {
                if *dep_status != Status::Closed {
                    blocked = true;
                    break;
                }
            }
        }
        t.front.status = if blocked { Status::Blocked } else { Status::Open };
    }
}

fn generate_mermaid(tasks: &HashMap<String, Task>, edges: &[(String, String)]) -> String {
    let mut out = String::new();
    out.push_str("graph TD\n");
    out.push_str("    classDef closed stroke:#28a745,stroke-width:3px;\n");
    out.push_str("    classDef blocked stroke:#dc3545,stroke-width:3px;\n");
    out.push_str("    classDef open stroke:#ffc107,stroke-width:3px;\n");
    for phase in &["P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9", "P10"] {
        out.push_str(&format!("    classDef {} fill:#f8f9fa;\n", phase));
    }

    let mut ids: Vec<_> = tasks.keys().cloned().collect();
    ids.sort();
    for id in ids {
        let t = tasks.get(&id).unwrap();
        let phase = id.split('.').next().unwrap_or("");
        let status_class = match t.front.status {
            Status::Open => "open",
            Status::Blocked => "blocked",
            Status::Closed => "closed",
        };
        let title = t.front.title.replace('"', "\"");
        out.push_str(&format!("    {}[\"{}\"]\n", id, title));
        out.push_str(&format!("    class {} {},{}\n", id, status_class, phase));
    }

    for (from, to) in edges {
        out.push_str(&format!("    {} --> {}\n", from, to));
    }

    out
}

fn save_all(tasks: &HashMap<String, Task>) -> Result<()> {
    // group by phase README path
    let mut phases: HashMap<PathBuf, Vec<Task>> = HashMap::new();
    for t in tasks.values() {
        phases.entry(t.path.clone()).or_default().push(t.clone());
    }
    for tasks in phases.values_mut() {
        tasks.sort_by(|a, b| a.front.id.cmp(&b.front.id));
    }
    for (readme, ts) in phases {
        write_phase(&readme, &ts)?;
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
    {
        let to_task = tasks.get_mut(to).ok_or_else(|| anyhow!("to task not found"))?;
        if !to_task.front.blocked_by.contains(&from.to_string()) {
            to_task.front.blocked_by.push(from.to_string());
        }
    }
    {
        let from_task = tasks.get_mut(from).ok_or_else(|| anyhow!("from task not found"))?;
        if !from_task.front.blocks.contains(&to.to_string()) {
            from_task.front.blocks.push(to.to_string());
        }
    }
    recompute_blocked(tasks);
    save_all(tasks)
}

fn cmd_enrich(tasks: &mut HashMap<String, Task>, phase: &str) -> Result<()> {
    if phase != "P2" {
        return Err(anyhow!("enrich currently supports only P2"));
    }
    let plans = plans::p2_plans();
    for (id, plan) in plans {
        if let Some(task) = tasks.get_mut(id) {
            // update value from plan
            if let Some(v) = plan.value {
                task.front.value = Some(v.to_string());
            }

            // rebuild body with plan content
            let steps = plan
                .steps
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {}", i + 1, s))
                .collect::<Vec<_>>()
                .join("\n");
            let ready =
                plan.ready.iter().map(|r| format!("- [ ] {}", r)).collect::<Vec<_>>().join("\n");
            let mut tests = String::from("## Test Plan\n");
            if !plan.unit.is_empty() {
                tests.push_str("\n### Unit Tests\n");
                for c in plan.unit {
                    tests.push_str(&format!("- [ ] {}\n", c));
                }
            }
            if !plan.integ.is_empty() {
                tests.push_str("\n### Integration Tests\n");
                for c in plan.integ {
                    tests.push_str(&format!("- [ ] {}\n", c));
                }
            }
            tests.push_str("\n### End-to-end Tests\n- N/A\n");
            task.body = format!(
                "# [{}] {}\n\n## Summary\n\n{}\n\n## Steps\n{}\n\n## Ready When\n{}\n\n{}",
                task.front.id, task.front.title, plan.summary, steps, ready, tests
            );
        }
    }
    save_all(tasks)
}

fn read_edges(path: &Path) -> Result<Vec<(String, String)>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = fs::read_to_string(path).context("read DAG.csv")?;
    let mut rdr = csv::Reader::from_reader(data.as_bytes());
    let mut edges = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        let from = rec.get(0).unwrap_or("").trim();
        let to = rec.get(1).unwrap_or("").trim();
        if !from.is_empty() && !to.is_empty() {
            edges.push((from.to_string(), to.to_string()));
        }
    }
    Ok(edges)
}

fn cmd_sync_dag(tasks: &mut HashMap<String, Task>, phase: &str) -> Result<()> {
    // gather edges from global DAG and phase DAG (if present)
    let mut edges = read_edges(&Path::new("docs/ROADMAP").join("DAG.csv"))?;
    let phase_path = Path::new("docs/ROADMAP").join(phase).join("DAG.csv");
    edges.extend(read_edges(&phase_path)?);

    if phase == "all" {
        for t in tasks.values_mut() {
            t.front.blocked_by.clear();
            t.front.blocks.clear();
        }
    } else {
        for t in tasks.values_mut() {
            if t.front.id.starts_with(phase) {
                t.front.blocked_by.clear();
                t.front.blocks.clear();
            }
        }
    }

    for (from, to) in edges {
        if phase != "all" {
            // skip edges not touching this phase
            if !from.starts_with(phase) && !to.starts_with(phase) {
                continue;
            }
        }
        if let Some(to_task) = tasks.get_mut(&to) {
            if !to_task.front.blocked_by.contains(&from) {
                to_task.front.blocked_by.push(from.clone());
            }
        }
        if let Some(from_task) = tasks.get_mut(&from) {
            if !from_task.front.blocks.contains(&to) {
                from_task.front.blocks.push(to.clone());
            }
        }
    }
    recompute_blocked(tasks);
    save_all(tasks)?;

    // refresh mermaid + svg
    let edges_all = collect_all_edges()?;
    let mermaid = generate_mermaid(tasks, &edges_all);
    let mmd_path = Path::new("docs/ROADMAP/roadmap.mmd");
    fs::write(mmd_path, mermaid)?;
    // best-effort SVG if mmdc exists
    if Command::new("which").arg("mmdc").output().map(|o| o.status.success()).unwrap_or(false) {
        let _ = Command::new("mmdc")
            .args(["-i", mmd_path.to_str().unwrap(), "-o", "docs/ROADMAP/roadmap.svg"])
            .status();
    }

    Ok(())
}

fn collect_all_edges() -> Result<Vec<(String, String)>> {
    let mut edges = read_edges(&Path::new("docs/ROADMAP").join("DAG.csv"))?;
    for entry in WalkDir::new("docs/ROADMAP") {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if name == "DAG.csv"
                    && entry.path().parent().map(|p| p.ends_with("ROADMAP")).unwrap_or(false)
                {
                    continue;
                }
                if name == "DAG.csv" {
                    edges.extend(read_edges(entry.path())?);
                }
            }
        }
    }
    Ok(edges)
}

fn append_edges(edges: &[(String, String)]) -> Result<()> {
    let dag_path = Path::new("docs/ROADMAP").join("DAG.csv");
    let mut existing = read_edges(&dag_path)?;
    for e in edges {
        if !existing.contains(e) {
            existing.push(e.clone());
        }
    }
    // write unique with header
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(&["from", "to"])?;
    for (from, to) in &existing {
        wtr.write_record(&[from, to])?;
    }
    let data = String::from_utf8(wtr.into_inner()?)?;
    fs::create_dir_all(dag_path.parent().unwrap())?;
    fs::write(dag_path, data)?;
    Ok(())
}

fn cmd_add(
    tasks: &mut HashMap<String, Task>,
    id: &str,
    title: &str,
    est: &str,
    value: &str,
    deps: Vec<String>,
) -> Result<()> {
    let phase = id.split('.').next().ok_or_else(|| anyhow!("bad id"))?;
    let readme_path = Path::new("docs/ROADMAP").join(phase).join("README.md");

    let fm = Frontmatter {
        id: id.to_string(),
        title: title.to_string(),
        estimate: Some(est.to_string()),
        status: Status::Open,
        blocked_by: deps.clone(),
        blocks: Vec::new(),
        value: Some(value.to_string()),
    };
    let body = "#### Summary\n- [ ] Fill in summary\n\n#### Steps\n1. Write/adjust tests\n2. Implement\n3. Update docs/ADR\n\n#### Test Plan\n- [ ] Unit tests\n- [ ] Integration tests\n".to_string();
    let task = Task { front: fm, body, path: readme_path.clone() };
    tasks.insert(id.to_string(), task);

    // append edges to global DAG
    let new_edges: Vec<(String, String)> =
        deps.iter().map(|d| (d.clone(), id.to_string())).collect();
    append_edges(&new_edges)?;

    // recompute and persist
    cmd_sync_dag(tasks, "all")?;
    Ok(())
}

fn compute_ready(tasks: &HashMap<String, Task>) -> Vec<String> {
    let mut ready = Vec::new();
    for (id, t) in tasks {
        if t.front.status != Status::Open {
            continue;
        }
        let mut blocked = false;
        for dep in &t.front.blocked_by {
            if let Some(dep_task) = tasks.get(dep) {
                if dep_task.front.status != Status::Closed {
                    blocked = true;
                    break;
                }
            } else {
                // missing dep, treat as blocked
                blocked = true;
                break;
            }
        }
        if !blocked {
            ready.push(id.clone());
        }
    }
    ready
}

fn parse_estimate(est: &Option<String>) -> f64 {
    if let Some(s) = est {
        let digits: String = s.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
        if let Ok(v) = digits.parse::<f64>() {
            return if v > 0.0 { v } else { 2.0 };
        }
    }
    2.0
}

fn value_weight(v: &Option<String>) -> f64 {
    match v.as_deref() {
        Some("H") => 3.0,
        Some("M") => 2.0,
        Some("L") => 1.0,
        _ => 1.0,
    }
}

fn suggest(tasks: &HashMap<String, Task>, scope: &str) -> Result<()> {
    let edges = collect_all_edges()?;
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (from, to) in edges {
        adj.entry(from).or_default().push(to);
    }
    // compute downstream depth (longest path) via dfs memo
    fn depth(
        node: &str,
        adj: &HashMap<String, Vec<String>>,
        memo: &mut HashMap<String, usize>,
    ) -> usize {
        if let Some(&d) = memo.get(node) {
            return d;
        }
        let d = 1 + adj
            .get(node)
            .map(|v| v.iter().map(|n| depth(n, adj, memo)).max().unwrap_or(0))
            .unwrap_or(0);
        memo.insert(node.to_string(), d);
        d
    }
    let mut memo = HashMap::new();
    let ready = compute_ready(tasks);
    let mut ranked = Vec::new();
    for id in ready {
        if scope != "all" && !id.starts_with(scope) {
            continue;
        }
        let t = tasks.get(&id).unwrap();
        let dur = parse_estimate(&t.front.estimate);
        let val = value_weight(&t.front.value);
        let score = val / dur;
        let dep_depth = depth(&id, &adj, &mut memo);
        ranked.push((score, dep_depth, id.clone(), t.front.title.clone(), dur, val));
    }
    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(b.1.cmp(&a.1)));
    println!("Ready tasks (sorted by value/duration, then downstream depth):");
    for (score, depth, id, title, dur, val) in &ranked {
        println!(
            "- {} ({}) score={:.2} value={} dur={}h depth={}",
            id, title, score, val, dur, depth
        );
    }
    if ranked.is_empty() {
        println!("No ready tasks in scope {scope}");
    }
    Ok(())
}
fn usage() {
    eprintln!(
        r"Usage:
  cargo run --bin tasks set <TASK_ID> <open|blocked|closed>
  cargo run --bin tasks block <FROM_ID> <TO_ID>
  cargo run --bin tasks enrich P2            # apply canned plans to phase 2
  cargo run --bin tasks sync-dag <PHASE|all> # import DAG.csv edges into blocked_by/blocks
  cargo run --bin tasks suggest [PHASE|all]  # list ready tasks ranked by value/duration
  cargo run --bin tasks add <TASK_ID> <TITLE> <EST> <VALUE> <DEP1,DEP2,...|->  # create task and edges
  cargo run --bin tasks it-nats              # spin up NATS via docker compose and run integration tests
  cargo run --bin tasks it-kafka             # spin up Kafka via docker compose and run integration tests
  cargo run --bin tasks it-elastic           # spin up Elasticsearch via docker compose and run integration tests"
    );
}

fn run(cmd: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(dir.unwrap_or_else(|| Path::new(".")))
        .status()
        .with_context(|| format!("failed to run {} {:?}", cmd, args))?;
    if !status.success() {
        Err(anyhow!("command {:?} {:?} failed with status {}", cmd, args, status))
    } else {
        Ok(())
    }
}

fn docker_compose(args: &[&str], dir: &Path) -> Result<()> {
    // Prefer `docker compose`; fall back to `docker-compose` if needed.
    let try_docker = Command::new("docker")
        .args(std::iter::once("compose").chain(args.iter().copied()))
        .current_dir(dir)
        .status();
    match try_docker {
        Ok(status) if status.success() => Ok(()),
        Ok(_) | Err(_) => run("docker-compose", args, Some(dir)),
    }
}

fn wait_for_host(hostport: &str, attempts: usize, sleep: Duration) -> Result<()> {
    for i in 0..attempts {
        if TcpStream::connect(hostport).is_ok() {
            return Ok(());
        }
        if i + 1 == attempts {
            break;
        }
        thread::sleep(sleep);
    }
    Err(anyhow!("service at {} did not become ready", hostport))
}

fn parse_host_port(addr: &str) -> Option<String> {
    let first = addr.split(',').next()?;
    let without_scheme = first.split("//").nth(1).unwrap_or(first);
    let host_port = without_scheme.split('/').next()?;
    if host_port.contains(':') && !host_port.is_empty() {
        Some(host_port.to_string())
    } else {
        None
    }
}

fn cmd_it_nats() -> Result<()> {
    let env_var = "NINE_LIVES_TEST_NATS_URL";
    let compose_dir = Path::new("ninelives-nats");

    let provided = std::env::var(env_var).ok();
    let url = provided.clone().unwrap_or_else(|| "nats://127.0.0.1:4222".to_string());
    let host_port = parse_host_port(&url).unwrap_or_else(|| "127.0.0.1:4222".to_string());

    let should_start_compose = provided.is_none();
    let _guard = if should_start_compose {
        docker_compose(&["up", "-d"], compose_dir)?;
        // best-effort cleanup
        struct Guard<'a> {
            dir: &'a Path,
        }
        impl<'a> Drop for Guard<'a> {
            fn drop(&mut self) {
                let _ = docker_compose(&["down", "-v"], self.dir);
            }
        }
        Some(Guard { dir: compose_dir })
    } else {
        None
    };

    wait_for_host(&host_port, 30, Duration::from_millis(250))
        .with_context(|| format!("waiting for NATS at {}", host_port))?;

    let mut cmd = Command::new("cargo");
    cmd.args(["test", "-p", "ninelives-nats"]);
    cmd.env(env_var, &url);
    let status = cmd.status().context("running cargo test -p ninelives-nats")?;
    if !status.success() {
        return Err(anyhow!("tests failed"));
    }
    Ok(())
}

fn cmd_it_kafka() -> Result<()> {
    let env_var = "NINE_LIVES_TEST_KAFKA_BROKERS";
    let compose_dir = Path::new("ninelives-kafka");

    let provided = std::env::var(env_var).ok();
    let brokers = provided.clone().unwrap_or_else(|| "127.0.0.1:9092".to_string());
    let host_port = parse_host_port(&brokers).unwrap_or_else(|| "127.0.0.1:9092".to_string());

    let should_start_compose = provided.is_none();
    let _guard = if should_start_compose {
        docker_compose(&["up", "-d"], compose_dir)?;
        struct Guard<'a> {
            dir: &'a Path,
        }
        impl<'a> Drop for Guard<'a> {
            fn drop(&mut self) {
                let _ = docker_compose(&["down", "-v"], self.dir);
            }
        }
        Some(Guard { dir: compose_dir })
    } else {
        None
    };

    wait_for_host(&host_port, 40, Duration::from_millis(300))
        .with_context(|| format!("waiting for Kafka at {}", host_port))?;

    let mut cmd = Command::new("cargo");
    cmd.args(["test", "-p", "ninelives-kafka"]);
    cmd.env(env_var, &brokers);
    let status = cmd.status().context("running cargo test -p ninelives-kafka")?;
    if !status.success() {
        return Err(anyhow!("tests failed"));
    }
    Ok(())
}

fn cmd_it_elastic() -> Result<()> {
    let env_var = "NINE_LIVES_TEST_ELASTIC_URL";
    let compose_dir = Path::new("ninelives-elastic");

    let provided = std::env::var(env_var).ok();
    let url = provided.clone().unwrap_or_else(|| "http://127.0.0.1:9200".to_string());
    let host_port = parse_host_port(&url).unwrap_or_else(|| "127.0.0.1:9200".to_string());

    let should_start_compose = provided.is_none();
    let _guard = if should_start_compose {
        docker_compose(&["up", "-d"], compose_dir)?;
        struct Guard<'a> {
            dir: &'a Path,
        }
        impl<'a> Drop for Guard<'a> {
            fn drop(&mut self) {
                let _ = docker_compose(&["down", "-v"], self.dir);
            }
        }
        Some(Guard { dir: compose_dir })
    } else {
        None
    };

    wait_for_host(&host_port, 50, std::time::Duration::from_millis(300))
        .with_context(|| format!("waiting for Elasticsearch at {}", host_port))?;

    let status = Command::new("cargo")
        .args(["test", "-p", "ninelives-elastic"])
        .env(env_var, &url)
        .status()
        .context("running cargo test -p ninelives-elastic")?;
    if !status.success() {
        return Err(anyhow!("tests failed"));
    }
    Ok(())
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
        "it-kafka" => {
            cmd_it_kafka()?;
        }
        "it-elastic" => {
            cmd_it_elastic()?;
        }
        "enrich" => {
            if args.len() != 1 {
                usage();
                std::process::exit(1);
            }
            let phase = &args[0];
            cmd_enrich(&mut tasks, phase)?;
            println!("Enriched tasks for {phase}");
        }
        "sync-dag" => {
            if args.is_empty() || args.len() > 1 {
                usage();
                std::process::exit(1);
            }
            let phase = &args[0];
            if phase == "all" {
                // iterate all phase directories
                for entry in WalkDir::new("docs/ROADMAP") {
                    let entry = entry?;
                    if entry.file_type().is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with('P') && name.len() <= 3 {
                                cmd_sync_dag(&mut tasks, name)?;
                            }
                        }
                    }
                }
                println!("Synced DAG for all phases");
            } else {
                cmd_sync_dag(&mut tasks, phase)?;
                println!("Synced DAG for {phase}");
            }
        }
        "suggest" => {
            let scope = if args.is_empty() { "all".to_string() } else { args[0].clone() };
            suggest(&tasks, &scope)?;
        }
        "add" => {
            if args.len() < 5 {
                usage();
                std::process::exit(1);
            }
            let id = args[0].clone();
            let title = args[1].clone();
            let est = args[2].clone();
            let value = args[3].clone();
            let dep_arg = args[4].clone();
            let deps: Vec<String> = if dep_arg == "-" {
                Vec::new()
            } else {
                dep_arg.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
            };
            cmd_add(&mut tasks, &id, &title, &est, &value, deps)?;
            println!("Added task {id}");
        }
        "it-nats" => {
            cmd_it_nats()?;
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
    Ok(())
}
