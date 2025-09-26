# proxyctl

`proxyctl` 是一个用于 **WSL2 环境** 下管理代理设置的小工具。  
它可以自动探测宿主机的网关 IP，或者通过用户手动指定，快速切换代理的启用与关闭。  
同时支持 **fish shell 的 universal variables**，也可以仅在当前进程中生效。

---

## ✨ 特性

- 🚀 **一键切换**：`proxyctl on` / `proxyctl off`
- 🔍 **自动探测网关 IP**（通过 `ip route show`）
- ⚙️ **支持配置文件**：`~/.proxyctl.toml`，可定义端口和模式
- 🐟 **fish shell 集成**：使用 UVAR，使代理在整个 fish 会话中生效
- 🔧 **灵活模式**：
  - `mixed`：HTTP 与 SOCKS 共用同一端口（默认 7890）
  - `split`：HTTP 用 7892，SOCKS 用 7891
- 📦 **内置测试与 CI**：带 smoke 测试和 GitHub Actions 检查

---

## 📦 安装

```bash
# 构建 release
cargo build --release

# 将二进制加入 PATH
cp target/release/proxyctl ~/.local/bin/
````

或者在 `~/.bashrc` / `~/.config/fish/config.fish` 中添加：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

---

## ⚡ 用法

```bash
# 开启代理（自动探测 IP）
proxyctl on

# 关闭代理
proxyctl off

# 查看状态
proxyctl status

# 指定 profile 模式（split/mixed）
proxyctl --profile split on

# 手动指定 IP
proxyctl --ip 172.17.208.1 on
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

本项目基于 [MIT License](./LICENSE) 发布，欢迎自由使用和修改。




