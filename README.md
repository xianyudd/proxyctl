# proxyctl

面向 **WSL2** 的轻量级代理开关工具：自动探测宿主机网关 IP，一键开启/关闭，支持 fish 的 universal variables，亦可仅作用于当前进程。

---

## ✨ 特性
- `proxyctl on` / `proxyctl off` 一键切换
- 自动探测宿主机网关 IP（`ip route show`）
- 配置文件 `~/.proxyctl.toml`
- fish 集成（universal variables）
- 两种配置档：`mixed`（默认 7890）/ `split`（http=7892，socks=7891）
- 连通性测试：常见站点快速检查

---：

## 📦 安装

**一条命令（自动识别 glibc/musl）**
```bash
curl -fsSL https://raw.githubusercontent.com/xianyudd/proxyctl/main/scripts/install.sh | bash
````

固定版本：

```bash
curl -fsSL https://raw.githubusercontent.com/xianyudd/proxyctl/main/scripts/install.sh | env VERSION=v0.1.0 bash
```

无 sudo 安装到用户目录：

```bash
PREFIX="$HOME/.local" BINDIR="$HOME/.local/bin" \
curl -fsSL https://raw.githubusercontent.com/xianyudd/proxyctl/main/scripts/install.sh | bash
```

---

## 🚀 快速上手

```bash
proxyctl on [--ip <IP>]            # 开启代理（自动/手动 IP）
proxyctl off                       # 关闭代理
proxyctl status                    # 查看状态
proxyctl test [--ip <IP>] [--timeout <秒>] [--dry-run]  # 连通性测试
```

全局选项：

* `--mode <auto|process|fish-uvars>`
  `auto`（默认，fish 用 uvars；其他仅进程）/ `process` / `fish-uvars`
* `--profile <mixed|split>`
  `mixed`（混合端口 7890）/ `split`（http=7892，socks=7891）

---

## ⚙️ 配置

首次运行会生成 `~/.proxyctl.toml`：

```toml
[proxy]
# host_ip    = "172.17.208.1"   # 留空=自动探测；需要固定时再启用
profile    = "mixed"
mixed_port = 7890
socks_port = 7891
http_port  = 7892
```

---

## 🧱 平台

* Linux / WSL2 ✅（推荐）
* macOS 🟨（可能需要调整 IP 探测命令）
* Windows 原生命令行（cmd/Powershell）不支持，请在 WSL2 使用

---

## 🧪 开发

```bash
# 快速检查（fmt + 构建）
python scripts/check.py --fast

# 全量（clippy + 测试）
python scripts/check.py
cargo test
```

---

## 📜 许可证

MIT License，详见 [LICENSE](./LICENSE)。


