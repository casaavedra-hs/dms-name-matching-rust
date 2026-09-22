use std::{
    collections::VecDeque,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{Context, Result};
use crossbeam_channel::unbounded;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    db,
    engine::{self, EngineControl},
    model::{AppConfig, DbConfig, EngineEvent, JobSnapshot, MatchLevel, APP_NAME, APP_VERSION},
    report,
};

const INDEX_HTML: &str = include_str!("../web/index.html");
const APP_JS: &str = include_str!("../web/app.js");
const APP_CSS: &str = include_str!("../web/app.css");

struct UiState {
    config: AppConfig,
    snapshot: JobSnapshot,
    logs: VecDeque<String>,
    active_job_id: String,
    running: bool,
    paused: bool,
    notice: String,
    error: String,
    control: Option<EngineControl>,
    root: PathBuf,
    config_path: PathBuf,
}

impl UiState {
    fn bootstrap_json(&self) -> Value {
        let mut cfg = self.config.clone();
        cfg.source1.password.clear();
        cfg.source2.password.clear();
        cfg.destination.password.clear();
        json!({
            "app_name": APP_NAME,
            "app_version": APP_VERSION,
            "config": cfg,
            "snapshot": self.snapshot,
            "logs": self.logs,
            "active_job_id": self.active_job_id,
            "running": self.running,
            "paused": self.paused,
            "notice": self.notice,
            "error": self.error,
            "core_levels": MatchLevel::CORE.iter().map(level_json).collect::<Vec<_>>(),
            "advanced_levels": MatchLevel::ADVANCED.iter().map(level_json).collect::<Vec<_>>()
        })
    }
}

fn level_json(level: &MatchLevel) -> Value {
    json!({
        "value": format!("{:?}", level),
        "code": level.code(),
        "label": level.label(),
        "phase": if level.exact_phase() { "Exact" } else { "Fuzzy" }
    })
}

#[derive(Debug, Deserialize)]
struct SaveConfigRequest {
    config: AppConfig,
    #[serde(default)]
    remember_passwords: bool,
}

#[derive(Debug, Deserialize)]
struct JobRequest {
    config: AppConfig,
    #[serde(default)]
    job_id: String,
}

#[derive(Debug, Deserialize)]
struct PauseRequest { paused: bool }

#[derive(Debug, Deserialize)]
struct ReportRequest { job_id: String }

#[derive(Debug, Deserialize)]
struct HistoryRequest {
    #[serde(default = "default_history_limit")]
    limit: usize,
}

fn default_history_limit() -> usize { 100 }

#[derive(Debug, Serialize)]
struct ApiResponse<T: Serialize> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self { Self { ok: true, data: Some(data), error: None } }
}

fn api_error(message: impl Into<String>) -> ApiResponse<Value> {
    ApiResponse { ok: false, data: None, error: Some(message.into()) }
}

pub fn run() -> Result<()> {
    let root = application_root()?;
    let runtime = root.join("runtime");
    fs::create_dir_all(&runtime)?;
    fs::create_dir_all(root.join("reports"))?;
    let config_path = runtime.join("app_config.json");
    let mut config = fs::read_to_string(&config_path)
        .ok()
        .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
        .unwrap_or_default();
    config.source1.password.clear();
    config.source2.password.clear();
    config.destination.password.clear();

    let state = Arc::new(Mutex::new(UiState {
        config,
        snapshot: JobSnapshot::default(),
        logs: VecDeque::with_capacity(500),
        active_job_id: String::new(),
        running: false,
        paused: false,
        notice: String::new(),
        error: String::new(),
        control: None,
        root: root.clone(),
        config_path,
    }));

    let listener = TcpListener::bind("127.0.0.1:0").context("Unable to bind local DMS UI server")?;
    let addr = listener.local_addr()?;
    let url = format!("http://{}", addr);
    write_startup_log(&root, &format!("{} {} listening on {}", APP_NAME, APP_VERSION, url));
    let _ = fs::write(root.join("runtime").join("ui-url.txt"), &url);
    open_browser(&url);

    println!("{} {}", APP_NAME, APP_VERSION);
    println!("Local UI: {url}");
    println!("Keep this process running while using the application.");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&state);
                thread::spawn(move || {
                    if let Err(e) = handle_connection(stream, state) {
                        eprintln!("UI request failed: {e:#}");
                    }
                });
            }
            Err(e) => eprintln!("UI listener error: {e}"),
        }
    }
    Ok(())
}

fn application_root() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("Unable to locate application executable")?;
    let bin = exe.parent().context("Application executable has no parent directory")?;
    if bin.file_name().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("bin")).unwrap_or(false) {
        Ok(bin.parent().unwrap_or(bin).to_path_buf())
    } else {
        Ok(std::env::current_dir().unwrap_or_else(|_| bin.to_path_buf()))
    }
}

