# Skill: generate-knowledge-index

- **Trigger:**
  `IF [cần (tạo mới HOẶC làm mới) bản đồ onboarding repo: docs/KNOWLEDGE_INDEX.md]`
  — repo mới cài Harness chưa có index; HOẶC cấu trúc thư mục / tech stack đổi;
  HOẶC `harness-cli knowledge check` báo lỗi (thiếu/lệch/TODO). Bỏ qua nếu index
  đã có và `check` xanh.
- **Lane:** mọi lane (thường gắn GĐ2 khi onboard hoặc GĐ6 khi bảo trì).
- **Giai đoạn:** GĐ2 (Planning) hoặc GĐ6 (Growth).

Index = phần SỰ THẬT (Key Technologies, How to Run, Top-Level Structure, Key
Subdirectories) do `harness-cli` sinh tất định + phần NGỮ NGHĨA (Purpose, Key
Concepts) do người/agent soạn và được giữ lại giữa các marker. KHÔNG tự gõ tay
phần sự thật.

## INPUT (đọc trước khi chạy)

- `_harness/03-CLI_REFERENCE.md` mục `knowledge` để biết cú pháp.
- `_harness/docs/GLOSSARY.md` + `README.md` (+ `docs/` nếu có) để soạn Purpose /
  Concepts.
- Index hiện tại nếu có: `docs/KNOWLEDGE_INDEX.md`.

## STEPS

1. **SCAFFOLD:** chạy `_harness/bin/harness-cli knowledge scaffold`. Lệnh tạo/làm
   mới `docs/KNOWLEDGE_INDEX.md`: regenerate Key Technologies, How to Run,
   Top-Level Structure và Key Subdirectories; GIỮ NGUYÊN Purpose/Key Concepts và
   mô tả từng mục đã soạn. How to Run là tất định (rút từ manifest) — không soạn
   tay.
2. **AUTHOR:** điền giữa các marker — `KNOWLEDGE:PURPOSE:*` (1–3 câu repo dùng
   để làm gì) và `KNOWLEDGE:CONCEPTS:*` (thuật ngữ lõi, trỏ `_harness/docs/GLOSSARY.md`,
   KHÔNG nhân bản). Thay mọi `TODO: describe.` ở Top-Level Structure VÀ Key
   Subdirectories bằng mô tả 1 dòng.
3. **FORMAT:** chạy `npx prettier --write docs/KNOWLEDGE_INDEX.md` (repo dùng
   `proseWrap: always`). Re-scaffold giữ nguyên nội dung đã soạn, an toàn.

## VERIFY (bằng chứng cơ học)

- `_harness/bin/harness-cli knowledge check` exit `0` (không thiếu mục, không
  lệch cấu trúc so với cây thư mục hiện tại, không còn `TODO`).
- `npx prettier --check docs/KNOWLEDGE_INDEX.md` pass.

## ARTIFACTS (đầu ra)

- `docs/KNOWLEDGE_INDEX.md` (versioned).
- Trace GĐ5: ghi `"skill: generate-knowledge-index"` ở `--actions`/`--notes`.

## FRICTION HOOKS

- `IF [scaffold sinh tech sai/thiếu]` HOẶC `[mục cấu trúc gây nhiễu cần loại]`:
  ghi friction (GĐ5) + cân nhắc backlog (GĐ6) để chỉnh detection trong
  `harness-cli` thay vì sửa tay file.

## EXIT (tiêu chí xong)

- `knowledge check` xanh, prettier pass, file đã tạo/thay đổi; Purpose + Key
  Concepts do người soạn, mọi mục cấu trúc có mô tả.

> **Vì sao làm mới quan trọng (consume ↔ produce):** index này được ĐỌC đầu mỗi
> tác vụ như bản đồ Orient (xem ĐỊNH MỨC TOKEN ở `01-WORKFLOW.md`). Giới hạn
> freshness của `knowledge check` (đỏ/xanh nghĩa là gì, vì sao xanh CHƯA chắc
> tươi): xem nguồn duy nhất ở `00-AGENTS.md` §1. Vì vậy CHỦ ĐỘNG chạy skill này
> để `scaffold` + soạn lại phần ngữ nghĩa mỗi khi cấu trúc / tech stack đổi,
> đừng đợi `check` đỏ.
