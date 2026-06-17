#!/usr/bin/env bash
# migrate.sh — Nâng cấp một repo ĐÃ cài Harness theo cấu trúc CŨ (scripts/ +
# harness.db ở root + tài liệu hạ tầng nằm trong docs/) sang cấu trúc MỚI: gom
# toàn bộ hạ tầng vào MỘT thư mục `_harness/`.
#
# Chạy MỘT lần tại gốc repo đích đã cài Harness:
#     bash migrate.sh
# hoặc one-liner:
#     curl -fsSL https://raw.githubusercontent.com/haketienloc10/repo-harness/main/migrate.sh | bash
#
# An toàn:
#   - Đòi cây git SẠCH (mọi thay đổi dễ xem qua `git diff`, có thể revert).
#   - Dùng `git mv`/`git rm` khi file được track → giữ lịch sử.
#   - KHÔNG đụng workspace của repo: docs/{decisions,stories,product,wiki},
#     docs/KNOWLEDGE_INDEX.md và mã nguồn sản phẩm được giữ nguyên.
#   - Nhật ký trace/evidence trong DB là LỊCH SỬ → giữ nguyên; chỉ sửa chuỗi
#     path "sống" (story.verify_command).
set -Eeuo pipefail

REPO_OWNER="${HARNESS_LITE_OWNER:-haketienloc10}"
REPO_NAME="${HARNESS_LITE_REPO:-repo-harness}"
REF="${HARNESS_LITE_REF:-main}"
TARGET_DIR="${HARNESS_LITE_TARGET_DIR:-$PWD}"

log()  { printf '[migrate] %s\n' "$*"; }
fail() { printf '[migrate] ERROR: %s\n' "$*" >&2; exit 1; }

cd "$TARGET_DIR"

# --- 0. Tiền điều kiện ------------------------------------------------------
command -v curl >/dev/null 2>&1 || fail "Thiếu curl"
IN_GIT=0
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  IN_GIT=1
  [ -z "$(git status --porcelain)" ] || \
    fail "Cây git chưa sạch. Hãy commit/stash trước khi migrate để dễ review & revert."
fi

# --- 1. Phát hiện layout cũ -------------------------------------------------
OLD=0
[ -f harness.db ]        && OLD=1
[ -d scripts ]           && OLD=1
[ -f docs/HARNESS.md ]   && OLD=1
[ -f docs/TRACE_SPEC.md ] && OLD=1
if [ "$OLD" -eq 0 ]; then
  log "Không thấy dấu hiệu layout cũ (scripts/, harness.db ở root, docs/HARNESS.md…)."
  log "Repo có thể đã ở cấu trúc mới — không cần migrate. Dừng."
  exit 0
fi
log "Phát hiện layout CŨ → bắt đầu migrate sang _harness/."

# Helper: di chuyển file/dir, ưu tiên git mv để giữ lịch sử.
move() {
  local src="$1" dst="$2"
  [ -e "$src" ] || return 0
  mkdir -p "$(dirname "$dst")"
  if [ "$IN_GIT" -eq 1 ] && git ls-files --error-unmatch "$src" >/dev/null 2>&1; then
    git mv -k "$src" "$dst"
  else
    mv "$src" "$dst"
  fi
}
# Helper: xoá bản sao hạ tầng CŨ (đã có bản mới do install.sh mang về).
remove() {
  local p="$1"
  [ -e "$p" ] || return 0
  if [ "$IN_GIT" -eq 1 ] && git ls-files --error-unmatch "$p" >/dev/null 2>&1; then
    git rm -rq "$p"
  else
    rm -rf "$p"
  fi
}

# --- 2. Dời CSDL vận hành: harness.db (+wal/shm) -> _harness/ ----------------
# install.sh KHÔNG ship harness.db (loại trừ), nên _harness/harness.db sẽ trống
# sau khi cài; ta đưa DB cũ (dữ liệu thật) vào đúng chỗ engine mới mong đợi.
mkdir -p _harness
for f in harness.db harness.db-wal harness.db-shm; do
  if [ -f "$f" ] && [ ! -e "_harness/$f" ]; then
    move "$f" "_harness/$f"
    log "DB: $f -> _harness/$f"
  fi
done

# --- 3. Cài đè engine mới (binary, schema, reference docs, skill, block AGENTS)
# install.sh: overwrite mọi thứ trong _harness/* (nâng cấp khung), KHÔNG đè
# workspace docs/ của repo, và NHÚNG lại block Harness vào AGENTS.md.
log "Tải & cài khung mới từ ${REPO_OWNER}/${REPO_NAME}@${REF}…"
HARNESS_LITE_OWNER="$REPO_OWNER" HARNESS_LITE_REPO="$REPO_NAME" \
HARNESS_LITE_REF="$REF" HARNESS_LITE_TARGET_DIR="$TARGET_DIR" \
  bash -c "curl -fsSL 'https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/${REF}/install.sh' | bash"

