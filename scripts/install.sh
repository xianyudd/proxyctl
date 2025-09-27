#!/usr/bin/env sh
set -e

REPO="xianyudd/proxyctl"   #仓库地址
BIN="proxyctl"
PREFIX="${PREFIX:-/usr/local}"
BINDIR="${BINDIR:-$PREFIX/bin}"
TMPDIR="${TMPDIR:-$(mktemp -d)}"

# 支持 VERSION=v0.1.0 指定版本；缺省用 latest
VERSION="${VERSION:-latest}"

command_exists() { command -v "$1" >/dev/null 2>&1; }

detect_libc() {
  if command_exists ldd; then
    if ldd --version 2>&1 | grep -iq musl; then echo "musl"; else echo "gnu"; fi
    return
  fi
  if getconf GNU_LIBC_VERSION >/dev/null 2>&1; then echo "gnu"; return; fi
  if ls /lib/*musl* >/dev/null 2>&1 || ls /usr/lib/*musl* >/dev/null 2>&1; then
    echo "musl"
  else
    echo "gnu"
  fi
}

sha256_check() {
  FILE="$1"; SUMFILE="$2"
  if command_exists sha256sum; then
    sha256sum -c "$SUMFILE"
  elif command_exists shasum; then
    shasum -a 256 -c "$SUMFILE"
  else
    echo "⚠️ 未找到 sha256sum/shasum，跳过校验" >&2
    return 0
  fi
}

arch="$(uname -m)"
case "$arch" in
  x86_64|amd64) target_arch="x86_64" ;;
  *) echo "暂不支持架构: $arch"; exit 1 ;;
esac

libc="$(detect_libc)"
case "$libc" in
  gnu)  target="${target_arch}-unknown-linux-gnu" ;;
  musl) target="${target_arch}-unknown-linux-musl" ;;
  *) echo "无法识别 libc: $libc"; exit 1 ;;
esac

echo "检测到: arch=$target_arch libc=$libc → target=$target"

if [ "$VERSION" = "latest" ]; then
  BASE="https://github.com/$REPO/releases/latest/download"
  PKG="${BIN}-${target}.tar.gz"
  SUM="${PKG}.sha256"
else
  BASE="https://github.com/$REPO/releases/download/${VERSION}"
  PKG="${BIN}-${VERSION}-${target}.tar.gz"
  SUM="${PKG}.sha256"
fi

echo "下载: $BASE/$PKG"
cd "$TMPDIR"
curl -fL --retry 3 -o "$PKG" "$BASE/$PKG"
if curl -fsL -o "$SUM" "$BASE/$SUM"; then
  echo "校验 SHA256..."
  sha256_check "$PKG" "$SUM"
fi

mkdir -p pkg && tar -xzf "$PKG" -C pkg
mkdir -p "$BINDIR"
install -m 0755 "pkg/$BIN" "$BINDIR/$BIN"

echo "✅ 安装完成: $BINDIR/$BIN"
"$BINDIR/$BIN" --version || true