fn write_startup_log(root: &Path, line: &str) {
    let path = root.join("runtime").join("application.log");
    let _ = fs::OpenOptions::new().create(true).append(true).open(path)
        .and_then(|mut f| writeln!(f, "{}", line));
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    { let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn(); }
    #[cfg(target_os = "macos")]
    { let _ = Command::new("open").arg(url).spawn(); }
    #[cfg(all(unix, not(target_os = "macos")))]
    { let _ = Command::new("xdg-open").arg(url).spawn(); }
}

fn handle_connection(mut stream: TcpStream, state: Arc<Mutex<UiState>>) -> Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(15))).ok();
    let request = read_http_request(&mut stream)?;
    let response = route(&request.method, &request.path, &request.body, state);
    write_http_response(&mut stream, response)?;
    Ok(())
}

struct HttpRequest { method: String, path: String, body: Vec<u8> }
struct HttpResponse { status: u16, content_type: &'static str, body: Vec<u8> }

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    const MAX_HEADER: usize = 64 * 1024;
    const MAX_BODY: usize = 10 * 1024 * 1024;
    let mut buf = Vec::with_capacity(8192);
    let mut tmp = [0u8; 8192];
    let header_end;
    loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 { anyhow::bail!("Connection closed before request headers completed"); }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > MAX_HEADER { anyhow::bail!("HTTP request headers are too large"); }
        if let Some(pos) = find_bytes(&buf, b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
    }
    let header = std::str::from_utf8(&buf[..header_end]).context("Invalid HTTP request header encoding")?;
    let mut lines = header.split("\r\n");
    let request_line = lines.next().context("Missing HTTP request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").split('?').next().unwrap_or("/").to_string();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }
    if content_length > MAX_BODY { anyhow::bail!("HTTP request body is too large"); }
    while buf.len() < header_end + content_length {
        let n = stream.read(&mut tmp)?;
        if n == 0 { break; }
        buf.extend_from_slice(&tmp[..n]);
    }
    if buf.len() < header_end + content_length { anyhow::bail!("Incomplete HTTP request body"); }
    Ok(HttpRequest { method, path, body: buf[header_end..header_end + content_length].to_vec() })
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn route(method: &str, path: &str, body: &[u8], state: Arc<Mutex<UiState>>) -> HttpResponse {
    let result = match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => return text_response(200, "text/html; charset=utf-8", INDEX_HTML),
        ("GET", "/app.js") => return text_response(200, "application/javascript; charset=utf-8", APP_JS),
        ("GET", "/app.css") => return text_response(200, "text/css; charset=utf-8", APP_CSS),
        ("GET", "/favicon.ico") => return HttpResponse { status: 204, content_type: "image/x-icon", body: vec![] },
        ("GET", "/api/health") => Ok(json!({"status":"ok","version":APP_VERSION})),
        ("GET", "/api/bootstrap") => state.lock().map(|s| s.bootstrap_json()).map_err(|_| anyhow::anyhow!("UI state is unavailable")),
        ("POST", "/api/config/save") => save_config(body, &state),
        ("POST", "/api/db/test") => parse_json::<DbConfig>(body).and_then(|cfg| db::test_connection(&cfg).map(|v| json!({"version":v}))),
        ("POST", "/api/db/databases") => parse_json::<DbConfig>(body).and_then(|cfg| db::list_databases(&cfg).map(|v| json!(v))),
        ("POST", "/api/db/tables") => parse_json::<DbConfig>(body).and_then(|cfg| db::list_tables(&cfg).map(|v| json!(v))),
        ("POST", "/api/db/columns") => parse_json::<DbConfig>(body).and_then(|cfg| db::list_columns(&cfg).map(|v| json!(v))),
        ("POST", "/api/job/start") => start_job(body, &state, false),
        ("POST", "/api/job/resume") => start_job(body, &state, true),
        ("POST", "/api/job/pause") => pause_job(body, &state),
        ("POST", "/api/job/stop") => stop_job(&state),
        ("POST", "/api/history") => load_history(body, &state),
        ("POST", "/api/report/load") => load_report(body, &state),
        ("POST", "/api/report/export") => export_report(body, &state),
        _ => return json_response(404, &api_error("Not found")),
    };
    match result {
        Ok(v) => json_response(200, &ApiResponse::ok(v)),
        Err(e) => json_response(400, &api_error(format!("{e:#}"))),
    }
}

fn parse_json<T: for<'de> Deserialize<'de>>(body: &[u8]) -> Result<T> {
    serde_json::from_slice(body).context("Invalid JSON request")
}

fn save_config(body: &[u8], state: &Arc<Mutex<UiState>>) -> Result<Value> {
    let req: SaveConfigRequest = parse_json(body)?;
    let mut cfg_to_save = req.config.clone();
    if !req.remember_passwords {
        cfg_to_save.source1.password.clear();
        cfg_to_save.source2.password.clear();
        cfg_to_save.destination.password.clear();
    }
    let (path, serialized) = {
        let mut s = state.lock().map_err(|_| anyhow::anyhow!("UI state is unavailable"))?;
        s.config = req.config;
        s.notice = "Configuration saved locally.".into();
        s.error.clear();
        (s.config_path.clone(), serde_json::to_string_pretty(&cfg_to_save)?)
    };
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    fs::write(path, serialized)?;
    Ok(json!({"saved":true}))
}

