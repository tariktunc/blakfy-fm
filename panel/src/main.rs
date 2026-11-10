#![windows_subsystem = "windows"]
//! Blakfy FM panel: start / stop / restart the Discord music bot stack.

use eframe::egui;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const BOT_DIR: &str = r"C:\Users\Tarik\Desktop\github\discord-muzik-botu";
const HENKER_EXE: &str = r"C:\Users\Tarik\Downloads\HenkerDPI-v3.0.5\HenkerDPI.exe";
const DOCKER_DESKTOP: &str = r"C:\Program Files\Docker\Docker\Docker Desktop.exe";
const CIPHER: &str = "yt-cipher";
const LAVALINK: &str = "lavamusic-lavalink";
const BOT: &str = "lavamusic";
const NO_WINDOW: u32 = 0x0800_0000;
/// #sohbet on the Blakfy server. Override with BLAKFY_FM_CHANNEL_ID in .env.
const SOHBET_CHANNEL: &str = "982026261025157210";

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
                    let rest = &v[1..];
                    rest.split(q).next().unwrap_or("").to_string()
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

/// Runs a PowerShell script with the bot token passed via env var (not argv).
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

/// Sends a message to Discord (#sohbet unless BLAKFY_FM_CHANNEL_ID is set).
fn notify(s: &S, text: &str) {
    let Some(token) = env_value("TOKEN") else {
        log(s, "Discord mesajı: .env içinde TOKEN yok.");
        return;
    };
    let channel = env_value("BLAKFY_FM_CHANNEL_ID").unwrap_or_else(|| SOHBET_CHANNEL.to_string());
    let body = format!("{{\"content\":\"{}\"}}", text.replace('\\', "\\\\").replace('"', "\\\""));
    let script = "$ErrorActionPreference='Stop'; [Net.ServicePointManager]::SecurityProtocol='Tls12'; Invoke-RestMethod -UserAgent 'DiscordBot (https://blakfy.com, 1.0)' -Method Post -Uri ('https://discord.com/api/v10/channels/'+$env:BFM_CH+'/messages') -Headers @{Authorization='Bot '+$env:BFM_TOKEN} -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes($env:BFM_BODY)) | Out-Null";
    let (ok, out) = ps_env(script, &[("BFM_TOKEN", &token), ("BFM_CH", &channel), ("BFM_BODY", &body)]);
    if ok {
        log(s, format!("Discord'a yazıldı: {text}"));
    } else {
        log(s, format!("Discord mesajı gönderilemedi: {out}"));
    }
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

#[derive(Clone, Default)]
struct Status {
    docker: bool,
    henker: bool,
    cipher: String,
    lavalink: String,
    bot: String,
}

fn read_status() -> Status {
    let docker = docker_ok();
    let (cipher, lavalink, bot) = if docker {
        (container_state(CIPHER), container_state(LAVALINK), container_state(BOT))
    } else {
        ("-".into(), "-".into(), "-".into())
    };
    Status { docker, henker: henker_running(), cipher, lavalink, bot }
}

#[derive(Default)]
struct Shared {
    status: Status,
    log: Vec<String>,
    busy: bool,
    yt_code: Option<String>,
    docker_log: Option<(String, String)>,
}

type S = Arc<Mutex<Shared>>;

fn log(s: &S, msg: impl Into<String>) {
    let mut g = s.lock().unwrap();
    g.log.push(msg.into());
    let n = g.log.len();
    if n > 200 {
        g.log.drain(0..n - 200);
    }
}

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
    let (ok, out) = run(prog, args);
    if !ok {
        log(s, format!("HATA ({label}): {out}"));
    }
    ok
}

fn wait_healthy(s: &S, name: &str) -> bool {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(120) {
        let st = container_state(name);
        if st.contains("(healthy)") {
            return true;
        }
        thread::sleep(Duration::from_secs(2));
    }
    log(s, format!("UYARI: {name} 2 dakikada hazır olmadı."));
    false
}

