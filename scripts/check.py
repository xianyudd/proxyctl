#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import argparse
import tempfile
from pathlib import Path
import subprocess
import sys
from typing import List, Tuple, Optional

# ===== 基础工具 =====
def run(cmd: List[str], cwd: Path = Path.cwd(), capture: bool = True) -> Tuple[int, str, str]:
    proc = subprocess.run(
        cmd,
        cwd=str(cwd),
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    return proc.returncode, proc.stdout or "", proc.stderr or ""

def which(prog: str) -> Optional[str]:
    from shutil import which as _which
    return _which(prog)

def in_git_repo() -> bool:
    code, _, _ = run(["git", "rev-parse", "--is-inside-work-tree"])
    return code == 0

def changed_files() -> List[str]:
    if not in_git_repo():
        return []
    code, out, _ = run(["git", "ls-files", "-m"])
    if code != 0:
        return []
    return [x.strip() for x in out.splitlines() if x.strip()]

def section(title: str):
    print(f"\n== {title} ==")

# ===== 提取摘要 =====
def print_extracted_clippy_issues(log_path: Path):
    if not log_path.exists():
        print("  （未找到 clippy 日志）")
        return
    lines = log_path.read_text(encoding="utf-8", errors="ignore").splitlines()
    show_next = False
    printed = 0
    for ln in lines:
        if "error:" in ln or "warning:" in ln:
            print("  " + ln)
            show_next = True
            printed += 1
            continue
        if show_next and ln.strip().startswith("-->"):
            print("  " + ln)
            show_next = False
    if printed == 0:
        print("  （无可提炼的问题，详见完整日志）")

def print_failed_tests_summary(log_path: Path):
    if not log_path.exists():
        return
    for ln in log_path.read_text(encoding="utf-8", errors="ignore").splitlines():
        if ln.startswith("failures:") or ln.startswith("---- "):
            print("  " + ln)

def optional_tool(name: str, cmd: List[str]):
    if which(name):
        print(f"-- {name} --")
        # 可选工具不影响主流程通过与否
        run(cmd, capture=False)
    else:
        print(f"跳过 {name}（未安装）")

# ===== 主流程 =====
def main() -> int:
    parser = argparse.ArgumentParser(description="proxyctl 本地检查脚本")
    parser.add_argument("--fast", action="store_true",
                        help="快速模式：自动格式化 + 编译测试（不运行），适合 pre-commit")
    parser.add_argument("--build", action="store_true",
                        help="在全量模式额外执行 cargo build --release --locked（默认不跑）")
    args = parser.parse_args()

    tmpdir = Path(tempfile.mkdtemp(prefix="proxyctl-check-"))
    clippy_log = tmpdir / "clippy.log"
    test_log   = tmpdir / "test.log"
    fmt_changed = tmpdir / "fmt_changed.txt"
    print(f"📝 Logs -> {tmpdir}")

    # 0) 工具存在性
    for tool in ["cargo", "rustc"]:
        if not which(tool):
            print(f"❌ 未找到 {tool}（请安装 Rust 工具链）")
            return 1

    # 1) 自动格式化
    section("fmt（自动格式化）")
    run(["cargo", "fmt", "--all"], capture=False)
    changed = [f for f in changed_files() if f.endswith((".rs", ".toml", ".lock"))]
    fmt_changed.write_text("\n".join(changed), encoding="utf-8")
    if changed:
        print("已自动格式化的文件：")
        for f in changed:
            print(" ", f)
    else:
        print("无需格式化（已规范）")

    # 快速模式：只做“能否编译”的快速检查
    if args.fast:
        section("fast compile（不运行测试）")
        code, _, _ = run(["cargo", "test", "--no-run", "--locked"], capture=False)
        if code != 0:
            print("❌ 编译测试未通过")
            return 1
        print("✅ 快速检查通过")
        return 0

    # 2) clippy（尝试自动修复 + 严格检查）
    section("clippy（尝试自动修复）")
    # 尝试自动修复（不可用/无修复项会失败，忽略退出码）
    run(["cargo", "clippy", "--fix", "-Z", "unstable-options", "--allow-dirty", "--allow-staged"])
    # 严格：把 warning 当错误
    code, out, err = run(["cargo", "clippy", "--all-targets", "--", "-D", "warnings"])
    (clippy_log).write_text(out + err, encoding="utf-8")
    clippy_failed = code != 0
    if clippy_failed:
        print(f"❌ Clippy 未通过（日志：{clippy_log})")
    else:
        print("✅ Clippy 通过")

    # 3) （可选）构建
    build_failed = False
    if args.build:
        section("build（release，可选）")
        code, _, _ = run(["cargo", "build", "--release", "--locked"], capture=False)
        build_failed = code != 0
        print("✅ Build 通过" if not build_failed else "❌ Build 失败")

    # 4) 测试
    section("test（全部）")
    code, out, err = run(["cargo", "test", "--all", "--all-features", "--locked"])
    (test_log).write_text(out + err, encoding="utf-8")
    sys.stdout.write(out)
    sys.stderr.write(err)
    test_failed = code != 0
    print("✅ Test 通过" if not test_failed else f"❌ Test 失败（日志：{test_log})")

    # 5) 依赖与安全（可选工具存在才执行）
    section("依赖与安全（可选工具存在才执行）")
    optional_tool("cargo-audit",   ["cargo", "audit"])
    optional_tool("cargo-udeps",   ["cargo", "udeps", "--workspace"])
    optional_tool("cargo-outdated",["cargo", "outdated", "-R"])

    # 6) 汇总
    section("汇总（需要手动修改的内容）")
    if clippy_failed:
        print("👉 Clippy 仍有问题（请手动修改）：")
        print_extracted_clippy_issues(clippy_log)
    else:
        print("Clippy 无需手动修改。")
    if args.build and build_failed:
        print("👉 Build 失败：请查看构建错误（或重跑：cargo build --release --locked）")
    if test_failed:
        print("👉 测试失败：建议查看失败用例（或日志：", test_log, ")")
        print_failed_tests_summary(test_log)

    # 7) 展示可能被自动格式化/修复的文件（基于 git 变更）
    if in_git_repo():
        section("本次可能被自动格式化/修复的文件（基于 git 变更）")
        code, out, _ = run(["git", "status", "--porcelain"])
        if out.strip():
            print(out, end="")
        else:
            print("  无")

    # 8) 退出码策略
    if clippy_failed or test_failed or (args.build and build_failed):
        print("\n❌ 检查未通过，请根据上面的摘要处理后重试。")
        return 1

    print("\n✅ 所有检查通过。")
    return 0

if __name__ == "__main__":
    sys.exit(main())

