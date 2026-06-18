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
  "_harness"
  "docs"
  ".agents"
)

# AGENTS.md KHÔNG nằm trong INSTALL_ITEMS: thay vì copy nguyên file, ta NHÚNG
# block Harness vào AGENTS.md của repo đích (xem install_agents_md). Nhờ vậy nội
# dung "đây là tooling, KHÔNG phải source sản phẩm" chỉ xuất hiện ở repo ĐÍCH —
# còn repo-harness (nơi _harness/ chính LÀ sản phẩm) không bị dính rule đó.
HARNESS_BLOCK_BEGIN="<!-- HARNESS:BEGIN -->"
HARNESS_BLOCK_END="<!-- HARNESS:END -->"

# Danh sách file thực sự được copy — ghi vào _harness/.harness-manifest ở cuối.
# Vừa là DẤU HIỆU "repo này đã cài Harness", vừa phục vụ gỡ/nâng cấp về sau.
INSTALLED_FILES=()

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
  mkdir -p "$TARGET_DIR/_harness/docs/proposals"
}

# Trả về 0 (true => LOẠI) nếu path tương đối là artifact riêng của repo nguồn.
is_excluded() {
  local p="$1"
  case "$p" in
    # Dữ liệu vận hành / evidence / CSDL riêng của repo nguồn (đều trong _harness/)
    _harness/harness.db) return 0 ;;
    _harness/.harness-manifest) return 0 ;;
    _harness/evidence/*) return 0 ;;
    # Ma trận test được generate riêng cho repo nguồn
    _harness/docs/TEST_MATRIX.md) return 0 ;;
    # Bản đồ orient + wiki được generate riêng cho repo nguồn
    docs/KNOWLEDGE_INDEX.md) return 0 ;;
    docs/wiki/*) return 0 ;;
    # Thư mục scaffold: chỉ giữ hướng dẫn generic, bỏ nội dung thực của repo nguồn
    _harness/docs/proposals/*)
      case "$p" in _harness/docs/proposals/README.md) return 1 ;; *) return 0 ;; esac ;;
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

# Sinh block Harness (kèm marker) để nhúng vào AGENTS.md của repo đích.
build_harness_block() {
  printf '%s\n' "$HARNESS_BLOCK_BEGIN"
  cat <<'EOF'

## Harness

Repo này **CÀI Harness** (xem `_harness/.harness-manifest`). Toàn bộ **tầng vận
hành** của Harness nằm gọn trong MỘT thư mục:

- `_harness/` — engine (`_harness/bin/harness-cli`, `_harness/schema/`), CSDL vận
  hành (`_harness/harness.db`), tài liệu khung + template (`_harness/docs/`), và
  skill. Đây **KHÔNG** phải mã nguồn sản phẩm của repo: đừng sửa/test/review/
  refactor như source, đừng mô tả như "the codebase" trong orient/wiki. Chỉ chạm
  khi tác vụ là **Harness Delta**.

➡️ **MỌI THỨ NGOÀI `_harness/` là của repo này** — gồm mã nguồn sản phẩm và tài
liệu sản phẩm trong `docs/` (`product/`, `stories/`, `decisions/`, `wiki/`,
`KNOWLEDGE_INDEX.md`) mà Harness chỉ quản lý ĐỊNH DẠNG; được sửa khi làm story.

**GATE (chặn cứng — không bỏ qua):** Hành động ĐẦU TIÊN của bạn trong repo này là
đọc `_harness/00-AGENTS.md`. Khi chưa đọc xong: KHÔNG đọc code, KHÔNG lập kế
hoạch, KHÔNG sửa/chạy bất cứ thứ gì. Áp dụng cho MỌI tác vụ — kể cả tác vụ trông
như một dòng. File đó định nghĩa quy trình bắt buộc của repo; bỏ qua = output sai
quy trình.
EOF
  printf '%s\n' "$HARNESS_BLOCK_END"
}

# Thay nội dung giữa marker HARNESS:BEGIN/END bằng block mới (idempotent khi
# cài lại / nâng cấp). Block mới đã chứa sẵn cả hai marker.
replace_harness_block() {
  local file="$1" block="$2" tmp
  tmp="$(mktemp)"
  BLOCK="$block" awk -v b="$HARNESS_BLOCK_BEGIN" -v e="$HARNESS_BLOCK_END" '
    $0 == b { print ENVIRON["BLOCK"]; skip=1; next }
    $0 == e { skip=0; next }
    !skip   { print }
  ' "$file" > "$tmp"
  mv "$tmp" "$file"
}

# Đảm bảo AGENTS.md repo đích có block Harness, KHÔNG ghi đè hướng dẫn riêng của
# họ: tạo mới nếu chưa có; thay block nếu đã có marker; chèn cuối nếu chưa có.
install_agents_md() {
  local dest="$TARGET_DIR/AGENTS.md" block
  block="$(build_harness_block)"

  if [ ! -e "$dest" ]; then
    {
      printf '# Agent Instructions\n\n'
      printf 'Add project-specific agent instructions here.\n\n'
      printf '%s\n' "$block"
    } >"$dest"
    log "Tạo mới AGENTS.md + nhúng block Harness"
  elif grep -qF "$HARNESS_BLOCK_BEGIN" "$dest"; then
    replace_harness_block "$dest" "$block"
    log "Cập nhật block Harness trong AGENTS.md có sẵn"
  else
    printf '\n%s\n' "$block" >>"$dest"
    log "Chèn block Harness vào cuối AGENTS.md có sẵn"
  fi
}

# Ghi _harness/.harness-manifest: đánh dấu repo đã cài Harness + liệt kê file.
write_manifest() {
  local manifest="$TARGET_DIR/_harness/.harness-manifest"
  mkdir -p "$(dirname "$manifest")"
  {
    printf '# Harness manifest — sinh tự động bởi install.sh, KHÔNG sửa tay.\n'
    printf '# Sự hiện diện của file này = repo CÀI Harness (không phải repo nguồn).\n'
    printf 'source = %s/%s\n' "$REPO_OWNER" "$REPO_NAME"
    printf 'ref = %s\n' "$REF"
    printf '\n[files]\n'
    if [ "${#INSTALLED_FILES[@]}" -gt 0 ]; then
      printf '%s\n' "${INSTALLED_FILES[@]}" | LC_ALL=C sort
    fi
  } >"$manifest"
  log "Ghi manifest: _harness/.harness-manifest (${#INSTALLED_FILES[@]} file)"
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
EXISTING_FILES=0

# Copy một file đơn lẻ, tôn trọng is_excluded + KHÔNG ghi đè file của repo đích.
copy_file() {
  local rel="$1" src="$2"
  if is_excluded "$rel"; then
    SKIPPED_FILES=$((SKIPPED_FILES + 1))
    return 0
  fi
  local dest="$TARGET_DIR/$rel"
  # _harness/ thuộc Harness hoàn toàn → luôn ghi đè để NÂNG CẤP khung. Mọi path
  # khác (dotfile, docs/ workspace) có thể là tài sản của repo đích → KHÔNG đè
  # nếu đã tồn tại, tránh nuốt config/nội dung sẵn có của họ.
  case "$rel" in
    _harness/*) : ;;
    *)
      if [ -e "$dest" ]; then
        EXISTING_FILES=$((EXISTING_FILES + 1))
        return 0
      fi
      ;;
  esac
  mkdir -p "$(dirname "$dest")"
  cp "$src" "$dest"
  INSTALLED_FILES+=("$rel")
}

for item in "${INSTALL_ITEMS[@]}"; do
  src="$SRC_DIR/$item"

  if [ ! -e "$src" ]; then
    MISSING_ITEMS+=("$item")
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

# Nhúng block Harness vào AGENTS.md repo đích (sau khi _harness/ đã có mặt) và
# ghi manifest đánh dấu chế độ "đã cài Harness".
install_agents_md
write_manifest

if [ "$SKIPPED_FILES" -gt 0 ]; then
  log "Đã bỏ qua $SKIPPED_FILES file artifact (tài nguyên riêng của repo nguồn)."
fi

if [ "$EXISTING_FILES" -gt 0 ]; then
  log "Giữ nguyên $EXISTING_FILES file đã có sẵn của repo đích (không ghi đè)."
fi

if [ "${#MISSING_ITEMS[@]}" -gt 0 ]; then
  log "Một số item không tồn tại trong repo source:"
  for item in "${MISSING_ITEMS[@]}"; do
    printf '  - %s\n' "$item"
  done
fi

log "Hoàn tất."
