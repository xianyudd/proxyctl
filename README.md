# proxyctl

`proxyctl` 是一个用于 WSL2 环境下管理代理设置的小工具。
它可以自动探测宿主机的网关 IP，或者通过用户手动指定，快速切换代理的启用与关闭。
同时支持 fish shell 的 universal variables，也可以仅在当前进程中生效。

---

## ✨ 特性

* 🚀 一键切换：`proxyctl on` / `proxyctl off`
* 🔍 自动探测宿主机网关 IP（通过 `ip route show`）
* ⚙️ 支持配置文件：`~/.proxyctl.toml`，可定义端口和模式
* 🐟 fish shell 集成：使用 universal variables，使代理在整个 fish 会话中生效
* 🔧 灵活模式：

  * `mixed`：HTTP 与 SOCKS 共用同一端口（默认 7890）
  * `split`：HTTP 用 7892，SOCKS 用 7891
* 📦 内置测试与 CI：带 smoke 测试和 GitHub Actions 检查
* 🌐 内置连通性测试：快速验证代理是否能访问常见站点（Google、GitHub、YouTube、HuggingFace、Cloudflare）

---

## 📖 当前支持的命令

```bash
proxyctl on [--ip <IP>]        # 开启代理，可自动探测 IP 或手动指定
proxyctl off                   # 关闭代理
proxyctl status                # 查看代理状态
proxyctl test [--ip <IP>] [--timeout <秒>] [--dry-run]
                               # 测试代理连通性（HTTP / SOCKS5），支持指定 IP 与超时
```

### 全局选项

* `--mode <auto|process|fish-uvars>`

  * `auto`：默认模式，fish 下用 uvars，其它情况只设置进程变量
  * `process`：仅设置当前进程的环境变量
  * `fish-uvars`：强制使用 fish universal variables
* `--profile <mixed|split>`

  * `mixed`：HTTP 与 SOCKS 共用同一端口（默认 7890）
  * `split`：HTTP 用 7892，SOCKS 用 7891

---

## 📦 安装

### 构建

```bash
cargo build --release
cp target/release/proxyctl ~/.local/bin/
```

### 添加到 PATH

在 `~/.bashrc` 或 `~/.config/fish/config.fish` 里添加：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

---

## 💻 支持平台

* **Linux**（已在 WSL2 环境下测试，推荐场景）
* **WSL2 (Windows Subsystem for Linux 2)**
* **macOS**（理论可用，但 IP 探测命令可能需要适配）

> ⚠️ Windows 原生命令行（cmd/PowerShell）不在支持范围内，建议通过 WSL2 使用。

---

## ⚡ 用法示例

```bash
# 自动探测 IP 开启代理
proxyctl on

# 关闭代理
proxyctl off

# 查看当前代理状态
proxyctl status

# 手动指定 IP
proxyctl --ip 172.17.208.1 on

# 使用 split profile
proxyctl --profile split on

# 测试连通性，超时设为 8 秒
proxyctl test --timeout 8
```

---

## ⚙️ 配置文件

默认会在第一次运行时生成 `~/.proxyctl.toml`：

```toml
[proxy]
# host_ip    = "172.17.208.1"   # 留空=自动探测；想固定可取消注释
profile    = "mixed"            # mixed: 都用混合端口；split: http=7892, socks=7891
mixed_port = 7890               # Clash/Mihomo 混合端口
socks_port = 7891               # Socks5 端口
http_port  = 7892               # HTTP 端口
```

---

## 🧪 开发与测试

运行完整检查脚本：

```bash
# 快速模式（提交前用，fmt + 编译测试）
python scripts/check.py --fast

# 全量模式（clippy + 测试）
python scripts/check.py
```

运行测试：

```bash
cargo test
```

---

## 📜 许可证

本项目基于 [MIT License](./LICENSE)发布，欢迎自由使用和修改。

---