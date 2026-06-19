# Skill: quality-gate-review

- **Trigger:** `IF [GĐ3 xong code logic, TRƯỚC khi sang GĐ4]` (bỏ qua nếu
  docs-only/tiny).
- **Lane:** normal | high-risk.
- **Giai đoạn:** Cổng GĐ3 → GĐ4.

Chạy 1 vòng review độc lập 4 lens rồi GATE. Self-check nội tuyến GĐ3 không thay
được lens này.

## INPUT (đọc trước khi chạy)

- Diff: `git status --short` + nội dung file đã đổi.
- `02-STANDARDS.md` (Dependency Rule, Parse-First, Command/Query, Test Matrix,
  Observability Contract, §3 nấc thang Lazy Senior — Lens 4 là mặt review của
  nấc thang này).
- Story packet (`execplan.md`/`design.md` + Validation) + intake (Lane / Hard
  Gates đã đếm GĐ1).
- 11 Responsibilities (danh sách ở `01-WORKFLOW.md` GĐ5; mô tả sâu
  `_harness/docs/HARNESS_COMPONENTS.md`) để quy gán finding.

## STEPS

Mỗi lens: KHÔNG sửa code, chỉ xuất finding
`{severity: blocking|minor, responsibility}`. Chạy 4 lens ĐỘC LẬP, spawn 4
sub-agent song song; nếu tuần tự, KHÔNG để lens trước dẫn dắt lens sau.

1. **Lens 1 — Quality & Architecture:** `interface` gọi thẳng `database`? Input
   biên đã parse thành typed DTO/Command trước khi vào application/domain? Inner
   layer phụ thuộc outer layer? Command/Query lẫn lộn? Lệch `design.md`?
2. **Lens 2 — Security & Risk:** chạm Hard Gate nào (auth, authorization,
   data-loss/migration, audit/security, external provider, làm yếu validation)?
   Input biên chưa validate? Secret/credential lộ trong code/log? Hard Gate
   THIẾU decision record (`docs/decisions/NNNN-*.md`)?
3. **Lens 3 — Maintainability & Proof:** thiếu test tier Test Matrix cho hành vi
   mới? Thiếu test negative/failure path? Proof lệch claim? Naming/coupling cản
   phiên sau? Thiếu log JSON theo Observability Contract?
4. **Lens 4 — Code Economy:** soi DIFF tìm phức tạp THỪA — mặt review của nấc
   thang Lazy Senior (`02-STANDARDS.md §3`). Phạm vi CHỈ phức tạp/over-engineer;
   correctness/security thuộc Lens 1-2, bug/perf KHÔNG thuộc lens này. Mỗi finding
   một dòng: `L<line>: <tag> <cái cần cắt>. <cái thay thế>.` (đa-file:
   `<file>:L<line>: ...`). Tag:
   - `delete:` code chết / linh hoạt không dùng / feature đầu cơ → thay bằng không gì.
   - `stdlib:` tự cuộn thứ stdlib đã có → nêu tên hàm.
   - `native:` dep/code làm thứ nền tảng đã làm → nêu tính năng.
   - `yagni:` abstraction một implementation / config không ai set / layer một caller.
   - `shrink:` cùng logic, ít dòng hơn → nêu dạng ngắn.

   Chốt bằng MỘT metric: `net: -<N> lines possible.`; không có gì để cắt →
   `Lean already. Ship.`. KHÔNG cờ để xóa: layering theo Dependency Rule (§1) và
   một smoke/`assert` self-check (sàn proof §3) — đó là chuẩn, không phải bloat.

## VERIFY

- Chạy `harness-cli story verify <ID>` và ĐỌC log trước khi chốt Lens 3 (theo
  Cửa ải Bằng chứng GĐ4). Verify hợp nhất tại đây; GĐ4 tái dùng log nếu code
  chưa đổi.
- Mỗi finding `blocking`: HOẶC sửa code rồi `story verify` lại pass, HOẶC
  `harness-cli backlog add --predicted "<tác động>"`.
- **GATE:** còn finding `blocking` chưa xử lý → CẤM đánh proof `1` / CẤM sang
  GĐ4.

## ARTIFACTS

- Khối review (4 verdict + finding list + dòng `net:` của Lens 4) → trace GĐ5
  `--notes`; ghi `"skill: quality-gate-review"` ở `--actions`/`--notes`.
- Finding `blocking` hoãn sửa → 1 `backlog` item (`--predicted`).
- Lỗi thật/luật thiếu →
  `--friction "... Attribution: <1 trong 11 Responsibilities>."`.

## FRICTION HOOKS

- Theo trigger friction GĐ5 (đặc biệt: phải suy đoán luật/nguồn-sự-thật thiếu,
  lỗi cùng loại lặp lại) → friction (GĐ5) + backlog (GĐ6).

## EXIT

- 4 lens đều có verdict (Lens 4 có dòng `net: -N` hoặc `Lean already. Ship.`).
- 0 finding `blocking` tồn đọng.
- `story verify <ID>` đã chạy + đã đọc log.
- Trace GĐ5 ghi `"skill: quality-gate-review"`.