# --- 4a. DỜI nội dung riêng-theo-repo (install KHÔNG ship lại → phải bảo toàn)
# TEST_MATRIX.md được generate per-repo; proposals/ có thể chứa đề xuất của repo.
move docs/TEST_MATRIX.md _harness/docs/TEST_MATRIX.md
if [ -d docs/proposals ]; then
  while IFS= read -r -d '' f; do
    move "$f" "_harness/docs/proposals/${f#docs/proposals/}"
  done < <(find docs/proposals -type f -print0)
  remove docs/proposals              # vỏ thư mục rỗng còn lại
fi
log "Đã dời TEST_MATRIX.md + proposals/ riêng của repo sang _harness/docs/."

# --- 4b. XOÁ bản sao hạ tầng do install SHIP LẠI TƯƠI (bản cũ nay là stale) --
remove scripts                       # bin/ + schema/ + README cũ
for d in \
  ARCHITECTURE CLI_REFERENCE CONTEXT_RULES FEATURE_INTAKE GLOSSARY \
  HARNESS_AUDIT HARNESS_BACKLOG HARNESS_COMPONENTS HARNESS_MATURITY HARNESS \
  IMPROVEMENT_PROTOCOL TOOL_REGISTRY TRACE_SPEC README; do
  remove "docs/${d}.md"
done
remove docs/templates                # harness-owned → bản mới ở _harness/docs/templates
log "Đã gỡ bản sao hạ tầng cũ (scripts/, reference docs, templates) ở vị trí cũ."

# --- 5. Sửa chuỗi path "sống" trong DB --------------------------------------
# CHỈ sửa story.verify_command (lệnh sẽ chạy lại). Trace/evidence là lịch sử →
# giữ nguyên để audit log không bị bóp méo. import brownfield refresh các bảng
# seed từ markdown (TEST_MATRIX/decisions/backlog) ở vị trí _harness/ mới.
BIN="_harness/bin/harness-cli"
if [ -x "$BIN" ]; then
  if command -v python3 >/dev/null 2>&1; then
    python3 - <<'PY'
import sqlite3, os
db = sqlite3.connect("_harness/harness.db")
cur = db.cursor()
n = 0
for sid, vc in cur.execute(
        "SELECT id, verify_command FROM story WHERE verify_command LIKE 'scripts/%'").fetchall():
    new = vc.replace("scripts/bin/", "_harness/bin/").replace("scripts/schema", "_harness/schema")
    cur.execute("UPDATE story SET verify_command=? WHERE id=?", (new, sid))
    n += 1
db.commit()
print(f"[migrate] Sửa {n} story.verify_command sang _harness/")
PY
  else
    log "Thiếu python3 → BỎ QUA sửa story.verify_command tự động."
    log "Hãy chạy tay: $BIN story update --id <ID> --verify '_harness/bin/harness-cli …'"
  fi
  # Áp mọi migration schema mới (idempotent) + refresh bảng seed từ markdown mới.
  "$BIN" migrate >/dev/null 2>&1 || true
  "$BIN" import brownfield >/dev/null 2>&1 || true
fi

# --- 6. Regenerate bản đồ Orient (cấu trúc đã đổi) --------------------------
if [ -x "$BIN" ]; then
  "$BIN" knowledge scaffold >/dev/null 2>&1 || true
  log "Đã scaffold lại docs/KNOWLEDGE_INDEX.md — HÃY soạn lại mô tả mục còn 'TODO'."
fi

# --- 7. Kết & kiểm tra ------------------------------------------------------
log "Migrate hoàn tất. Kiểm tra:"
if [ -x "$BIN" ]; then
  "$BIN" knowledge check 2>&1 | sed 's/^/  /' || true
fi
cat <<EOF

[migrate] BƯỚC THỦ CÔNG còn lại:
  1. Mở docs/KNOWLEDGE_INDEX.md: thay mọi 'TODO: describe.' bằng mô tả 1 dòng,
     rồi: npx prettier --write docs/KNOWLEDGE_INDEX.md
  2. Soát AGENTS.md: block giữa HARNESS:BEGIN/END đã được cập nhật sang
     namespace _harness/ — giữ nguyên hướng dẫn riêng của repo bên ngoài block.
  3. Xem lại toàn bộ thay đổi: git status && git diff   →   commit khi ưng.
EOF
