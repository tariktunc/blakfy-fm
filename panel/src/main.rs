#![windows_subsystem = "windows"]
//! Blakfy FM panel: start / stop / restart the Discord music bot stack,
//! watch RAM/CPU, read logs and commands, get the YouTube login code.

use eframe::egui;
use egui::{Align2, Color32, CornerRadius, FontId, Margin, RichText, Sense, Stroke, pos2, vec2};
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const BOT_DIR: &str = r"C:\Users\Tarik\Desktop\github\discord-muzik-botu";
const HENKER_EXE: &str = r"C:\Users\Tarik\Downloads\HenkerDPI-v3.0.5\HenkerDPI.exe";
const DOCKER_DESKTOP: &str = r"C:\Program Files\Docker\Docker\Docker Desktop.exe";
const CIPHER: &str = "yt-cipher";
const LAVALINK: &str = "lavamusic-lavalink";
const BOT: &str = "lavamusic";
const PROXY: &str = "radio-proxy";
const NO_WINDOW: u32 = 0x0800_0000;
/// #sohbet on the Blakfy server. Override with BLAKFY_FM_CHANNEL_ID in .env.
const SOHBET_CHANNEL: &str = "982026261025157210";
/// #music on the Blakfy server (the bot's own text channel), fallback.
const MUSIC_CHANNEL: &str = "1439982697593176337";

/// Containers of the stack, in display order: (container, label).
const CONTAINERS: [(&str, &str); 4] = [
    (BOT, "Bot"),
    (LAVALINK, "Lavalink"),
    (PROXY, "Radyo dönüştürücü"),
    (CIPHER, "Şifre çözücü"),
];

// ---------- theme ----------
const BG: Color32 = Color32::from_rgb(14, 15, 20);
const CARD: Color32 = Color32::from_rgb(23, 25, 35);
const CARD_HI: Color32 = Color32::from_rgb(31, 34, 48);
const BORDER: Color32 = Color32::from_rgb(42, 46, 61);
const TEXT: Color32 = Color32::from_rgb(231, 233, 240);
const MUTED: Color32 = Color32::from_rgb(138, 144, 166);
const ACCENT: Color32 = Color32::from_rgb(139, 92, 246);
const ACCENT2: Color32 = Color32::from_rgb(236, 72, 153);
const GREEN: Color32 = Color32::from_rgb(34, 197, 94);
const RED: Color32 = Color32::from_rgb(239, 68, 68);
const AMBER: Color32 = Color32::from_rgb(245, 158, 11);

// ---------- process helpers ----------

fn run(prog: &str, args: &[&str]) -> (bool, String) {
    match Command::new(prog)
        .args(args)
        .current_dir(BOT_DIR)
        .creation_flags(NO_WINDOW)
        .output()
    {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).to_string();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            (o.status.success(), s.trim().to_string())
        }
        Err(e) => (false, e.to_string()),
    }
}

