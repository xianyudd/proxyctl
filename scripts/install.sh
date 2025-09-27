#!/usr/bin/env sh
# POSIX sh 版本：稳健、可在多数 Linux 直接运行
set -eu
umask 022

REPO="${REPO:-xianyudd/proxyctl}"   # 可用 REPO 覆盖
BIN="${BIN:-proxyctl}"
VERSION="${VERSION:-latest}"         # latest 或 v0.1.0
PREFIX="${PREFIX:-}"                 # 留空=自动选择
BINDIR="${BINDIR:-}"                 # 留空=自动选择
CHECKSUM="${CHECKSUM:-1}"            # 1=启用校验，0=跳过（调试时可临时设0）

info() { printf '%s\n' "==> $*"; }
warn() { printf '%s\n' "⚠️  $*" >&2; }
die()  { printf '%s\n' "❌ $*" >&2; exit 1; }

command_exists() { command -v "$1" >/dev/null 2>&1; }

TMPDIR="${TMPDIR:-$(mktemp -d 2>/dev/null || mktemp -d -t proxyctl)}"
cleanup() { rm -rf "$TMPDIR"; }
trap cleanup INT TERM EXIT

# --- 下载器：curl 优先，wget 兜底 ---
download() {
  url="$1"; out="$2"
  if command_exists curl; then
    curl -fL --retry 3 --connect-timeout 10 -o "$out" "$url"
  elif command_exists wget; then
    wget -q -O "$out" "$url"
  else
    die "需要 curl 或 wget"
  fi
}

# --- 检测架构 ---
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64)  ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) die "不支持的架构: $ARCH" ;;
esac

# --- 检测 libc ---
detect_libc() {
  if command_exists ldd && ldd --version 2>&1 | grep -iq musl; then
    echo musl; return
  fi
  if command_exists getconf && getconf GNU_LIBC_VERSION >/dev/null 2>&1; then
    echo gnu; return
  fi
  # 兜底：存在 musl 动态链接器文件即判为 musl
  if ls /lib/*musl* >/dev/null 2>&1 || ls /usr/lib/*musl* >/dev/null 2>&1; then
    echo musl
  else
    echo gnu
  fi
}
LIBC="$(detect_libc)"
TARGET="${ARCH}-unknown-linux-${LIBC}"
info "检测到: arch=${ARCH} libc=${LIBC} → target=${TARGET}"

# --- 组装下载 URL ---
if [ "$VERSION" = "latest" ]; then
  BASE="https://github.com/${REPO}/releases/latest/download"
  PKG="${BIN}-${TARGET}.tar.gz"
  SUM="${PKG}.sha256"
else
  BASE="https://github.com/${REPO}/releases/download/${VERSION}"
  PKG="${BIN}-${VERSION}-${TARGET}.tar.gz"
  SUM="${PKG}.sha256"
fi

# --- 下载包与校验文件 ---
cd "$TMPDIR"
info "下载: $BASE/$PKG"
download "$BASE/$PKG" "$PKG"

if [ "$CHECKSUM" = "1" ]; then
  if download "$BASE/$SUM" "$SUM"; then
    # 只比对哈希，不依赖文件名
    if command_exists sha256sum; then
      EXPECTED="$(awk 'NR==1{print $1}' "$SUM")"
      ACTUAL="$(sha256sum "$PKG" | awk '{print $1}')"
    elif command_exists shasum; then
      EXPECTED="$(awk 'NR==1{print $1}' "$SUM")"
      ACTUAL="$(shasum -a 256 "$PKG" | awk '{print $1}')"
    else
      warn "未找到 sha256sum/shasum，跳过校验"
      EXPECTED=""; ACTUAL=""
    fi
    if [ -n "${EXPECTED}" ] && [ -n "${ACTUAL}" ]; then
      [ "$EXPECTED" = "$ACTUAL" ] || die "SHA256 校验失败：expected=$EXPECTED actual=$ACTUAL"
      info "SHA256 校验通过"
    fi
  else
    warn "未获取到校验文件，跳过校验"
  fi
else
  warn "已跳过 SHA256 校验（CHECKSUM=0）"
fi

# --- 选择安装目录 ---
choose_bindir() {
  # 用户显式指定优先
  if [ -n "$BINDIR" ]; then echo "$BINDIR"; return; fi
  if [ -n "$PREFIX" ]; then echo "$PREFIX/bin"; return; fi
  # 系统可写 → /usr/local/bin；否则回退 ~/.local/bin
  if [ -w /usr/local/bin ] 2>/dev/null; then echo "/usr/local/bin"; return; fi
  echo "${HOME}/.local/bin"
}
BINDIR="$(choose_bindir)"
mkdir -p "$BINDIR"

# PATH 提示
case ":$PATH:" in
  *":$BINDIR:"*) : ;;
  *) warn "$BINDIR 不在 PATH 中。可执行：  echo 'export PATH=\"$BINDIR:\$PATH\"' >> ~/.bashrc && . ~/.bashrc" ;;
esac

# --- 解包并安装 ---
WORK="$TMPDIR/unpack"
mkdir -p "$WORK"
tar -xzf "$PKG" -C "$WORK"

# 兼容包内层级：优先直接在根找，同名文件找不到再扫描
SRC=""
[ -f "$WORK/$BIN" ] && SRC="$WORK/$BIN"
if [ -z "$SRC" ]; then
  # 最多向下一层/两层找
  SRC="$(find "$WORK" -maxdepth 2 -type f -name "$BIN" | head -n1 || true)"
fi
[ -n "$SRC" ] || die "安装包里未找到可执行文件：$BIN"

# 安装
DEST="$BINDIR/$BIN"
# 有些系统没有 /usr/bin/install，用 cp 兜底
if command_exists install; then
  install -m 0755 "$SRC" "$DEST"
else
  cp "$SRC" "$DEST" && chmod 0755 "$DEST"
fi

info "安装完成：$DEST"
"$DEST" --version >/dev/null 2>&1 && "$DEST" --version || true