fn start_job(body: &[u8], state: &Arc<Mutex<UiState>>, resume: bool) -> Result<Value> {
    let req: JobRequest = parse_json(body)?;
    {
        let s = state.lock().map_err(|_| anyhow::anyhow!("UI state is unavailable"))?;
        if s.running { anyhow::bail!("A job is already running. Stop it or wait for completion before starting another job."); }
    }
    let (tx, rx) = unbounded::<EngineEvent>();
    let control = EngineControl::default();
    let resume_id = if resume && !req.job_id.trim().is_empty() { Some(req.job_id.trim().to_string()) } else { None };
    let job_id = engine::spawn_job(req.config.clone(), resume_id, tx, control.clone());
    {
        let mut s = state.lock().map_err(|_| anyhow::anyhow!("UI state is unavailable"))?;
        s.config = req.config;
        s.active_job_id = job_id.clone();
        s.running = true;
        s.paused = false;
        s.control = Some(control);
        s.error.clear();
        s.notice = if resume { "Resume requested.".into() } else { "Processing started.".into() };
        s.logs.clear();
    }
    let state2 = Arc::clone(state);
    thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            let mut s = match state2.lock() { Ok(v) => v, Err(_) => return };
            match event {
                EngineEvent::Snapshot(snapshot) => s.snapshot = snapshot,
                EngineEvent::Log(line) => {
                    if s.logs.len() >= 500 { s.logs.pop_front(); }
                    s.logs.push_back(line);
                }
                EngineEvent::Finished(snapshot) => {
                    s.snapshot = snapshot;
                    s.running = false;
                    s.paused = false;
                    s.control = None;
                    s.notice = "Processing completed and reconciled successfully.".into();
                    break;
                }
                EngineEvent::Failed(message) => {
                    s.running = false;
                    s.paused = false;
                    s.control = None;
                    s.error = message;
                    break;
                }
            }
        }
    });
    Ok(json!({"job_id":job_id}))
}

fn pause_job(body: &[u8], state: &Arc<Mutex<UiState>>) -> Result<Value> {
    let req: PauseRequest = parse_json(body)?;
    let mut s = state.lock().map_err(|_| anyhow::anyhow!("UI state is unavailable"))?;
    let control = s.control.clone().context("No job is currently running")?;
    control.request_pause(req.paused);
    s.paused = req.paused;
    s.notice = if req.paused { "Pause requested. The engine will pause at a safe checkpoint.".into() } else { "Resume requested.".into() };
    Ok(json!({"paused":req.paused}))
}

fn stop_job(state: &Arc<Mutex<UiState>>) -> Result<Value> {
    let mut s = state.lock().map_err(|_| anyhow::anyhow!("UI state is unavailable"))?;
    let control = s.control.clone().context("No job is currently running")?;
    control.request_stop();
    s.notice = "Stop requested. The engine will stop at a safe checkpoint.".into();
    Ok(json!({"stop_requested":true}))
}

fn load_history(body: &[u8], state: &Arc<Mutex<UiState>>) -> Result<Value> {
    let req: HistoryRequest = parse_json(body).unwrap_or(HistoryRequest { limit: 100 });
    let dest = state.lock().map_err(|_| anyhow::anyhow!("UI state is unavailable"))?.config.destination.clone();
    let rows = engine::list_recent_jobs(&dest, req.limit.clamp(1, 500))?;
    Ok(json!(rows))
}

fn load_report(body: &[u8], state: &Arc<Mutex<UiState>>) -> Result<Value> {
    let req: ReportRequest = parse_json(body)?;
    let dest = state.lock().map_err(|_| anyhow::anyhow!("UI state is unavailable"))?.config.destination.clone();
    let report = report::load_vital_report(&dest, req.job_id.trim())?;
    Ok(serde_json::to_value(report)?)
}

fn export_report(body: &[u8], state: &Arc<Mutex<UiState>>) -> Result<Value> {
    let req: ReportRequest = parse_json(body)?;
    let (dest, root) = {
        let s = state.lock().map_err(|_| anyhow::anyhow!("UI state is unavailable"))?;
        (s.config.destination.clone(), s.root.join("reports"))
    };
    let path = report::export_job(&dest, req.job_id.trim(), &root)?;
    Ok(json!({"path":path.to_string_lossy()}))
}

fn text_response(status: u16, content_type: &'static str, text: &str) -> HttpResponse {
    HttpResponse { status, content_type, body: text.as_bytes().to_vec() }
}

fn json_response<T: Serialize>(status: u16, value: &T) -> HttpResponse {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{\"ok\":false,\"error\":\"Serialization failed\"}".to_vec());
    HttpResponse { status, content_type: "application/json; charset=utf-8", body }
}

fn write_http_response(stream: &mut TcpStream, response: HttpResponse) -> Result<()> {
    let reason = match response.status {
        200 => "OK", 204 => "No Content", 400 => "Bad Request", 404 => "Not Found", 500 => "Internal Server Error", _ => "OK",
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        response.status, reason, response.content_type, response.body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()?;
    Ok(())
}