/// Reads one key from the bot's .env (value never logged).
fn env_value(key: &str) -> Option<String> {
    let txt = std::fs::read_to_string(format!(r"{BOT_DIR}\.env")).ok()?;
    for l in txt.lines() {
        let l = l.trim();
        if l.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            if k.trim() == key {
                let v = v.trim();
                // Quoted value: take up to the closing quote (ignores inline comments).
                let v = if let Some(q) = v.chars().next().filter(|c| *c == '"' || *c == '\'') {
                    v[1..].split(q).next().unwrap_or("").to_string()
                } else {
                    v.split(" #").next().unwrap_or("").trim().to_string()
                };
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Runs a PowerShell script with secrets passed via env vars (not argv).
fn ps_env(script: &str, vars: &[(&str, &str)]) -> (bool, String) {
    let mut c = Command::new("powershell");
    c.args(["-NoProfile", "-Command", script]).creation_flags(NO_WINDOW);
    for (k, v) in vars {
        c.env(k, v);
    }
    match c.output() {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).to_string();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            let s: String = s.trim().chars().take(300).collect();
            (o.status.success(), s)
        }
        Err(e) => (false, e.to_string()),
    }
}

/// Sends a message to Discord: BLAKFY_FM_CHANNEL_ID, then #sohbet, then #music.
/// The bot has no access to #sohbet on the Blakfy server (Discord 50001).
fn notify(s: &S, text: &str) {
    let Some(token) = env_value("TOKEN") else {
        log(s, "Discord mesajı: .env içinde TOKEN yok.");
        return;
    };
    let mut channels: Vec<String> = Vec::new();
    if let Some(c) = env_value("BLAKFY_FM_CHANNEL_ID") {
        channels.push(c);
    }
    channels.push(SOHBET_CHANNEL.to_string());
    channels.push(MUSIC_CHANNEL.to_string());

    let body = format!("{{\"content\":\"{}\"}}", text.replace('\\', "\\\\").replace('"', "\\\""));
    // Prints only Discord's short error text, never the full PowerShell trace.
    let script = "[Net.ServicePointManager]::SecurityProtocol='Tls12'; try { Invoke-RestMethod -UserAgent 'DiscordBot (https://blakfy.com, 1.0)' -Method Post -Uri ('https://discord.com/api/v10/channels/'+$env:BFM_CH+'/messages') -Headers @{Authorization='Bot '+$env:BFM_TOKEN} -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes($env:BFM_BODY)) | Out-Null } catch { $m = $_.ErrorDetails.Message; if (-not $m) { $m = $_.Exception.Message }; Write-Output $m; exit 1 }";
    let mut last_err = String::new();
    for ch in &channels {
        let (ok, out) = ps_env(script, &[("BFM_TOKEN", &token), ("BFM_CH", ch), ("BFM_BODY", &body)]);
        if ok {
            log(s, format!("Discord'a yazıldı: {text}"));
            return;
        }
        last_err = out;
    }
    let reason = if last_err.contains("50001") || last_err.contains("50013") {
        "botun kanala yazma izni yok".to_string()
    } else {
        last_err
    };
    log(s, format!("Discord mesajı gönderilemedi: {reason}"));
}

fn docker_ok() -> bool {
    run("docker", &["info", "--format", "{{.ServerVersion}}"]).0
}

fn container_state(name: &str) -> String {
    let (ok, out) = run(
        "docker",
        &[
            "inspect",
            "-f",
            "{{.State.Status}}{{if .State.Health}} ({{.State.Health.Status}}){{end}}",
            name,
        ],
    );
    if ok { out } else { "yok".into() }
}

fn henker_running() -> bool {
    let (_, out) = run("tasklist", &["/FI", "IMAGENAME eq HenkerDPI.exe", "/NH"]);
    out.to_lowercase().contains("henkerdpi.exe")
}

// ---------- status ----------

#[derive(Clone, Default)]
struct Svc {
    label: String,
    running: bool,
    /// Human Turkish status, e.g. "Açık · 12 dk · sağlıklı".
    text: String,
    /// (used MiB, limit MiB)
    mem: Option<(f64, f64)>,
    cpu: Option<f64>,
}

#[derive(Clone, Default)]
struct Status {
    loaded: bool,
    docker: bool,
    henker: bool,
    svcs: Vec<Svc>,
}

impl Status {
    fn running_count(&self) -> usize {
        self.svcs.iter().filter(|s| s.running).count()
    }
    fn all_up(&self) -> bool {
        self.docker && self.henker && !self.svcs.is_empty() && self.running_count() == self.svcs.len()
    }
    fn bot_up(&self) -> bool {
        self.svcs.first().map(|s| s.running).unwrap_or(false)
    }
    fn total_mem(&self) -> f64 {
        self.svcs.iter().filter_map(|s| s.mem.map(|m| m.0)).sum()
    }
}

/// Parses "294.4MiB" / "1.2GiB" / "512kB" into MiB.
fn to_mib(s: &str) -> f64 {
    let s = s.trim();
    let (num, mul) = if let Some(n) = s.strip_suffix("GiB") {
        (n, 1024.0)
    } else if let Some(n) = s.strip_suffix("MiB") {
        (n, 1.0)
    } else if let Some(n) = s.strip_suffix("KiB").or_else(|| s.strip_suffix("kB")) {
        (n, 1.0 / 1024.0)
    } else if let Some(n) = s.strip_suffix('B') {
        (n, 1.0 / 1024.0 / 1024.0)
    } else {
        (s, 1.0)
    };
    num.trim().parse::<f64>().unwrap_or(0.0) * mul
}

/// "Up 5 minutes (healthy)" -> "Açık · 5 dk · sağlıklı"; "Exited (143) 3 hours ago" -> "Kapalı · 3 sa önce".
fn tr_status(s: &str) -> String {
    let mut t = s.to_string();
    for (a, b) in [
        ("Less than a second", "az önce"),
        ("About a minute", "1 dk"),
        ("About an hour", "1 sa"),
        (" seconds", " sn"),
        (" second", " sn"),
        (" minutes", " dk"),
        (" minute", " dk"),
        (" hours", " sa"),
        (" hour", " sa"),
        (" days", " gün"),
        (" day", " gün"),
        (" weeks", " hafta"),
        (" week", " hafta"),
        (" months", " ay"),
        (" ago", " önce"),
        ("(healthy)", "· sağlıklı"),
        ("(unhealthy)", "· sorunlu"),
        ("(health: starting)", "· başlıyor"),
    ] {
        t = t.replace(a, b);
    }
    if let Some(r) = t.strip_prefix("Up ") {
        format!("Açık · {r}")
    } else if t.starts_with("Exited") {
        let r = t.split(')').nth(1).unwrap_or("").trim().to_string();
        if r.is_empty() { "Kapalı".into() } else { format!("Kapalı · {r}") }
    } else if t.starts_with("Restarting") {
        "Yeniden başlıyor".into()
    } else if t.starts_with("Created") {
        "Oluşturuldu, çalışmıyor".into()
    } else {
        t
    }
}

fn read_status() -> Status {
    let docker = docker_ok();
    let henker = henker_running();
    let mut svcs: Vec<Svc> = CONTAINERS
        .iter()
        .map(|(_, label)| Svc { label: label.to_string(), text: "-".into(), ..Default::default() })
        .collect();
    if docker {
        let (_, ps) = run("docker", &["ps", "-a", "--format", "{{.Names}}|{{.State}}|{{.Status}}"]);
        for line in ps.lines() {
            let mut p = line.splitn(3, '|');
            let (Some(name), Some(state), Some(status)) = (p.next(), p.next(), p.next()) else { continue };
            if let Some(i) = CONTAINERS.iter().position(|(c, _)| *c == name) {
                svcs[i].running = state == "running";
                svcs[i].text = tr_status(status);
            }
        }
        for s in svcs.iter_mut().filter(|s| s.text == "-") {
            s.text = "Kurulu değil".into();
        }
        if svcs.iter().any(|s| s.running) {
            let (_, st) = run("docker", &["stats", "--no-stream", "--format", "{{.Name}}|{{.MemUsage}}|{{.CPUPerc}}"]);
            for line in st.lines() {
                let mut p = line.splitn(3, '|');
                let (Some(name), Some(mem), Some(cpu)) = (p.next(), p.next(), p.next()) else { continue };
                if let Some(i) = CONTAINERS.iter().position(|(c, _)| *c == name) {
                    if let Some((u, l)) = mem.split_once('/') {
                        svcs[i].mem = Some((to_mib(u), to_mib(l)));
                    }
                    svcs[i].cpu = cpu.trim().trim_end_matches('%').parse::<f64>().ok();
                }
            }
        }
    }
    Status { loaded: true, docker, henker, svcs }
}

// ---------- schedule (mirrors the bot's radioSchedule.ts) ----------

/// (days 0=Sun..6=Sat, start min, end min, name) in Turkey time.
const SLOTS: [(&[u32], u32, u32, &str); 3] = [
    (&[1, 2, 3, 4, 5], 7 * 60, 9 * 60, "Nihat'la Muhabbet"),
    (&[1, 2, 3, 4, 5], 18 * 60, 20 * 60, "Nihat'la Sivrisinek"),
    (&[6], 17 * 60, 19 * 60, "Nihat'la Sivrisinek"),
];

fn fmt_dur(min: u32) -> String {
    let (d, h, m) = (min / 1440, (min % 1440) / 60, min % 60);
    if d > 0 {
        format!("{d} gün {h} sa")
    } else if h > 0 {
        format!("{h} sa {m} dk")
    } else {
        format!("{m} dk")
    }
}

/// (is_live, text) for the scheduled Kafa Radyo. Turkey is fixed UTC+3.
fn schedule_text() -> (bool, String) {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) + 3 * 3600;
    let days = secs / 86400;
    let dow = ((days + 4) % 7) as u32; // 1970-01-01 was a Thursday
    let now = ((secs % 86400) / 60) as u32;
    for (ds, s, e, name) in SLOTS {
        if ds.contains(&dow) && now >= s && now < e {
            return (true, format!("Şu an: {name} · bitişe {}", fmt_dur(e - now)));
        }
    }
    // Next start within 8 days.
    for ahead in 1..=(8 * 1440u32) {
        let t = now + ahead;
        let d = (dow + t / 1440) % 7;
        let m = t % 1440;
        for (ds, s, _, name) in SLOTS {
            if ds.contains(&d) && m == s {
                return (false, format!("Sıradaki: {name} · {:02}:{:02} · {} sonra", s / 60, s % 60, fmt_dur(ahead)));
            }
        }
    }
    (false, String::new())
}

