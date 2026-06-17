# Harness CLI — Tham chiếu đầy đủ

> Đây là bản tham chiếu SÂU, đọc on-demand theo từng mục khi cần ngữ nghĩa, ví
> dụ đầy đủ, hoặc gotchas của một lệnh. Bản cheatsheet luôn-được-đọc (quy ước
> giá trị + cú pháp mọi lệnh) nằm ở `_harness/03-CLI_REFERENCE.md`. Chi tiết
> flag của một lệnh bất kỳ: `harness-cli <cmd> --help`.

Trạng thái vận hành (intake, story, decision, backlog, trace) sống trong
`harness.db` và được thao tác qua `_harness/bin/harness-cli` (macOS/Linux) hoặc
`_harness/bin/harness-cli.exe` (Windows). Agent và người PHẢI dùng binary này cho
mọi việc Harness; KHÔNG sửa tay `harness.db`. Schema nằm dưới `_harness/schema/`.

## Quy ước giá trị

- **Input type** (`--type`): `new spec`, `spec slice`, `change request`,
  `new initiative`, `maintenance request`, `harness improvement`.
- **Lane / risk** (`--lane`, `--risk`): `tiny`, `normal`, `high-risk`. `low`
  KHÔNG hợp lệ.
- **Outcome** (`--outcome`): `completed`, `blocked`, `partial`, `failed`.
- **Proof booleans** (`--unit/--integration/--e2e/--platform`): dùng SỐ `1`/`0`
  (`1` = yes, `0` = no). CLI từ chối chữ `yes`/`no`.
- **Trường danh sách của trace** (`--actions/--read/--changed/--decisions`...):
  chuỗi phân tách bằng DẤU PHẨY, KHÔNG dùng ngoặc vuông JSON. Dùng `none` khi
  rỗng.

## 1. Setup

```bash
_harness/bin/harness-cli init        # Khởi tạo harness.db nếu chưa có
_harness/bin/harness-cli migrate     # Áp các schema migration còn thiếu
_harness/bin/harness-cli import brownfield  # Seed durable records từ state markdown
_harness/bin/harness-cli --version   # Xem phiên bản CLI
```

- `migrate` áp các file dưới `_harness/schema/` chưa chạy. Chạy sau khi cập nhật
  binary hoặc thêm schema mới.
- `import brownfield` chỉ dùng khi khởi tạo durable layer từ docs có sẵn.

## 2. Intake (phân loại đầu vào)

```bash
# --flags lưu các Risk Flag đã đếm (CSV) → bằng chứng đằng sau lane
_harness/bin/harness-cli intake --type <type> --summary "<text>" --lane <lane> \
  --flags "auth,data-model,..."
```

## 3. Story & Verify

```bash
# Thêm story (BẮT BUỘC chạy TRƯỚC story update/verify — CLI báo lỗi
# `story '<id>' not found` nếu row chưa tồn tại; có thể gắn proof bằng --verify)
_harness/bin/harness-cli story add --id <id> --title "<text>" --lane <lane> \
  --verify "<command>"

# Cập nhật status
_harness/bin/harness-cli story update --id <id> --status <status>

# Cập nhật proof booleans (số 1/0)
_harness/bin/harness-cli story update --id <id> \
  --unit 1 --integration 1 --e2e 0 --platform 0

# Cấu hình / đổi lệnh verify
_harness/bin/harness-cli story update --id <id> --verify "<command>"

# Chạy verify (chỉ nhận story id) — tự ghi log vào evidence store (default-on)
_harness/bin/harness-cli story verify <id>
_harness/bin/harness-cli story verify <id> --no-capture   # bỏ ghi evidence (lần nháp)

# Chạy TẤT CẢ verify command đã cấu hình (batch) — dùng trước merge / claim maturity / benchmark
_harness/bin/harness-cli story verify-all
```

- `story verify` chạy lệnh từ gốc repo, ghi `last_verified_at` +
  `last_verified_result`, thoát `0` nếu pass / `1` nếu fail.
