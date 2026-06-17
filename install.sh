#!/usr/bin/env bash
set -Eeuo pipefail

REPO_OWNER="${HARNESS_LITE_OWNER:-haketienloc10}"
REPO_NAME="${HARNESS_LITE_REPO:-repo-harness}"
REF="${HARNESS_LITE_REF:-main}"
TARGET_DIR="${HARNESS_LITE_TARGET_DIR:-$PWD}"

ARCHIVE_URL="https://codeload.github.com/${REPO_OWNER}/${REPO_NAME}/tar.gz/${REF}"

# Khung mẫu (scaffold) được cài vào repo đích. CHỈ liệt kê những thứ là bộ
# khung dùng chung cho mọi repo — KHÔNG liệt kê tài nguyên riêng của repo
# repo-harness (xem EXCLUDE_PATHS bên dưới để lọc artifact lẫn trong các thư mục).
INSTALL_ITEMS=(
  ".editorconfig"
  ".prettierignore"
  ".prettierrc"
  "AGENTS.md"
  "_harness"
  "docs"
  "scripts"
  ".agents"
)

# Artifact là TÀI NGUYÊN riêng của repo-harness — không phải khung mẫu, không
# được sao chép sang repo đích. So khớp theo đường dẫn tương đối tính từ gốc
# repo (xem is_excluded). Quy ước:
#   - "dir/*"      => bỏ MỌI file dưới dir đó
#   - "dir/keep/*" cộng nhánh keep ở is_excluded => giữ lại ngoại lệ
# Các thư mục product/stories/decisions/proposals chỉ giữ README/backlog/template
# generic; nội dung thực (story, decision record, proposal, read-model...) bị loại.
ensure_empty_dir() {
  # Một số thư mục scaffold (vd: proposals) sau khi lọc sẽ rỗng. Tạo sẵn để
  # agent có chỗ ghi mà không kéo theo artifact của repo nguồn.
  mkdir -p "$TARGET_DIR/docs/proposals"
}

# Trả về 0 (true => LOẠI) nếu path tương đối là artifact riêng của repo nguồn.
is_excluded() {
  local p="$1"
  case "$p" in
    # Dữ liệu vận hành / evidence riêng của repo nguồn
    harness.db) return 0 ;;
    _harness/evidence/*) return 0 ;;
    # Bản đồ orient + wiki được generate riêng cho repo nguồn
    docs/KNOWLEDGE_INDEX.md) return 0 ;;
    docs/wiki/*) return 0 ;;
    # Thư mục scaffold: chỉ giữ hướng dẫn generic, bỏ nội dung thực của repo nguồn
    docs/proposals/*)
      case "$p" in docs/proposals/README.md) return 1 ;; *) return 0 ;; esac ;;
    docs/decisions/*)
      case "$p" in docs/decisions/README.md) return 1 ;; *) return 0 ;; esac ;;
    docs/product/*)
      case "$p" in docs/product/README.md) return 1 ;; *) return 0 ;; esac ;;
    docs/stories/epics/*)
      case "$p" in docs/stories/epics/README.md) return 1 ;; *) return 0 ;; esac ;;
    docs/stories/*)
      case "$p" in
        docs/stories/README.md | docs/stories/backlog.md) return 1 ;;
        *) return 0 ;;
      esac ;;
  esac
  return 1
}

log() {
  printf '[repo-harness] %s\n' "$*"
}

fail() {
  printf '[repo-harness] ERROR: %s\n' "$*" >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || fail "Thiếu curl"
command -v tar >/dev/null 2>&1 || fail "Thiếu tar"

[ -d "$TARGET_DIR" ] || fail "TARGET_DIR không tồn tại: $TARGET_DIR"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

log "Đang tải ${REPO_OWNER}/${REPO_NAME}@${REF}..."
curl -fsSL "$ARCHIVE_URL" -o "$TMP_DIR/source.tar.gz"

log "Đang giải nén..."
tar -xzf "$TMP_DIR/source.tar.gz" -C "$TMP_DIR"

SRC_DIR="$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
[ -n "$SRC_DIR" ] || fail "Không tìm thấy thư mục source sau khi giải nén"

log "Cài khung mẫu vào workspace: $TARGET_DIR"

MISSING_ITEMS=()
SKIPPED_FILES=0

# Copy một file đơn lẻ, tôn trọng is_excluded.
copy_file() {
  local rel="$1" src="$2"
  if is_excluded "$rel"; then
    SKIPPED_FILES=$((SKIPPED_FILES + 1))
    return 0
  fi
  local dest="$TARGET_DIR/$rel"
  mkdir -p "$(dirname "$dest")"
  cp "$src" "$dest"
}

for item in "${INSTALL_ITEMS[@]}"; do
  src="$SRC_DIR/$item"

  if [ ! -e "$src" ]; then
    MISSING_ITEMS+=("$item")
    continue
  fi

  # Không ghi đè AGENTS.md sẵn có của repo đích (chứa hướng dẫn riêng của họ).
  if [ "$item" = "AGENTS.md" ] && [ -e "$TARGET_DIR/AGENTS.md" ]; then
    log "Giữ nguyên AGENTS.md có sẵn (không ghi đè)"
    continue
  fi

  if [ -d "$src" ]; then
    # Duyệt từng file, bỏ qua artifact theo is_excluded.
    while IFS= read -r -d '' f; do
      rel="${f#"$SRC_DIR"/}"
      copy_file "$rel" "$f"
    done < <(find "$src" -type f -print0)
    log "Copied dir: $item"
  else
    copy_file "$item" "$src"
    log "Copied file: $item"
  fi
done

ensure_empty_dir

if [ "$SKIPPED_FILES" -gt 0 ]; then
  log "Đã bỏ qua $SKIPPED_FILES file artifact (tài nguyên riêng của repo nguồn)."
fi

if [ "${#MISSING_ITEMS[@]}" -gt 0 ]; then
  log "Một số item không tồn tại trong repo source:"
  for item in "${MISSING_ITEMS[@]}"; do
    printf '  - %s\n' "$item"
  done
fi

log "Hoàn tất."
