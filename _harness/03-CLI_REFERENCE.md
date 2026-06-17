# Tra cứu lệnh Harness CLI (Cheatsheet)

Trạng thái vận hành (intake, story, decision, backlog, trace) sống trong
`harness.db`, thao tác qua `_harness/bin/harness-cli` (macOS/Linux) hoặc
`_harness/bin/harness-cli.exe` (Windows) — dưới đây viết tắt `harness-cli`. KHÔNG
sửa tay `harness.db`. Schema: `_harness/schema/`.

File này là CHEATSHEET (đủ cú pháp + quy ước cho mọi lệnh). Tra cứu thêm theo
thứ tự: (1) flag đầy đủ của một lệnh → `harness-cli <cmd> --help`; (2) ngữ
nghĩa, ví dụ, gotchas → `_harness/docs/CLI_REFERENCE.md` (on-demand, CHỈ đọc mục cần).

## Quy ước giá trị

- **Input type** (`--type`): `new spec`, `spec slice`, `change request`,
  `new initiative`, `maintenance request`, `harness improvement`.
- **Lane / risk** (`--lane`, `--risk`): `tiny`, `normal`, `high-risk`. `low`
  KHÔNG hợp lệ.
- **Outcome** (`--outcome`): `completed`, `blocked`, `partial`, `failed`.
- **Proof booleans** (`--unit/--integration/--e2e/--platform`): dùng SỐ `1`/`0`.
  CLI từ chối chữ `yes`/`no`.
- **Trường danh sách của trace** (`--actions/--read/--changed/--decisions`...):
  chuỗi phân tách bằng DẤU PHẨY, KHÔNG dùng ngoặc vuông JSON. Dùng `none` khi
  rỗng.

## Cú pháp lệnh

```bash
# Setup
harness-cli init | migrate | import brownfield | --version

# GĐ1 — Intake (--flags lưu bằng chứng lane; in ra intake_id để link GĐ4/GĐ5)
harness-cli intake --type <type> --summary "<text>" --lane <lane> [--flags "f1,f2"] [--docs "d1,d2"] [--story <id>]

# GĐ2 — Story & Decision (BẮT BUỘC `story add` trước khi `story update/verify`)
harness-cli story add --id <id> --title "<text>" --lane <lane> [--verify "<cmd>"] [--contract "<text>"]
harness-cli decision add --id <id> --title "<text>" --doc docs/decisions/<file>.md [--verify "<cmd>"] [--predicted "<impact>"] [--notes "<text>"]

# GĐ4 — Verify & Proof
harness-cli story verify <id> [--no-capture]   # exit 0=pass / 1=fail; tự ghi log vào evidence (default-on)
harness-cli story verify-all         # batch; exit 1 nếu BẤT KỲ story nào fail
harness-cli story update --id <id> [--status <status>] [--unit 1 --integration 1 --e2e 0 --platform 0] [--verify "<cmd>"] [--next-action "<text>"]
harness-cli decision verify <id>

# GĐ4/5 — Evidence (con trỏ artifact bền vững; file nằm dưới _harness/evidence/ gitignored)
harness-cli evidence add --kind <log|diff|screenshot|report|file> --path <p> \
  [--story <id>] [--trace <id>] [--command "<cmd>"] [--source <agent|human|ci|reviewer>] [--notes "<t>"]
harness-cli evidence list [--story <id>] [--trace <id>] [--kind <k>] [--json]

# GĐ5 — Trace & Intervention
harness-cli trace --summary "<text>" --outcome <outcome> \
  [--intake <id> --story <id> --agent <name> --duration <s> --tokens <n> \
   --actions "a,b" --read "f1,f2" --changed "f1,f2" --decisions "d1,d2" \
   --errors "none" --friction "Mô tả. Attribution: <nguồn>." --notes "<text>" \
   --next-action "<text>"]   # BẮT BUỘC khi outcome ∈ {partial,blocked,failed}; completed tự clear story.next_action
harness-cli intervention add --type <correction|override|escalation|approval> --description "<text>" --source <human|reviewer|ci|agent> [--trace <id>] [--story <id>] [--impact "<text>"]
harness-cli score-trace --id <id>    # chấm lại trace lịch sử (điểm in sẵn sau `trace`)
harness-cli score-context <trace-id> # advisory: đối chiếu files_read với context rules

# GĐ6 — Backlog & Improvement
harness-cli backlog add --title "<text>" --pain "<text>" --risk <lane> --predicted "<impact>"
harness-cli backlog close --id <id> --outcome "<actual>"
harness-cli audit                    # drift + entropy score (thấp là tốt)
harness-cli propose [--commit]       # --commit CHỈ tạo backlog `proposed`, người duyệt

# GĐ7 — Done-check gate (lane-aware; exit 0=tất cả pass, 1=bất kỳ fail)
harness-cli done-check [--story <id>] [--intake <id>] [--json]

# Knowledge & Tool Registry
harness-cli knowledge scaffold       # sau đó: npx prettier --write docs/KNOWLEDGE_INDEX.md
harness-cli knowledge check          # cổng cơ học; exit != 0 nếu lỗi
harness-cli tool register --name <n> --command <cmd> --description "<10-200 ký tự>" --responsibility <1 trong 11 Responsibilities — xem 01-WORKFLOW GĐ5> \
    [--kind cli|binary|mcp|skill|http] [--capability <kebab-case>] [--scan <path|url>] [--args "name:type:required[:help]"] [--force]
harness-cli tool check [--name <name>] [--json]   # quét present/missing/unknown + ghi checked_at
harness-cli tool remove --name <name>

# Query
harness-cli query status [--json] [--lane <lane>] [--limit <n>] [--full]   # Read-Model: đang/đã/cần làm gì (≤~1k token)
harness-cli query recap [--story <id>] [--epic <prefix>] [--since <YYYY-MM-DD>] [--json]  # rollup tất định theo trace
harness-cli query matrix [--numeric]       # proof map; --numeric để copy vào story update
harness-cli query backlog --open|--closed
harness-cli query intakes | decisions | traces | friction | stats
harness-cli query interventions [--story <id>|--trace <id>|--type <type>]
harness-cli query tools [--summary|--json|--responsibility <R>|--capability <cap>|--status present|missing|unknown]
harness-cli query sql "<SQL>"              # SQL thô (đọc)
```

## Gotchas bắt buộc

- `story verify-all`: BẮT BUỘC chạy trước merge, trước claim maturity (H4+), và
  trước benchmark run.
- Decision đụng auth/authorization/data ownership/API shape/audit-security/
  validation: ghi Ở CẢ HAI nơi — file `docs/decisions/*.md` VÀ `decision add`.
  Trường `--decisions` của trace KHÔNG thay thế bản ghi durable.
- Độ sâu trường trace theo tier (lane): xem `_harness/docs/TRACE_SPEC.md`.