- **Auto-capture (default-on):** mỗi lần `story verify` tự ghi stdout+stderr vào
  evidence store (`kind='log'`, kèm `command` + `result`) và in evidence id.
  Dedup keep-last theo `(story, kind, result)` + sha256, NGOẠI LỆ `fail→pass`
  giữ cả hai. ⇒ proof boolean `1` luôn có log tươi đỡ lưng. `--no-capture` chỉ
  cho lần chạy nháp. Xem mục 5b (Evidence) và decision 0002.
- `story verify-all` chạy mọi story có `verify_command`, in 1 kết quả/story, BỎ
  QUA story không cấu hình verify, và thoát `1` nếu BẤT KỲ story nào fail. BẮT
  BUỘC chạy trước khi merge, trước khi claim maturity (H4+), và trước benchmark
  run.
- Khi `trace --story <id>` trỏ tới story có lệnh verify CHƯA từng pass, trace
  vẫn ghi nhưng in cảnh báo trước khi đóng.
- Lấy giá trị proof để copy ngược vào `story update`: dùng
  `query matrix --numeric` (xem mục 6).

## 4. Decision (quyết định)

```bash
_harness/bin/harness-cli decision add \
  --id <id> \
  --title "<text>" \
  --doc docs/decisions/<file>.md \
  --notes "<notes>"
```

- High-risk đụng auth, authorization, sở hữu dữ liệu, API shape, audit/security,
  hoặc validation: ghi quyết định ở CẢ hai nơi — file markdown dưới
  `docs/decisions/` (từ `_harness/docs/templates/decision.md`) VÀ bản ghi durable ở trên.
- Trường `--decisions` của trace chỉ là bằng chứng, KHÔNG thay cho bản ghi
  decision durable.
- Có thể gắn lệnh verify cho decision và chạy:

```bash
_harness/bin/harness-cli decision add --id <id> --title "<text>" --verify "<command>"
_harness/bin/harness-cli decision verify <id>
```

## 5. Trace (ghi vết thực thi)

```bash
_harness/bin/harness-cli trace \
  --summary "<text>" \
  --intake <id> \
  --story <id> \
  --agent <name> \
  --outcome <outcome> \
  --duration <seconds> \
  --tokens <estimate> \
  --actions "action1,action2" \
  --read "file1,file2" \
  --changed "file1,file2" \
  --decisions "decision1,decision2" \
  --errors "none" \
  --friction "Mô tả. Attribution: <nguồn>." \
  --notes "<text>" \
  --next-action "<việc kế tiếp>"
```

- Độ sâu trường theo tier (lane) — xem `_harness/docs/TRACE_SPEC.md`. Lane càng cao,
  trace càng phải đầy đủ (actions, read, changed, intake/story, friction).
- **`--next-action` (resume continuity):** BẮT BUỘC khi `--outcome` ∈
  `{partial, blocked, failed}` (CLI từ chối rỗng, exit `!= 0`). Nếu trace có
  `--story`, hint ghi luôn vào `story.next_action` và nổi lên ở RESUME của
  `query status`. `--outcome completed` + `--story` ⇒ tự clear
  `story.next_action`. Đặt con trỏ WIP sống trực tiếp:
  `story update --id <id> --next-action "<text>"`.
- Xem điểm trace in tự động ngay sau `trace`. Chỉ dùng `score-trace --id <id>`
  khi cần chấm lại một trace lịch sử cụ thể:

```bash
_harness/bin/harness-cli score-trace --id <id>
```

- `score-context <trace-id>` (advisory): chấm `files_read` của trace so với
  context rules đã biên dịch (`_harness/docs/CONTEXT_RULES.md`). KHÔNG đổi trace; chỉ báo
  coverage để biết thiếu ngữ cảnh gì.

```bash
_harness/bin/harness-cli score-context <trace-id>
```

## 5b. Evidence (con trỏ artifact bền vững)

Db giữ pointer + sha256 + digest; artifact thô nằm dưới `_harness/evidence/`
(gitignored). Tên file content-addressed (`<kind>-<sha16><ext>`). Xem decision
0002.