// ---------- shared state ----------

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Activity,
    Commands,
    BotLog,
    LavaLog,
}

#[derive(Default)]
struct Shared {
    status: Status,
    /// Total RAM samples (MiB), one per poll.
    ram_hist: Vec<f64>,
    log: Vec<String>,
    busy: bool,
    busy_label: String,
    yt_code: Option<String>,
    /// Text of the Komutlar / Bot logu / Lavalink logu tab.
    tab_text: String,
}

type S = Arc<Mutex<Shared>>;

fn log(s: &S, msg: impl Into<String>) {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) + 3 * 3600;
    let stamp = format!("{:02}:{:02}:{:02}", (secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    let mut g = s.lock().unwrap();
    g.log.push(format!("{stamp}  {}", msg.into()));
    let n = g.log.len();
    if n > 300 {
        g.log.drain(0..n - 300);
    }
}

// ---------- actions ----------

fn ensure_docker(s: &S) -> bool {
    if docker_ok() {
        return true;
    }
    log(s, "Docker Desktop açılıyor...");
    let _ = Command::new(DOCKER_DESKTOP).creation_flags(NO_WINDOW).spawn();
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(180) {
        thread::sleep(Duration::from_secs(3));
        if docker_ok() {
            log(s, "Docker hazır.");
            return true;
        }
    }
    log(s, "HATA: Docker 3 dakikada açılmadı.");
    false
}

fn ensure_henker(s: &S) {
    if henker_running() {
        return;
    }
    log(s, "HenkerDPI açılıyor (yönetici onayı istenebilir)...");
    let cmd = format!("Start-Process -FilePath '{HENKER_EXE}' -Verb RunAs");
    let (ok, out) = run("powershell", &["-NoProfile", "-Command", &cmd]);
    if !ok {
        log(s, format!("HATA: HenkerDPI açılamadı: {out}"));
    }
}

fn step(s: &S, label: &str, prog: &str, args: &[&str]) -> bool {
    log(s, format!("{label}..."));
    s.lock().unwrap().busy_label = label.to_string();
    let (ok, out) = run(prog, args);
    if !ok {
        log(s, format!("HATA ({label}): {}", out.lines().last().unwrap_or("")));
    }
    ok
}

fn wait_healthy(s: &S, name: &str) -> bool {
    s.lock().unwrap().busy_label = "Lavalink hazırlanıyor".into();
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(120) {
        if container_state(name).contains("(healthy)") {
            return true;
        }
        thread::sleep(Duration::from_secs(2));
    }
    log(s, format!("UYARI: {name} 2 dakikada hazır olmadı."));
    false
}

fn find_yt_code() -> Option<String> {
    let (_, out) = run("docker", &["logs", "--tail", "500", LAVALINK]);
    let line = out.lines().filter(|l| l.contains("google.com/device")).last()?.to_string();
    let lower = line.to_lowercase();
    if let Some(i) = lower.rfind("code") {
        let rest = line[i + 4..].trim_start_matches([' ', ':']);
        let code: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        if code.len() >= 6 {
            return Some(code);
        }
    }
    Some(line)
}

fn action_start(s: &S) {
    if !ensure_docker(s) {
        return;
    }
    ensure_henker(s);
    step(s, "Şifre çözücü başlatılıyor", "docker", &["start", CIPHER]);
    if step(s, "Lavalink ve bot başlatılıyor", "docker", &["compose", "--profile", "lavalink", "up", "-d"]) {
        wait_healthy(s, LAVALINK);
        log(s, "Başlatıldı.");
        if container_state(BOT).starts_with("running") {
            notify(s, "🟢 Blakfy FM aktif.");
        }
    }
}

fn action_stop(s: &S) {
    if !docker_ok() {
        log(s, "Docker zaten kapalı.");
        return;
    }
    if container_state(BOT).starts_with("running") {
        notify(s, "🔴 Blakfy FM kapandı.");
    }
    step(s, "Bot ve Lavalink durduruluyor", "docker", &["compose", "--profile", "lavalink", "stop"]);
    step(s, "Şifre çözücü durduruluyor", "docker", &["stop", CIPHER]);
    log(s, "Durduruldu.");
}

fn action_restart(s: &S) {
    if !ensure_docker(s) {
        return;
    }
    ensure_henker(s);
    step(s, "Şifre çözücü başlatılıyor", "docker", &["start", CIPHER]);
    step(s, "Radyo dönüştürücü başlatılıyor", "docker", &["start", PROXY]);
    if step(s, "Lavalink yeniden başlatılıyor", "docker", &["restart", LAVALINK]) {
        wait_healthy(s, LAVALINK);
        step(s, "Bot yeniden başlatılıyor", "docker", &["restart", BOT]);
        log(s, "Yeniden başlatıldı. YouTube girişi gerekebilir.");
    }
}

fn action_code(s: &S) {
    match find_yt_code() {
        Some(c) => {
            log(s, format!("YouTube kodu bulundu: {c}"));
            s.lock().unwrap().yt_code = Some(c);
        }
        None => log(s, "Log'da YouTube giriş kodu yok (giriş gerekmiyor olabilir)."),
    }
}

/// Strips ANSI color codes from a log line.
fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\u{1b}' {
            for n in it.by_ref() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn load_tab(s: &S, tab: Tab) {
    let text = match tab {
        Tab::Activity => return,
        Tab::Commands => {
            // Who ran which command: bot log lines tagged [CMD].
            let (_, out) = run("docker", &["logs", "--tail", "5000", BOT]);
            let lines: Vec<String> = out
                .lines()
                .map(strip_ansi)
                .filter(|l| l.contains("[CMD]"))
                .map(|l| {
                    // "[11/10/2026] [8:32:32 PM] [...] ... [CMD] user (id) -> !cmd args"
                    let time = l.split("] [").nth(1).unwrap_or("").to_string();
                    let cmd = l.split("[CMD]").nth(1).unwrap_or("").trim().to_string();
                    format!("{time:>11}   {cmd}")
                })
                .collect();
            if lines.is_empty() {
                "(henüz komut yok)".to_string()
            } else {
                lines[lines.len().saturating_sub(200)..].join("\n")
            }
        }
        Tab::BotLog | Tab::LavaLog => {
            let name = if tab == Tab::BotLog { BOT } else { LAVALINK };
            let (_, out) = run("docker", &["logs", "--tail", "200", name]);
            let out: Vec<String> = out.lines().map(strip_ansi).collect();
            if out.is_empty() { "(log boş)".to_string() } else { out.join("\n") }
        }
    };
    s.lock().unwrap().tab_text = text;
}

// ---------- UI widgets ----------

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(14))
        .show(ui, add)
        .inner
}

