#!/usr/bin/env bash
# ============================================================
#  一键打包：Linux 下产出"完全静态(musl)"的后端可执行文件
#
#  用法：把这个脚本放在 rust_houduan 目录下，然后执行
#        bash build-linux-musl.sh
#
#  产物：项目根目录下的 ./renwu_houduan（可直接 scp 到服务器）
#        每次运行前会自动检查并补齐 musl 工具，无需手动装。
# ============================================================
set -euo pipefail

# ---------- 可修改的配置 ----------
CARGO_BIN="renwu-houduan"    # Cargo 编译产物名（由 Cargo.toml 的 [[bin]] name 决定）
OUTPUT_NAME="renwu_houduan"  # 最终输出的可执行文件名（部署到服务器用这个名字）
MUSL_TARGET="x86_64-unknown-linux-musl"
# ----------------------------------

cd "$(dirname "$0")"

if [ ! -f Cargo.toml ]; then
  echo "错误：没找到 Cargo.toml，请把脚本放到 rust_houduan 目录下再运行。" >&2
  exit 1
fi

echo "== [1/4] 检查 musl 编译目标 =="
if ! rustup target list --installed 2>/dev/null | grep -qx "$MUSL_TARGET"; then
  rustup target add "$MUSL_TARGET"
else
  echo "  已安装：$MUSL_TARGET"
fi

echo "== [2/4] 检查 musl 链接器 (musl-gcc) =="
if ! command -v musl-gcc >/dev/null 2>&1; then
  if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update && sudo apt-get install -y musl-tools
  elif command -v yum >/dev/null 2>&1; then
    sudo yum install -y musl-gcc
  else
    echo "找不到 apt/yum，请手动安装 musl-gcc 后再运行。" >&2
    exit 1
  fi
else
  echo "  已安装：musl-gcc"
fi

echo "== [3/4] 编译 =="
cargo build --release --target "$MUSL_TARGET"

echo "== [4/4] 产出部署文件 =="
SRC="target/$MUSL_TARGET/release/$CARGO_BIN"
install -m 0755 "$SRC" "./$OUTPUT_NAME"
file "./$OUTPUT_NAME"

echo
echo "完成！部署文件：$(pwd)/$OUTPUT_NAME"