```bash
# Ghi một artifact (hash + copy vào store + chèn con trỏ)
_harness/bin/harness-cli evidence add --kind log --path run.log --story US-003 \
  --command "cargo test" --source agent --notes "manual sample"

# Liệt kê (lọc theo story/trace/kind; --json để máy đọc)
_harness/bin/harness-cli evidence list --story US-003 --json
```

- `--kind` ∈ `log|diff|screenshot|report|file`. Text kind (`log/diff/report`)
  sinh digest head+tail (20+20 dòng); binary (`screenshot/file`) digest =
  metadata `<kind>/<bytes>/<ext>`.
- `--source` ∈ `agent|human|ci|reviewer` (default `agent`).
- Phải có `--story` HOẶC `--trace` để neo artifact.
- Content dedup theo `(story/trace, kind, sha256)`: artifact y hệt không tạo
  file/row mới, chỉ làm tươi `created_at`.
- `result` (`pass/fail`) chỉ tự set khi `story verify` auto-capture (mục 3); với
  `evidence add` thủ công thì để trống.

## 6. Query (truy vấn durable layer)

```bash
_harness/bin/harness-cli query status             # Read-Model: đang/đã/cần làm gì (mục 6b)
_harness/bin/harness-cli query recap --story <id>  # Rollup trace tất định (mục 6b)
_harness/bin/harness-cli query matrix             # Proof map dạng yes/no
_harness/bin/harness-cli query matrix --numeric   # Dạng 1/0 để copy vào update
_harness/bin/harness-cli query backlog --open     # Item proposed/accepted
_harness/bin/harness-cli query backlog --closed   # So predicted với outcome
_harness/bin/harness-cli query intakes            # Intake gần đây
_harness/bin/harness-cli query decisions          # Bản ghi decision durable
_harness/bin/harness-cli query traces             # Danh sách trace đã ghi
_harness/bin/harness-cli query friction           # Ma sát theo từng task
_harness/bin/harness-cli query tools --summary    # Tool registry (xem mục 9)
_harness/bin/harness-cli query interventions      # Can thiệp đã ghi (xem mục 10)
_harness/bin/harness-cli query stats              # Thống kê tổng quan
_harness/bin/harness-cli query sql "<SQL>"        # SQL thô trên harness.db (đọc)
```

## 6b. Read-Model: `query status` & `query recap`

VIEW dẫn xuất tất định — KHÔNG có schema riêng (xem decision 0001). Dùng ở GĐ0
khi CHẠY workflow 7-GĐ (chung predicate với Execution Tracker, `00-AGENTS.md`
§3).

```bash
# Session brief: ranked theo ưu tiên hành động (≤ ~1k token)
_harness/bin/harness-cli query status
_harness/bin/harness-cli query status --lane high-risk --limit 3
_harness/bin/harness-cli query status --full        # bỏ trần dòng
_harness/bin/harness-cli query status --json        # cho host/agent parse

# Rollup trace tất định (count/group, KHÔNG tóm tắt ngữ nghĩa)
_harness/bin/harness-cli query recap --story US-003
_harness/bin/harness-cli query recap --epic US-00 --since 2026-06-01 --json
```

- `query status` sections (theo thứ tự ưu tiên): ĐANG LÀM (in_progress) · CẦN
  PROOF (implemented chưa pass) · RESUME (partial/blocked/failed + next_action) ·
  BACKLOG MỞ (high-risk trước) · INTERVENTION gần đây · HOẠT ĐỘNG GẦN. Header
  có drift entropy. Mỗi section có trần `--limit` (mặc định 5) và in `(+N nữa)`
  — "no silent caps". `--lane` lọc các section dẫn-xuất-từ-story.
- `query recap` gom: outcome counts, files đụng (top theo tần suất), Friction
  theo 11 Component (`_harness/docs/HARNESS_COMPONENTS.md`), decisions, intervention
  counts. `--epic` khớp PREFIX của story id (vd `US-00` khớp US-001..US-009).
  Tất định: cùng db → cùng output.