fn section_title(ui: &mut egui::Ui, text: &str) {
    // Titles are passed already upper-cased (Rust's to_uppercase has no Turkish İ).
    ui.label(RichText::new(text).size(11.0).color(MUTED).strong());
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

/// Animated equalizer logo; bars dance while the bot is on air.
fn equalizer(ui: &mut egui::Ui, time: f64, live: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(44.0, 34.0), Sense::hover());
    let p = ui.painter();
    let n = 5;
    let w = 6.0;
    let gap = (rect.width() - w * n as f32) / (n as f32 - 1.0);
    for i in 0..n {
        let phase = time * 3.2 + i as f64 * 1.3;
        let h = if live {
            let v = (phase.sin() * 0.5 + 0.5) * 0.7 + ((phase * 1.9).cos() * 0.5 + 0.5) * 0.3;
            6.0 + v as f32 * (rect.height() - 6.0)
        } else {
            [10.0, 16.0, 8.0, 14.0, 6.0][i]
        };
        let x = rect.left() + i as f32 * (w + gap);
        let r = egui::Rect::from_min_max(pos2(x, rect.bottom() - h), pos2(x + w, rect.bottom()));
        let c = if live { lerp_color(ACCENT, ACCENT2, i as f32 / (n - 1) as f32) } else { BORDER };
        p.rect_filled(r, CornerRadius::same(3), c);
    }
}

fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.6)))
        .corner_radius(CornerRadius::same(255))
        .inner_margin(Margin::symmetric(12, 5))
        .show(ui, |ui| {
            ui.label(RichText::new(format!("● {text}")).color(color).strong().size(12.5));
        });
}

/// Big round power button. Green ring = on air. Spinning arc while busy.
fn power_button(ui: &mut egui::Ui, time: f64, on: bool, busy: bool) -> egui::Response {
    let size = 104.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let p = ui.painter();
    let c = rect.center();
    let r = size / 2.0 - 4.0;
    let hover = resp.hovered() && !busy;
    let ring = if busy { ACCENT } else if on { GREEN } else { MUTED };

    // Glow.
    for k in 1..=4 {
        let a = if on || busy { 0.07 } else { 0.03 } / k as f32;
        p.circle_filled(c, r + k as f32 * 3.0, ring.gamma_multiply(a));
    }
    p.circle_filled(c, r, if hover { CARD_HI } else { CARD });
    p.circle_stroke(c, r, Stroke::new(3.0, ring.gamma_multiply(if busy { 0.25 } else { 1.0 })));

    if busy {
        // Spinning arc.
        let start = time * 4.0;
        let pts: Vec<egui::Pos2> = (0..=24)
            .map(|i| {
                let a = start + i as f64 / 24.0 * 1.6;
                pos2(c.x + r * a.cos() as f32, c.y + r * a.sin() as f32)
            })
            .collect();
        p.add(egui::Shape::line(pts, Stroke::new(4.0, ACCENT2)));
    }

    // Power icon: open arc + vertical line.
    let ir = r * 0.36;
    let icon = if busy { MUTED } else if on { GREEN } else { TEXT };
    let pts: Vec<egui::Pos2> = (0..=40)
        .map(|i| {
            // From -60° to 240° (gap at the top), y axis down.
            let deg = -60.0 + i as f32 / 40.0 * 300.0;
            let a = (deg - 90.0).to_radians();
            pos2(c.x + ir * a.cos(), c.y + ir * a.sin())
        })
        .collect();
    let mut pts = pts;
    pts.reverse();
    p.add(egui::Shape::line(pts, Stroke::new(4.0, icon)));
    p.line_segment([pos2(c.x, c.y - ir - 6.0), pos2(c.x, c.y - 2.0)], Stroke::new(4.0, icon));

    let label = if busy { "Çalışıyor" } else if on { "Durdur" } else { "Başlat" };
    p.text(pos2(c.x, c.y + ir + 14.0), Align2::CENTER_CENTER, label, FontId::proportional(11.5), MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Thin rounded bar, `frac` 0..1. Turns amber at 70% and red at 85%.
fn usage_bar(ui: &mut egui::Ui, frac: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 6.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(3), BORDER);
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        let fill = egui::Rect::from_min_size(rect.min, vec2((rect.width() * f).max(6.0), rect.height()));
        let c = if f > 0.85 { RED } else if f > 0.70 { AMBER } else { lerp_color(ACCENT, ACCENT2, f) };
        p.rect_filled(fill, CornerRadius::same(3), c);
    }
}

