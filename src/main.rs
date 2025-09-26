use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use std::time::Duration; // 新增：Duration & 计时
use std::{fs, process::Command};

/// ====================== CLI ======================
#[derive(Parser)]
#[command(name = "proxyctl", version = "0.6")]
struct Cli {
    /// 生效模式（auto: fish 用 UVAR，其它进程内）
    #[arg(long, value_enum, default_value = "auto")]
    mode: Mode,

    /// 临时覆盖端口模式（mixed=都用7890；split=http用7892，socks用7891）
    #[arg(long, value_enum)]
    profile: Option<Profile>,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Clone, Copy, ValueEnum, Debug)]
enum Mode {
    Auto,
    Process,
    FishUvars,
}

#[derive(Clone, Copy, ValueEnum, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Profile {
    Mixed,
    Split,
}

#[derive(Subcommand)]
enum Cmd {
    /// 开启代理
    On {
        #[arg(long)]
        ip: Option<String>,
    },
    /// 关闭代理
    Off,
    /// 查看状态
    Status,

    /// 测试代理连通性
    Test {
        /// 覆盖宿主机ip
        #[arg(long)]
        ip: Option<String>,

        /// 超时时间
        #[arg(long, default_value_t = 5)]
        timeout: u64,

        /// 仅打印将要测试的代理与站点，不实际发请求（用于测试/CI）
        #[arg(long, hide = true)]
        dry_run: bool,
    },
}