## 7. Backlog (vòng cải tiến từ friction)

```bash
# Thêm đề xuất cải tiến
_harness/bin/harness-cli backlog add \
  --title "<short name>" \
  --pain "<what was hard>" \
  --risk <tiny|normal|high-risk> \
  --predicted "<measurable impact>"

# Đóng item kèm kết quả thực đo
_harness/bin/harness-cli backlog close --id <id> --outcome "<actual result>"
```

- Outcome loop: điền `--predicted` lúc tạo (tác động kỳ vọng), điền `--outcome`
  lúc đóng (kết quả đo thực / bằng chứng review), rồi đối chiếu bằng
  `query backlog --open` và `query backlog --closed`.

## 8. Knowledge (bản đồ onboarding repo)

```bash
# Tạo/làm mới docs/KNOWLEDGE_INDEX.md (regenerate Key Technologies, How to Run,
# Top-Level Structure, Key Subdirectories; giữ nguyên Purpose/Key Concepts và
# mô tả đã soạn giữa các marker)
_harness/bin/harness-cli knowledge scaffold

# Cổng cơ học: file có đủ mục, không lệch cấu trúc, không còn TODO (exit != 0 nếu lỗi)
_harness/bin/harness-cli knowledge check
```

- Phần tất định (Key Technologies, How to Run, Top-Level Structure, Key
  Subdirectories) do CLI sinh; phần ngữ nghĩa (Purpose, Key Concepts) do
  người/agent soạn và được giữ lại. Key Technologies nhận thêm framework và
  package manager bằng cách đọc nội dung manifest; How to Run rút lệnh
  build/test từ manifest (tất định). Quy trình đầy đủ ở
  `_harness/skills/generate-knowledge-index.md`.
- Sau `scaffold` luôn chạy `npx prettier --write docs/KNOWLEDGE_INDEX.md` (repo
  dùng `proseWrap: always`); round-trip là idempotent.

## 9. Tool Registry (inbound vs outbound; khám phá + đăng ký công cụ)

Hai loại "tool" tách biệt — chi tiết + Degrade Ladder ở `_harness/docs/TOOL_REGISTRY.md`:

- **Outbound (capability manifest):** các subcommand `harness-cli`, luôn compile
  sẵn — chính là harness.
- **Inbound registry:** công cụ do dự án trang bị để harness _dùng_ (linter,
  code-graph, deploy-check). Tùy chọn; thiếu thì skip sạch, KHÔNG chặn core.

```bash
# Xem công cụ compiled + đã đăng ký
_harness/bin/harness-cli query tools --summary
_harness/bin/harness-cli query tools --json
_harness/bin/harness-cli query tools --responsibility Verification

# Tra theo CAPABILITY (cách một step nên hỏi: "có gì cho mục đích này?")
_harness/bin/harness-cli query tools --capability impact-analysis --status present

# Đăng ký công cụ inbound như provider của một capability
_harness/bin/harness-cli tool register \
  --name deploy-check \
  --kind cli \
  --capability deploy-verification \
  --command ./scripts/deploy-check.sh \
  --description "<10-200 ký tự>" \
  --responsibility Verification \
  --args "env:enum:required:staging,production"

# Đăng ký MCP server / Claude skill (không nằm trên PATH; presence resolve sau)
_harness/bin/harness-cli tool register --name gitnexus --kind mcp \
  --capability impact-analysis --scan ".gitnexus" --command "mcp:gitnexus" \
  --description "Code-graph blast radius" --responsibility Verification

# Quét hiện diện → ghi status (present/missing/unknown) + checked_at
_harness/bin/harness-cli tool check            # quét tất cả
_harness/bin/harness-cli tool check --name c3  # quét một
_harness/bin/harness-cli tool check --json     # đọc được bằng máy

# Gỡ công cụ
_harness/bin/harness-cli tool remove --name <name>
```

