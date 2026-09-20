#!/usr/bin/env bash
set -e

export PATH=$HOME/.cargo/bin:$PATH
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_DIR"

echo "==> Building Mouros kernel & custom in-tree bootloader..."
cargo bootimage

ISO_ROOT="/tmp/mouros_iso_root"
rm -rf "$ISO_ROOT"
mkdir -p "$ISO_ROOT"

cp target/x86_64-mouros/debug/bootimage-mouros.bin "$ISO_ROOT/bootimage.bin"

echo "==> Creating bootable ISO with xorriso..."
xorriso -as mkisofs \
  -V "MOUROS" \
  -b bootimage.bin \
  -no-emul-boot \
  -boot-load-size 110 \
  -boot-info-table \
  -o mouros.iso \
  "$ISO_ROOT"

chmod +x mouros.iso || true
echo "==> Successfully generated mouros.iso ($(stat -c%s mouros.iso) bytes)!"