/// ====================== 配置 ======================
#[derive(Debug, Default, Deserialize)]
struct Config {
    #[serde(default)]
    proxy: ProxySection,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ProxySection {
    host_ip: Option<String>,
    profile: Profile, // mixed / split
    mixed_port: u16,  // 7890
    socks_port: u16,  // 7891
    http_port: u16,   // 7892
}
impl Default for ProxySection {
    fn default() -> Self {
        Self {
            host_ip: None,
            profile: Profile::Mixed,
            mixed_port: 7890,
            socks_port: 7891,
            http_port: 7892,
        }
    }
}

fn ensure_default_config() {
    let path = dirs::home_dir().unwrap().join(".proxyctl.toml");
    if !path.exists() {
        let tpl = r#"
# proxyctl 配置（首次运行自动生成）
[proxy]
# host_ip    = "172.17.208.1"   # 留空=自动探测；想固定可取消注释
profile    = "mixed"            # mixed: 都用混合端口；split: http=7892, socks=7891
mixed_port = 7890               # Clash/Mihomo 混合端口
socks_port = 7891               # Socks5 端口
http_port  = 7892               # HTTP 端口
"#;
        let _ = fs::write(&path, tpl);
        println!("📝 已生成默认配置：{}", path.display());
    }
}

fn load_config() -> Config {
    ensure_default_config();
    let path = dirs::home_dir().unwrap().join(".proxyctl.toml");
    fs::read_to_string(path)
        .ok()
        .and_then(|txt| toml::from_str(&txt).ok())
        .unwrap_or_default()
}

/// ====================== 代理连通性测试 ======================

// 经 HTTP 代理的 blocking 客户端
fn build_client_via_http(
    ip: &str,
    port: u16,
    timeout: Duration,
) -> Option<reqwest::blocking::Client> {
    let proxy = format!("http://{}:{}", ip, port);
    reqwest::blocking::Client::builder()
        .proxy(reqwest::Proxy::all(&proxy).ok()?)
        .timeout(timeout)
        .user_agent("proxyctl/0.6")
        .build()
        .ok()
}

// 经 SOCKS5(h) 代理的 blocking 客户端（h：让代理解析域名）
fn build_client_via_socks(
    ip: &str,
    port: u16,
    timeout: Duration,
) -> Option<reqwest::blocking::Client> {
    let proxy = format!("socks5h://{}:{}", ip, port);
    reqwest::blocking::Client::builder()
        .proxy(reqwest::Proxy::all(&proxy).ok()?)
        .timeout(timeout)
        .user_agent("proxyctl/0.6")
        .build()
        .ok()
}

// 修改：新增 dry_run 参数；为 true 时不发请求，只打印将要测试的站点
fn test_sites_via(label: &str, client: &reqwest::blocking::Client, sites: &[&str], dry_run: bool) {
    println!("🔎 Testing via {label} …");

    if dry_run {
        for &url in sites {
            println!("  [DRY] ----  ----  {}", url);
        }
        println!();
        return;
    }

    use std::io;
    for &url in sites {
        let t0 = std::time::Instant::now();
        let resp = client
            .get(url)
            .header("Accept", "text/html,*/*;q=0.8")
            .send();

        match resp {
            Ok(mut r) => {
                let status = r.status();
                // 丢进“黑洞”，读完但不分配
                let _ = io::copy(&mut r, &mut io::sink());
                let ms = t0.elapsed().as_millis();
                println!("  [OK ] {:>3}  {:>4}ms  {}", status.as_u16(), ms, url);
            }
            Err(e) => {
                let ms = t0.elapsed().as_millis();
                println!("  [ERR] ----  {:>4}ms  {}  ({})", ms, url, e);
            }
        }
    }
    println!();
}

// 汇总：对 HTTP 和 SOCKS5 分别测一遍
fn test_proxy(ip: &str, p: &EffPorts, timeout_secs: u64, dry_run: bool) {
    let timeout = std::time::Duration::from_secs(timeout_secs);

    // 测试站点
    let sites = [
        "https://www.google.com/generate_204",
        "https://www.github.com/",
        "https://www.youtube.com/robots.txt",
        "https://huggingface.co/",
        "https://www.cloudflare.com/cdn-cgi/trace",
    ];

    // 测试 HTTP 代理
    if let Some(c) = build_client_via_http(ip, p.http, timeout) {
        println!("➡️  HTTP proxy:  http://{}:{}", ip, p.http);
        test_sites_via("HTTP proxy", &c, &sites, dry_run);
    } else {
        println!("⚠️ 无法构建 HTTP 代理客户端（http://{}:{}）。", ip, p.http);
    }

    // 测试 SOCKS5 代理
    if let Some(c) = build_client_via_socks(ip, p.socks, timeout) {
        println!("➡️  SOCKS5 proxy: socks5h://{}:{}", ip, p.socks);
        test_sites_via("SOCKS5 proxy", &c, &sites, dry_run);
    } else {
        println!(
            "⚠️ 无法构建 SOCKS5 代理客户端（socks5h://{}:{}）。",
            ip, p.socks
        );
    }
}

/// ====================== IP 选择 ======================
fn auto_detect_ip() -> Option<String> {
    let out = Command::new("sh")
        .arg("-c")
        .arg("ip route show | grep default | awk '{print $3}'")
        .output()
        .ok()?;
    let ip = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if ip.is_empty() { None } else { Some(ip) }
}
fn choose_ip(cli_ip: Option<String>, cfg_ip: Option<String>) -> Option<String> {
    cli_ip.or(cfg_ip).or_else(auto_detect_ip)
}

/// 根据 profile 计算实际端口
struct EffPorts {
    http: u16,
    socks: u16,
}
fn effective_ports(cfg: &ProxySection, override_profile: Option<Profile>) -> EffPorts {
    let p = override_profile.unwrap_or(cfg.profile);
    match p {
        Profile::Mixed => EffPorts {
            http: cfg.mixed_port,
            socks: cfg.mixed_port,
        },
        Profile::Split => EffPorts {
            http: cfg.http_port,
            socks: cfg.socks_port,
        },
    }
}

/// ====================== 进程内设置（非 fish 回退） ======================
fn safe_set_var(key: &str, val: &str) {
    debug_assert!(!key.contains('\0') && !val.contains('\0'));
    unsafe { std::env::set_var(key, val) };
}
fn safe_remove_var(key: &str) {
    debug_assert!(!key.contains('\0'));
    unsafe { std::env::remove_var(key) };
}
fn set_proxy_in_process(ip: &str, p: &EffPorts) {
    let http = format!("http://{}:{}", ip, p.http);
    let socks = format!("socks5h://{}:{}", ip, p.socks);
    safe_set_var("http_proxy", &http);
    safe_set_var("https_proxy", &http);
    safe_set_var("all_proxy", &socks);
    println!(
        "✅ proxy ON (process) → {}  (http {}, socks {})",
        ip, p.http, p.socks
    );
}
fn unset_proxy_in_process() {
    for k in ["http_proxy", "https_proxy", "all_proxy"] {
        safe_remove_var(k);
    }
    println!("❌ proxy OFF (process)");
}
fn status_in_process() {
    for k in ["http_proxy", "https_proxy", "all_proxy"] {
        println!(
            "{:<12} = {}",
            k,
            std::env::var(k).unwrap_or_else(|_| "Not Set".into())
        );
    }
}

/// ====================== fish universal variables（无需 eval） ======================
fn is_fish() -> bool {
    std::env::var("SHELL").unwrap_or_default().contains("fish")
}
fn fish_set_uvars(ip: &str, p: &EffPorts) -> bool {
    let http = format!("http://{}:{}", ip, p.http);
    let socks = format!("socks5h://{}:{}", ip, p.socks);
    let cmd = format!(
        "set -Ux http_proxy '{}' ; set -Ux https_proxy '{}' ; set -Ux all_proxy '{}'",
        http, http, socks
    );
    Command::new("fish")
        .arg("-c")
        .arg(cmd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
fn fish_unset_uvars() -> bool {
    let cmd = "set -eU http_proxy ; set -eU https_proxy ; set -eU all_proxy";
    Command::new("fish")
        .arg("-c")
        .arg(cmd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
fn fish_print_status() {
    let cmd = r#"
        if set -q http_proxy;  printf "http_proxy  = %s\n" $http_proxy;  else; echo "http_proxy  = Not Set";  end;
        if set -q https_proxy; printf "https_proxy = %s\n" $https_proxy; else; echo "https_proxy = Not Set"; end;
        if set -q all_proxy;   printf "all_proxy   = %s\n" $all_proxy;   else; echo "all_proxy   = Not Set";   end;
    "#;
    if let Ok(out) = Command::new("fish").arg("-c").arg(cmd).output() {
        print!("{}", String::from_utf8_lossy(&out.stdout));
    }
}

/// ====================== 模式选择 ======================
fn resolve_mode(cli_mode: Mode) -> Mode {
    match cli_mode {
        Mode::Auto => {
            if is_fish() {
                Mode::FishUvars
            } else {
                Mode::Process
            }
        }
        other => other,
    }
}

/// ====================== main ======================
fn main() {
    let cli = Cli::parse();
    let cfg = load_config();
    let mode = resolve_mode(cli.mode);
    let ports = effective_ports(&cfg.proxy, cli.profile);

    match cli.command {
        Cmd::On { ip } => {
            let chosen = choose_ip(ip, cfg.proxy.host_ip.clone());
            match chosen {
                Some(ip_use) => match mode {
                    Mode::FishUvars => {
                        if fish_set_uvars(&ip_use, &ports) {
                            println!(
                                "✅ proxy ON (fish uvars) → {}  (http {}, socks {})",
                                ip_use, ports.http, ports.socks
                            );
                        } else {
                            eprintln!("⚠️ 设置 fish uvars 失败；回退进程内变量。");
                            set_proxy_in_process(&ip_use, &ports);
                        }
                    }
                    Mode::Process => set_proxy_in_process(&ip_use, &ports),
                    Mode::Auto => unreachable!(),
                },
                None => eprintln!(
                    "⚠️ 无法确定宿主机 IP。请用 --ip 指定或在 ~/.proxyctl.toml 的 [proxy].host_ip 中设置。"
                ),
            }
        }
        Cmd::Off => {
            match mode {
                Mode::FishUvars => {
                    // 若本就未设置，不报错
                    if fish_unset_uvars() {
                        println!("❌ proxy OFF (fish uvars)");
                    } else {
                        println!("ℹ️ fish uvars 未设置；已确保进程内变量清理。");
                        unset_proxy_in_process();
                    }
                }
                Mode::Process => unset_proxy_in_process(),
                Mode::Auto => unreachable!(),
            }
        }
        Cmd::Test {
            ip,
            timeout,
            dry_run,
        } => {
            let chosen = choose_ip(ip, cfg.proxy.host_ip.clone());
            match chosen {
                Some(ip_use) => {
                    println!(
                        "✅ Using IP: {}  (http {}, socks {})",
                        ip_use, ports.http, ports.socks
                    );
                    test_proxy(&ip_use, &ports, timeout, dry_run);
                }
                None => eprintln!(
                    "⚠️ 无法确定宿主机 IP。请用 --ip 指定或在 ~/.proxyctl.toml 的 [proxy].host_ip 中设置。"
                ),
            }
        }

        Cmd::Status => match mode {
            Mode::FishUvars => fish_print_status(),
            Mode::Process => status_in_process(),
            Mode::Auto => unreachable!(),
        },
    }
}