fn find_yt_code() -> Option<String> {
    let (_, out) = run("docker", &["logs", "--tail", "500", LAVALINK]);
    let line = out
        .lines()
        .filter(|l| l.contains("google.com/device"))
        .last()?
        .to_string();
    let lower = line.to_lowercase();
    if let Some(i) = lower.rfind("code") {
        let rest = line[i + 4..].trim_start_matches([' ', ':']);
        let code: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
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
    if step(s, "Lavalink yeniden başlatılıyor", "docker", &["restart", LAVALINK]) {
        wait_healthy(s, LAVALINK);
        step(s, "Bot yeniden başlatılıyor", "docker", &["restart", BOT]);
        log(s, "Yeniden başlatıldı. YouTube girişi gerekebilir.");
    }
}

fn action_code(s: &S) {
    match find_yt_code() {
        Some(c) => {
            log(s, format!("YouTube kodu: {c}"));
            s.lock().unwrap().yt_code = Some(c);
        }
        None => log(s, "Log'da YouTube giriş kodu yok (giriş gerekmiyor olabilir)."),
    }
}

fn show_docker_log(s: &S, name: &str, title: &str) {
    let (_, out) = run("docker", &["logs", "--tail", "150", name]);
    let out = if out.is_empty() { "(log boş)".to_string() } else { out };
    s.lock().unwrap().docker_log = Some((title.to_string(), out));
}

fn action_bot_log(s: &S) {
    show_docker_log(s, BOT, "Bot logu");
}

fn action_lava_log(s: &S) {
    show_docker_log(s, LAVALINK, "Lavalink logu");
}

/// Strips ANSI color codes from a log line.
fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\u{1b}' {
            while let Some(n) = it.next() {
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

/// "Komutlar" tab: who ran which command (bot logs lines tagged [CMD]).
fn action_cmd_log(s: &S) {
    let (_, out) = run("docker", &["logs", "--tail", "5000", BOT]);
    let lines: Vec<String> = out
        .lines()
        .map(strip_ansi)
        .filter(|l| l.contains("[CMD]"))
        .map(|l| {
            // "[11/10/2026] [8:32:32 PM] [...] ... [CMD] user (id) -> !cmd args"
            let time: String = l.split("] [").nth(1).unwrap_or("").to_string();
            let cmd = l.split("[CMD]").nth(1).unwrap_or("").trim().to_string();
            format!("{time}  {cmd}")
        })
        .collect();
    let start = lines.len().saturating_sub(200);
    let text = if lines.is_empty() {
        "(henüz komut yok)".to_string()
    } else {
        lines[start..].join("\n")
    };
    s.lock().unwrap().docker_log = Some(("Komutlar".to_string(), text));
}

struct App {
    s: S,
}

impl App {
    fn spawn(&self, f: fn(&S)) {
        let s = self.s.clone();
        {
            let mut g = s.lock().unwrap();
            if g.busy {
                return;
            }
            g.busy = true;
        }
        thread::spawn(move || {
            f(&s);
            let st = read_status();
            let mut g = s.lock().unwrap();
            g.status = st;
            g.busy = false;
        });
    }
}

fn dot(ui: &mut egui::Ui, label: &str, ok: bool, text: &str) {
    ui.horizontal(|ui| {
        let c = if ok { egui::Color32::from_rgb(40, 200, 90) } else { egui::Color32::from_rgb(220, 60, 60) };
        ui.colored_label(c, "●");
        ui.label(format!("{label}: {text}"));
    });
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        ctx.request_repaint_after(Duration::from_millis(500));
        let (st, busy, logs, code, dlog) = {
            let g = self.s.lock().unwrap();
            (g.status.clone(), g.busy, g.log.clone(), g.yt_code.clone(), g.docker_log.clone())
        };

        egui::CentralPanel::default().show(root, |ui| {
            ui.heading("Blakfy FM");
            ui.add_space(6.0);
            dot(ui, "Docker", st.docker, if st.docker { "açık" } else { "kapalı" });
            dot(ui, "HenkerDPI", st.henker, if st.henker { "açık" } else { "kapalı" });
            dot(ui, "Şifre çözücü", st.cipher.starts_with("running"), &st.cipher);
            dot(ui, "Lavalink", st.lavalink.starts_with("running"), &st.lavalink);
            dot(ui, "Bot", st.bot.starts_with("running"), &st.bot);
            ui.add_space(8.0);

            ui.add_enabled_ui(!busy, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("▶ Başlat").clicked() {
                        self.spawn(action_start);
                    }
                    if ui.button("■ Durdur").clicked() {
                        self.spawn(action_stop);
                    }
                    if ui.button("⟳ Yeniden başlat").clicked() {
                        self.spawn(action_restart);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("YouTube kodu").clicked() {
                        self.spawn(action_code);
                    }
                    if ui.button("google.com/device aç").clicked() {
                        let _ = Command::new("cmd")
                            .args(["/C", "start", "", "https://www.google.com/device"])
                            .creation_flags(NO_WINDOW)
                            .spawn();
                    }
                    if ui.button("✕ Kapat").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Komutlar").clicked() {
                        self.spawn(action_cmd_log);
                    }
                    if ui.button("Bot logu").clicked() {
                        self.spawn(action_bot_log);
                    }
                    if ui.button("Lavalink logu").clicked() {
                        self.spawn(action_lava_log);
                    }
                    if dlog.is_some() && ui.button("Logu gizle").clicked() {
                        self.s.lock().unwrap().docker_log = None;
                    }
                });
            });
            if busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Çalışıyor...");
                });
            }
            if let Some(c) = code {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Kod:");
                    ui.monospace(&c);
                    if ui.button("Kopyala").clicked() {
                        ctx.copy_text(c.clone());
                    }
                });
                ui.small("Yedek (premium) hesapla gir. Ana hesapla girme.");
            }
            if let Some((title, text)) = &dlog {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong(title);
                    if ui.button("Kopyala").clicked() {
                        ctx.copy_text(text.clone());
                    }
                });
                egui::ScrollArea::both()
                    .id_salt("dlog")
                    .max_height(260.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(egui::RichText::new(text).monospace().size(11.0)).extend());
                    });
            }
            ui.separator();
            egui::ScrollArea::vertical().id_salt("plog").stick_to_bottom(true).show(ui, |ui| {
                for l in &logs {
                    ui.label(l);
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let s: S = Arc::new(Mutex::new(Shared::default()));
    {
        let s = s.clone();
        thread::spawn(move || loop {
            let st = read_status();
            s.lock().unwrap().status = st;
            thread::sleep(Duration::from_secs(4));
        });
    }
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Blakfy FM")
            .with_inner_size([640.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native("Blakfy FM", opts, Box::new(|_cc| Ok(Box::new(App { s }))))
}
