# Skill: quality-gate-review

- **Trigger:** `IF [GĐ3 xong code logic, TRƯỚC khi sang GĐ4]` (bỏ qua nếu
  docs-only/tiny).
- **Lane:** normal | high-risk.
- **Giai đoạn:** Cổng GĐ3 → GĐ4.

Chạy 1 vòng review độc lập 3 lens rồi GATE. Self-check nội tuyến GĐ3 không thay
được lens này.

## INPUT (đọc trước khi chạy)

- Diff: `git status --short` + nội dung file đã đổi.
- `02-STANDARDS.md` (Dependency Rule, Parse-First, Command/Query, Test Matrix,
  Observability Contract).
- Story packet (`execplan.md`/`design.md` + Validation) + intake (Lane / Hard
  Gates đã đếm GĐ1).
- 11 Responsibilities (danh sách ở `01-WORKFLOW.md` GĐ5; mô tả sâu
  `_harness/docs/HARNESS_COMPONENTS.md`) để quy gán finding.

## STEPS

Mỗi lens: KHÔNG sửa code, chỉ xuất finding
`{severity: blocking|minor, responsibility}`. Chạy 3 lens ĐỘC LẬP, spawn 3
sub-agent song song; nếu tuần tự, KHÔNG để lens trước dẫn dắt lens sau.

1. **Lens 1 — Quality & Architecture:** `interface` gọi thẳng `database`? Input
   biên đã parse thành typed DTO/Command trước khi vào application/domain? Inner
   layer phụ thuộc outer layer? Command/Query lẫn lộn? Lệch `design.md`?
2. **Lens 2 — Security & Risk:** chạm Hard Gate nào (auth, authorization,
   data-loss/migration, audit/security, external provider, làm yếu validation)?
   Input biên chưa validate? Secret/credential lộ trong code/log? Hard Gate
   THIẾU decision record (`docs/decisions/NNNN-*.md`)?
3. **Lens 3 — Maintainability & Proof:** thiếu test tier Test Matrix cho hành vi
   mới? Thiếu test negative/failure path? Proof lệch claim? Naming/coupling/độ
   phức tạp cản phiên sau? Thiếu log JSON theo Observability Contract?

## VERIFY

- Chạy `harness-cli story verify <ID>` và ĐỌC log trước khi chốt Lens 3 (theo
  Cửa ải Bằng chứng GĐ4). Verify hợp nhất tại đây; GĐ4 tái dùng log nếu code
  chưa đổi.
- Mỗi finding `blocking`: HOẶC sửa code rồi `story verify` lại pass, HOẶC
  `harness-cli backlog add --predicted "<tác động>"`.
- **GATE:** còn finding `blocking` chưa xử lý → CẤM đánh proof `1` / CẤM sang
  GĐ4.

## ARTIFACTS

- Khối review (3 verdict + finding list) → trace GĐ5 `--notes`; ghi
  `"skill: quality-gate-review"` ở `--actions`/`--notes`.
- Finding `blocking` hoãn sửa → 1 `backlog` item (`--predicted`).
- Lỗi thật/luật thiếu →
  `--friction "... Attribution: <1 trong 11 Responsibilities>."`.

## FRICTION HOOKS

- Theo trigger friction GĐ5 (đặc biệt: phải suy đoán luật/nguồn-sự-thật thiếu,
  lỗi cùng loại lặp lại) → friction (GĐ5) + backlog (GĐ6).

## EXIT

- 3 lens đều có verdict.
- 0 finding `blocking` tồn đọng.
- `story verify <ID>` đã chạy + đã đọc log.
- Trace GĐ5 ghi `"skill: quality-gate-review"`.