- Tên tool phải duy nhất; `--description` 10-200 ký tự; `--responsibility` phải
  thuộc danh sách Runtime Substrate (`_harness/docs/HARNESS_COMPONENTS.md`).
- `--kind` ∈ `cli|binary|mcp|skill|http` (default `cli`). `cli`/`binary` được
  exec-probe trên PATH; `mcp`/`skill`/`http` quét qua `--scan` (path/URL).
- `--capability` chuẩn hóa kebab-case (space/`_` → `-`); là **coupling duy
  nhất** giữa step và tool — step tham chiếu capability, KHÔNG tham chiếu tên
  tool.
- `--command` (cho `cli`/`binary`) phải tồn tại trên PATH hoặc là đường dẫn; chỉ
  dùng `--force` khi cố tình chưa có. `mcp`/`skill`/`http` bỏ qua kiểm tra này.
- `--args` theo mẫu `name:type:required` hoặc `name:type:required:help`.
- `tool check` luôn exit `0`: công cụ vắng mặt là _fact_ để báo, không phải lỗi.
  Agent áp policy (skip/degrade/use) dựa trên `status` — xem Degrade Ladder.

## 10. Intervention (ghi can thiệp — tách khỏi trace)

Ghi khi human / reviewer / CI / agent khác **sửa, ghi đè, leo thang, hoặc
duyệt** công việc. Interventions lưu riêng trace và là đầu vào cho `propose`
(mục 11).

```bash
_harness/bin/harness-cli intervention add \
  --trace <id> \
  --type correction \
  --description "<text>" \
  --source human \
  --story <id> \
  --impact "<text>"

_harness/bin/harness-cli query interventions --story <id>
_harness/bin/harness-cli query interventions --trace <id>
_harness/bin/harness-cli query interventions --type correction
```

## 11. Audit & Improvement (drift + tự cải tiến)

Vòng tự cải tiến: `friction + interventions + audit -> propose -> backlog`. Xem
`_harness/docs/HARNESS_AUDIT.md`, `_harness/docs/IMPROVEMENT_PROTOCOL.md`.

```bash
# Drift audit: in từng nhóm lệch + điểm entropy (thấp là tốt, cap 100)
_harness/bin/harness-cli audit

# Sinh đề xuất cải tiến tất định từ friction/intervention/audit (advisory)
_harness/bin/harness-cli propose

# Chốt đề xuất thành backlog item `proposed` (KHÔNG sửa policy/duyệt hộ)
_harness/bin/harness-cli propose --commit
```

- `audit` chấm: orphaned/unverified stories, unverified decisions, backlog thiếu
  outcome, story stale, broken tools (xem bảng trọng số ở
  `_harness/docs/HARNESS_AUDIT.md`).
- `propose` tất định, evidence-backed; mỗi đề xuất gồm component, evidence,
  predicted impact, risk, suggested action, validation plan, confidence.
- `--commit` chỉ tạo backlog item `proposed`; con người vẫn là cổng duyệt
  (review qua `query backlog --open`).

## 12. Done-check (cổng hoàn thành GĐ7, lane-aware)

Aggregator các check sẵn có + evidence/next-action thành một gate có exit code.
KHÔNG thêm store. Dùng ở GĐ7 thay cho tự khẳng định "done".

```bash
_harness/bin/harness-cli done-check --story <id>     # exit 0=tất cả pass, 1=bất kỳ fail
_harness/bin/harness-cli done-check --intake <id>    # resolve story qua intake.story_id
_harness/bin/harness-cli done-check --story <id> --json
```

- In checklist `✔/✘` từng dòng kèm lý do; `--json` trả `{passed, checks:[...]}`.
- Check theo lane: **mọi lane** cần ≥1 trace liên kết story/intake.
  **normal/high-risk** thêm: `status=='implemented'`, `verify_command` set &
  `last_verified_result=='pass'`, có evidence `log` pass, ≥1 proof flag = 1,
  `story.next_action` đã clear. **high-risk** thêm: 4 neo packet
  (overview/execplan/design/validation) tồn tại cạnh `contract_doc`.
