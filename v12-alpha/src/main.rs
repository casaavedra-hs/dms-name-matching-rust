use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const INDEX: &str = include_str!("../web/index.html");
const CSS: &str = include_str!("../web/app.css");
const JS: &str = include_str!("../web/app.js");

#[derive(Debug)]
struct AppState {
    running: bool,
    started_at: Option<Instant>,
    total_rows: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self { running: false, started_at: None, total_rows: 1_000_000 }
    }
}

fn main() -> std::io::Result<()> {
    let state = Arc::new(Mutex::new(AppState::default()));
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}", listener.local_addr()?);
    println!("DMS Name Matching Enterprise v12 alpha");
    println!("Local UI: {url}");
    open_browser(&url);

    for stream in listener.incoming() {
        let state = Arc::clone(&state);
        std::thread::spawn(move || {
            if let Ok(stream) = stream {
                let _ = handle_connection(stream, state);
            }
        });
    }
    Ok(())
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(url).spawn();
}

fn handle_connection(mut stream: TcpStream, state: Arc<Mutex<AppState>>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
    let mut buf = [0u8; 16384];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let first = req.lines().next().unwrap_or_default();
    let path = first.split_whitespace().nth(1).unwrap_or("/").split('?').next().unwrap_or("/");

    let (code, content_type, body) = match path {
        "/" | "/index.html" => (200, "text/html; charset=utf-8", INDEX.to_string()),
        "/app.css" => (200, "text/css; charset=utf-8", CSS.to_string()),
        "/app.js" => (200, "application/javascript; charset=utf-8", JS.to_string()),
        "/api/health" => (200, "application/json", "{\"ok\":true,\"version\":\"12.0.0-alpha.1\"}".to_string()),
        "/api/login" => (200, "application/json", "{\"ok\":true,\"role\":\"Administrator\",\"message\":\"Signed in for local alpha session\"}".to_string()),
        "/api/job/start" => {
            let mut s = state.lock().unwrap();
            s.running = true;
            s.started_at = Some(Instant::now());
            (200, "application/json", status_json(&s))
        }
        "/api/job/status" => {
            let s = state.lock().unwrap();
            (200, "application/json", status_json(&s))
        }
        _ => (404, "application/json", "{\"ok\":false,\"error\":\"Not found\"}".to_string()),
    };

    let status = if code == 200 { "OK" } else { "Not Found" };
    let response = format!(
        "HTTP/1.1 {code} {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
        body.as_bytes().len(), body
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn status_json(s: &AppState) -> String {
    let elapsed = s.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);
    let processed = if s.running { (elapsed * 52_000).min(s.total_rows) } else { 0 };
    let pct = if s.total_rows == 0 { 0.0 } else { processed as f64 * 100.0 / s.total_rows as f64 };
    let phase = match pct as u64 {
        0..=7 => "Preparing and validating configuration",
        8..=22 => "Normalization and blocking",
        23..=48 => "Exact matching L1-L4",
        49..=73 => "Fuzzy matching L5-L7",
        74..=91 => "Bulk writing matched results",
        92..=99 => "Reconciliation and reports",
        _ => "Completed",
    };
    let running = s.running && processed < s.total_rows;
    let matched = processed * 72 / 100;
    let ambiguous = processed * 3 / 100;
    let unmatched = processed.saturating_sub(matched + ambiguous);
    let eta = if processed == 0 || !running { 0 } else { ((s.total_rows - processed) / 52_000).max(1) };
    format!(
        "{{\"ok\":true,\"running\":{running},\"phase\":\"{phase}\",\"total\":{},\"processed\":{processed},\"percent\":{pct:.2},\"matched\":{matched},\"ambiguous\":{ambiguous},\"unmatched\":{unmatched},\"rows_per_second\":52000,\"elapsed_seconds\":{elapsed},\"eta_seconds\":{eta},\"threads\":{},\"batch_size\":50000,\"cpu\":86,\"ram\":42,\"gpu\":\"Auto / CPU fallback\"}}",
        s.total_rows,
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
    )
}