fn svc_card(ui: &mut egui::Ui, svc: &Svc, time: f64, width: f32) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, if svc.running { GREEN.gamma_multiply(0.35) } else { BORDER }))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(width - 26.0);
            // The card sits in a horizontal row; lay its content out vertically.
            ui.vertical(|ui| {
            ui.horizontal(|ui| {
                // Pulsing dot when running.
                let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                let col = if svc.running { GREEN } else { RED };
                if svc.running {
                    let pulse = ((time * 2.0).sin() * 0.5 + 0.5) as f32;
                    ui.painter().circle_filled(r.center(), 4.0 + pulse * 3.0, col.gamma_multiply(0.25));
                }
                ui.painter().circle_filled(r.center(), 4.0, col);
                ui.label(RichText::new(&svc.label).strong().color(TEXT).size(14.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(cpu) = svc.cpu {
                        ui.label(RichText::new(format!("CPU %{cpu:.1}")).size(11.0).color(MUTED));
                    }
                });
            });
            ui.label(RichText::new(&svc.text).size(11.5).color(MUTED));
            ui.add_space(4.0);
            match svc.mem {
                Some((used, limit)) if limit > 0.0 => {
                    let frac = (used / limit) as f32;
                    usage_bar(ui, frac, width - 26.0);
                    ui.label(
                        RichText::new(format!("{used:.0} MB / {limit:.0} MB  ·  %{:.0}", frac * 100.0))
                            .size(11.0)
                            .color(if frac > 0.85 { RED } else { MUTED }),
                    );
                }
                _ => {
                    usage_bar(ui, 0.0, width - 26.0);
                    ui.label(RichText::new("RAM: -").size(11.0).color(MUTED));
                }
            }
            });
        });
}

/// Small host card (Docker / HenkerDPI) without RAM.
fn host_chip(ui: &mut egui::Ui, label: &str, ok: bool, detail: &str) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("●").color(if ok { GREEN } else { RED }));
                ui.label(RichText::new(label).strong().color(TEXT));
                ui.label(RichText::new(detail).size(11.5).color(MUTED));
            });
        });
}

/// RAM history graph (area + line), newest sample on the right.
fn sparkline(ui: &mut egui::Ui, hist: &[f64], width: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(8), BG);
    if hist.len() < 2 {
        p.text(rect.center(), Align2::CENTER_CENTER, "veri toplanıyor...", FontId::proportional(11.0), MUTED);
        return;
    }
    let max = hist.iter().cloned().fold(1.0, f64::max) * 1.15;
    let n = hist.len();
    let step = rect.width() / (crate::HIST_LEN as f32 - 1.0);
    let x0 = rect.right() - step * (n as f32 - 1.0);
    let pts: Vec<egui::Pos2> = hist
        .iter()
        .enumerate()
        .map(|(i, v)| pos2(x0 + i as f32 * step, rect.bottom() - 4.0 - (*v / max) as f32 * (rect.height() - 10.0)))
        .collect();
    // Area: one thin column per sample, fading downwards.
    for w in pts.windows(2) {
        let col = egui::Rect::from_min_max(pos2(w[0].x, w[0].y.min(w[1].y)), pos2(w[1].x, rect.bottom()));
        p.rect_filled(col, CornerRadius::ZERO, ACCENT.gamma_multiply(0.12));
    }
    p.add(egui::Shape::line(pts.clone(), Stroke::new(2.0, ACCENT2)));
    if let Some(last) = pts.last() {
        p.circle_filled(*last, 3.5, ACCENT2);
    }
    p.text(
        pos2(rect.left() + 8.0, rect.top() + 6.0),
        Align2::LEFT_TOP,
        format!("tepe {:.0} MB", max / 1.15),
        FontId::proportional(10.5),
        MUTED,
    );
}

const HIST_LEN: usize = 90; // 90 samples x 4 s = 6 min

// ---------- app ----------

struct App {
    s: S,
    tab: Tab,
    tab_loaded: Option<Instant>,
}

impl App {
    /// Runs a start/stop/restart action; buttons are locked while it runs.
    fn spawn(&self, label: &str, f: fn(&S)) {
        let s = self.s.clone();
        {
            let mut g = s.lock().unwrap();
            if g.busy {
                return;
            }
            g.busy = true;
            g.busy_label = label.to_string();
        }
        thread::spawn(move || {
            f(&s);
            let st = read_status();
            let mut g = s.lock().unwrap();
            g.status = st;
            g.busy = false;
            g.busy_label.clear();
        });
    }

    /// Background job that never locks the buttons (logs, YouTube code).
    fn spawn_bg(&self, f: impl FnOnce(&S) + Send + 'static) {
        let s = self.s.clone();
        thread::spawn(move || f(&s));
    }

    fn select_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.tab_loaded = Some(Instant::now());
        if tab != Tab::Activity {
            self.s.lock().unwrap().tab_text = "yükleniyor...".into();
            self.spawn_bg(move |s| load_tab(s, tab));
        }
    }
}

fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = BG;
    v.faint_bg_color = CARD_HI;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.5);
    v.hyperlink_color = ACCENT2;
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
        w.corner_radius = CornerRadius::same(10);
    }
    v.widgets.inactive.weak_bg_fill = CARD_HI;
    v.widgets.inactive.bg_fill = CARD_HI;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(44, 40, 70);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.6);
    // Always dark, whatever the Windows theme is.
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.spacing.button_padding = vec2(12.0, 7.0);
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(13.5));
        s.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(12.0));
    });
}

