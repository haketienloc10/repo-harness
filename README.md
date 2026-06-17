# repo-harness

Turn any software repo into an agent-ready workspace.

`repo-harness` is a repository-level operating harness for Claude Code,
Codex, Cursor, and other coding agents. It gives agents the missing project
context they need before they change code: where to start, what the product
contract says, how risky the work is, what proof is required, and which
decisions future agents should inherit.

The app is what users touch. The harness is what agents touch.

## Why Star This Repo

Star this repo if you want practical, reusable patterns for making AI-assisted
software development more reliable, inspectable, and easier for humans to steer.

This project is exploring a simple idea:

> Coding agents do not only need better prompts. They need better repositories.

## The Problem

Most repos are built for humans reading code in a familiar codebase. Coding
agents usually enter with only a chat prompt and a shallow snapshot of files.
That leads to common failure modes:

- The agent edits code before understanding product intent.
- Important constraints live only in chat history or in someone's head.
- Validation expectations are vague or discovered too late.
- Architecture tradeoffs are repeated instead of inherited.
- Large requests do not get broken into reviewable story-sized work.


# Documentation Map & Harness Onboarding

Dự án này sử dụng hệ thống **Harness**. Repo có **hai luồng tài liệu** song
song, phục vụ hai đối tượng khác nhau:

- **Luồng Agent (`_harness/`)** — bộ khung thực thi, văn phong mệnh lệnh, dành
  cho AI Agent đọc khi làm việc. Điểm vào: `AGENTS.md` →
  `_harness/00-AGENTS.md` → `_harness/01-WORKFLOW.md`, kèm
  `_harness/02-STANDARDS.md` (chuẩn kiến trúc) và `_harness/03-CLI_REFERENCE.md`
  (cú pháp CLI).
- **Luồng Người đọc (`docs/`)** — tài liệu tham chiếu sâu: chính sách, lý do,
  taxonomy, maturity, glossary. Dành cho con người và khi cần tra cứu chi tiết.
  Bắt đầu ở `_harness/docs/README.md` và `_harness/docs/HARNESS.md`.

Nếu bạn là **con người (Developer)**, README này là điểm xuất phát để hiểu cách
phối hợp với Agent. Nếu bạn là **Agent**, hãy đọc `_harness/00-AGENTS.md` và
`_harness/01-WORKFLOW.md`.

## 1. Sơ đồ Tư duy (Mental Model)

Mọi tác vụ trong dự án này đều đi qua một luồng khép kín từ lúc bạn (con người) đưa ra yêu cầu (Intent) cho đến khi ra kết quả.

```text
+------------------+
| Human intent    |
+------------------+
         |
         v
+------------------+
| Feature intake   | (Phân loại tự động rủi ro: Tiny/Normal/High-risk)
+------------------+
         |
         v
+------------------+
| Story packet     |
+------------------+
         |
         v
+------------------+
| Agent work loop  |
+------------------+
         |
         v
+------------------+
| Product delta    | ---> Code, Test, Tài liệu sản phẩm
+------------------+
         |
         v
+------------------+
| Validation proof | ---> Unit, Integration, E2E tests
+------------------+
         |
         v
+------------------+
| Harness delta    | ---> Cập nhật quy trình, Backlog, Cấu trúc
+------------------+
         |
         v
+------------------+
| Next intent      |
+------------------+

```

## 2. Lưu trữ Bền vững (Durable Layer)

Chính sách và cách làm việc được viết ở các file Markdown.
Nhưng dữ liệu vận hành thực tế (Tiến độ Story, Lịch sử Trace, Backlog) **được lưu trong cơ sở dữ liệu SQLite cục bộ**.
Hãy sử dụng CLI do dự án cung cấp:

* macOS/Linux: `_harness/bin/harness-cli`
* Windows: `_harness/bin/harness-cli.exe`

Ví dụ để kiểm tra trạng thái tiến độ chung, hãy chạy:
`_harness/bin/harness-cli query matrix`

Cú pháp mọi lệnh nằm ở `_harness/03-CLI_REFERENCE.md` (cheatsheet); ngữ nghĩa
và ví dụ đầy đủ ở `_harness/docs/CLI_REFERENCE.md`.

## 3. Hệ thống Trace (Ghi vết)

Mỗi khi Agent hoàn thành tác vụ, nó sẽ để lại một "Trace" (dấu vết). Đây là cơ sở để đánh giá năng lực của AI và tìm ra các điểm nghẽn (Friction) trong quy trình.

### Ví dụ về Trace Tốt (Tier Detailed)

Dùng cho công việc rủi ro cao (High-risk). Chứa đầy đủ bối cảnh, các trường danh
sách phân tách bằng dấu phẩy (KHÔNG dùng mảng JSON) và ghi nhận chính xác
lỗi/điểm nghẽn:

```bash
_harness/bin/harness-cli trace \
  --summary "Completed high-risk auth role migration with audit proof" \
  --intake 51 \
  --story US-014 \
  --agent codex \
  --outcome completed \
  --duration 4200 \
  --tokens 52000 \
  --actions "read access-control docs,created migration,updated audit tests" \
  --read "docs/product/permissions.md,docs/decisions/0008-auth-boundary.md" \
  --changed "src/auth/roles.ts,tests/auth-roles.test.ts" \
  --decisions "kept manager role scoped to workspace" \
  --errors "none" \
  --friction "Existing permission docs did not define delegated admin; added backlog item." \
  --notes "Detailed trace required because the task touched authorization."
```

### Ví dụ về Trace Kém (Không được chấp nhận)

Trace này vô giá trị cho việc đo lường và bàn giao:

```bash
_harness/bin/harness-cli trace \
  --summary "did phase 2" \
  --outcome completed

```

*(Lý do kém: Không có actions, không liệt kê file đã đọc/sửa, không liên kết story, thiếu friction signal)*.

---

## Install Harness Into A Project

From a target project directory, run:

```bash
curl -fsSL "https://raw.githubusercontent.com/haketienloc10/repo-harness/main/install.sh?$(date +%s)" | bash
```