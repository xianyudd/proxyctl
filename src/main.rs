use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
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
        Cmd::Status => match mode {
            Mode::FishUvars => fish_print_status(),
            Mode::Process => status_in_process(),
            Mode::Auto => unreachable!(),
        },
    }
}