fn action_button(ui: &mut egui::Ui, text: &str, fill: Color32) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(TEXT).strong())
            .fill(fill)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(CornerRadius::same(10))
            .min_size(vec2(0.0, 32.0)),
    )
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        let time = ctx.input(|i| i.time);
        let (st, busy, busy_label, logs, code, tab_text, hist) = {
            let g = self.s.lock().unwrap();
            (
                g.status.clone(),
                g.busy,
                g.busy_label.clone(),
                g.log.clone(),
                g.yt_code.clone(),
                g.tab_text.clone(),
                g.ram_hist.clone(),
            )
        };
        let live = st.bot_up() && st.docker;
        // Smooth animation while something moves; otherwise save CPU.
        ctx.request_repaint_after(Duration::from_millis(if live || busy { 50 } else { 500 }));

        // Auto-refresh the open log tab every 5 s.
        if self.tab != Tab::Activity && self.tab_loaded.map(|t| t.elapsed() > Duration::from_secs(5)).unwrap_or(true) {
            let tab = self.tab;
            self.tab_loaded = Some(Instant::now());
            self.spawn_bg(move |s| load_tab(s, tab));
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::same(18)))
            .show(root, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let full = ui.available_width();

                    // ---- header ----
                    ui.horizontal(|ui| {
                        equalizer(ui, time, live);
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("BLAKFY FM").size(24.0).strong().color(TEXT));
                            ui.label(RichText::new("Discord müzik botu · kontrol paneli").size(12.0).color(MUTED));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (t, c) = if !st.loaded {
                                ("YÜKLENİYOR", MUTED)
                            } else if busy {
                                ("İŞLEM SÜRÜYOR", ACCENT)
                            } else if !st.docker {
                                ("DOCKER KAPALI", RED)
                            } else if st.all_up() {
                                ("YAYINDA", GREEN)
                            } else if st.running_count() == 0 {
                                ("KAPALI", MUTED)
                            } else {
                                ("EKSİK ÇALIŞIYOR", AMBER)
                            };
                            pill(ui, t, c);
                        });
                    });
                    ui.add_space(12.0);

                    // ---- hero: power + schedule + actions ----
                    card(ui, |ui| {
                        ui.set_width(full - 30.0);
                        ui.horizontal(|ui| {
                            let resp = power_button(ui, time, st.bot_up(), busy);
                            if resp.clicked() && !busy {
                                if st.bot_up() {
                                    self.spawn("Durduruluyor", action_stop);
                                } else {
                                    self.spawn("Başlatılıyor", action_start);
                                }
                            }
                            ui.add_space(14.0);
                            ui.vertical(|ui| {
                                let headline = if busy {
                                    format!("{busy_label}...")
                                } else if st.all_up() {
                                    "Bot yayında, her şey çalışıyor.".to_string()
                                } else if st.bot_up() {
                                    "Bot açık, bazı parçalar kapalı.".to_string()
                                } else {
                                    "Bot kapalı.".to_string()
                                };
                                ui.label(RichText::new(headline).size(17.0).strong().color(TEXT));
                                let (sched_live, sched) = schedule_text();
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("◉").size(12.0).color(if sched_live { ACCENT2 } else { MUTED }));
                                    ui.label(
                                        RichText::new(format!("Kafa Radyo zamanlayıcı · {sched}"))
                                            .size(12.5)
                                            .color(if sched_live { ACCENT2 } else { MUTED }),
                                    );
                                });
                                ui.add_space(6.0);
                                ui.add_enabled_ui(!busy, |ui| {
                                    ui.horizontal(|ui| {
                                        if action_button(ui, "⟳  Yeniden başlat", CARD_HI).clicked() {
                                            self.spawn("Yeniden başlatılıyor", action_restart);
                                        }
                                        if action_button(ui, "✕  Paneli kapat", CARD_HI).clicked() {
                                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                        }
                                    });
                                });
                            });
                        });
                    });
                    ui.add_space(12.0);

                    // ---- host chips ----
                    ui.horizontal(|ui| {
                        host_chip(ui, "Docker", st.docker, if st.docker { "açık" } else { "kapalı" });
                        host_chip(ui, "HenkerDPI", st.henker, if st.henker { "açık" } else { "kapalı" });
                    });
                    ui.add_space(8.0);

                    // ---- service cards, 2 columns ----
                    section_title(ui, "SERVİSLER");
                    let col_w = (full - 8.0) / 2.0;
                    for pair in st.svcs.chunks(2) {
                        ui.horizontal(|ui| {
                            for svc in pair {
                                svc_card(ui, svc, time, col_w);
                            }
                        });
                    }
                    ui.add_space(8.0);

                    // ---- RAM ----
                    card(ui, |ui| {
                        ui.set_width(full - 30.0);
                        ui.horizontal(|ui| {
                            section_title(ui, "TOPLAM RAM");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new("son 6 dakika").size(11.0).color(MUTED));
                            });
                        });
                        let total = st.total_mem();
                        let limit: f64 = st.svcs.iter().filter_map(|s| s.mem.map(|m| m.1)).sum();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{total:.0} MB")).size(26.0).strong().color(TEXT));
                            if limit > 0.0 {
                                ui.label(RichText::new(format!("/ {limit:.0} MB sınır")).size(13.0).color(MUTED));
                            }
                        });
                        sparkline(ui, &hist, full - 30.0, 70.0);
                    });
                    ui.add_space(8.0);

                    // ---- YouTube login ----
                    card(ui, |ui| {
                        ui.set_width(full - 30.0);
                        section_title(ui, "YOUTUBE GİRİŞİ");
                        ui.horizontal(|ui| {
                            if action_button(ui, "Kodu bul", CARD_HI).clicked() {
                                self.spawn_bg(action_code);
                            }
                            if action_button(ui, "google.com/device aç", CARD_HI).clicked() {
                                let _ = Command::new("cmd")
                                    .args(["/C", "start", "", "https://www.google.com/device"])
                                    .creation_flags(NO_WINDOW)
                                    .spawn();
                            }
                            if let Some(c) = &code {
                                egui::Frame::new()
                                    .fill(BG)
                                    .stroke(Stroke::new(1.0, ACCENT))
                                    .corner_radius(CornerRadius::same(8))
                                    .inner_margin(Margin::symmetric(10, 5))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(c).monospace().size(15.0).strong().color(ACCENT2));
                                    });
                                if action_button(ui, "Kopyala", ACCENT.gamma_multiply(0.5)).clicked() {
                                    ctx.copy_text(c.clone());
                                }
                            }
                        });
                        ui.label(
                            RichText::new("Yedek (premium) hesapla gir, ana hesapla girme. Lavalink her yeniden başladığında gerekir.")
                                .size(11.0)
                                .color(MUTED),
                        );
                    });
                    ui.add_space(8.0);

                    // ---- tabs ----
                    card(ui, |ui| {
                        ui.set_width(full - 30.0);
                        ui.horizontal(|ui| {
                            for (t, name) in [
                                (Tab::Activity, "Etkinlik"),
                                (Tab::Commands, "Komutlar"),
                                (Tab::BotLog, "Bot logu"),
                                (Tab::LavaLog, "Lavalink logu"),
                            ] {
                                let sel = self.tab == t;
                                let b = egui::Button::new(
                                    RichText::new(name).color(if sel { TEXT } else { MUTED }).strong(),
                                )
                                .fill(if sel { ACCENT.gamma_multiply(0.45) } else { Color32::TRANSPARENT })
                                .stroke(Stroke::new(1.0, if sel { ACCENT } else { BORDER }))
                                .corner_radius(CornerRadius::same(255));
                                if ui.add(b).clicked() && !sel {
                                    self.select_tab(t);
                                }
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let text = if self.tab == Tab::Activity { logs.join("\n") } else { tab_text.clone() };
                                if ui.small_button("Kopyala").clicked() {
                                    ctx.copy_text(text);
                                }
                                if self.tab != Tab::Activity {
                                    ui.label(RichText::new("5 sn'de bir yenilenir").size(10.5).color(MUTED));
                                }
                            });
                        });
                        ui.add_space(6.0);
                        egui::Frame::new()
                            .fill(BG)
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(10))
                            .show(ui, |ui| {
                                ui.set_width(full - 50.0);
                                egui::ScrollArea::both()
                                    .id_salt(("tab", self.tab as u8))
                                    .max_height(260.0)
                                    .min_scrolled_height(260.0)
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        if self.tab == Tab::Activity {
                                            if logs.is_empty() {
                                                ui.label(RichText::new("Henüz işlem yok.").color(MUTED));
                                            }
                                            for l in &logs {
                                                let c = if l.contains("HATA") || l.contains("gönderilemedi") {
                                                    RED
                                                } else if l.contains("UYARI") {
                                                    AMBER
                                                } else if l.contains("Başlatıldı") || l.contains("yazıldı") || l.contains("hazır") {
                                                    GREEN
                                                } else {
                                                    TEXT
                                                };
                                                ui.label(RichText::new(l).monospace().color(c));
                                            }
                                        } else {
                                            ui.add(
                                                egui::Label::new(RichText::new(&tab_text).monospace().size(11.5))
                                                    .extend(),
                                            );
                                        }
                                    });
                            });
                    });
                });
            });
    }
}

fn main() -> eframe::Result<()> {
    let s: S = Arc::new(Mutex::new(Shared::default()));
    {
        // Status poller: every 4 s, also feeds the RAM graph.
        let s = s.clone();
        thread::spawn(move || loop {
            let st = read_status();
            let total = st.total_mem();
            {
                let mut g = s.lock().unwrap();
                g.ram_hist.push(total);
                let n = g.ram_hist.len();
                if n > HIST_LEN {
                    g.ram_hist.drain(0..n - HIST_LEN);
                }
                g.status = st;
            }
            thread::sleep(Duration::from_secs(4));
        });
    }
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Blakfy FM")
            .with_inner_size([720.0, 900.0])
            .with_min_inner_size([640.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Blakfy FM",
        opts,
        Box::new(|cc| {
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(App { s, tab: Tab::Activity, tab_loaded: None }))
        }),
    )
}
