# Trục Xương Sống: Quy trình 7 Giai đoạn (Harness Workflow)

## ĐỊNH MỨC TOKEN (Context Budget)

- **Phạm vi định mức (đọc trước khi áp số):** Định mức lane bên dưới CHỈ tính
  phần context BIẾN ĐỔI theo tác vụ (intake docs, product/story/decision docs,
  templates, file cần sửa). Tầng nền cố định — `00-AGENTS.md`, `01-WORKFLOW.md`,
  `docs/KNOWLEDGE_INDEX.md`, `03-CLI_REFERENCE.md` (cheatsheet) — là chi phí
  hằng (~9–10k token) cho MỌI lane, KHÔNG trừ vào định mức. (Bản guidance phía
  người đọc: `docs/CONTEXT_RULES.md` §Token Budget Guidance — giữ hai nơi này
  khớp nhau khi sửa.)
- **Tài liệu dùng chung (Luôn có thể truy xuất):** Bất cứ khi nào cần tương tác
  với `harness.db`, Agent luôn được phép đọc `_harness/03-CLI_REFERENCE.md`
  (cheatsheet gọn) để lấy cú pháp; chi tiết sâu hơn nằm ở
  `docs/CLI_REFERENCE.md` và `harness-cli <cmd> --help` — chỉ tra on-demand,
  KHÔNG preload. Để biết lệnh/công cụ nào đang có, dùng
  `harness-cli query tools --summary` (tool registry, xem
  `docs/TOOL_REGISTRY.md`) thay vì đoán.
- **Skill (nạp on-demand):** KHÔNG preload `skills/*`. Tới giai đoạn có trigger
  khớp trong registry `_harness/04-SKILLS.md`, mới đọc ĐÚNG file skill đó.
- **Tầng nền (MỌI lane, đọc ĐẦU TIÊN):** `docs/KNOWLEDGE_INDEX.md` — bản đồ
  onboarding cô đọng (ROUTER, không phải nguồn sự thật; xem `00-AGENTS.md` §1).
  Rẻ hơn crawl `docs/`; dùng nó để chọn đúng file cần đọc tiếp cho lane hiện
  tại. Giới hạn freshness của `knowledge check`: xem nguồn duy nhất ở
  `00-AGENTS.md` §1.
- **Tiny Lane:** ~2,000 tokens biến đổi. Ngoài tầng nền: đọc intake docs, matrix
  query, và file cần sửa.
- **Normal Lane:** ~5,000 tokens biến đổi. Đọc thêm product/story docs liên
  quan, architecture (nếu cần đổi cấu trúc), và validation expectations.
- **High-Risk Lane:** ~10,000 tokens biến đổi. Đọc toàn bộ intake, architecture,
  quyết định liên quan, templates rủi ro cao.

---

## GIAI ĐOẠN 1: INTAKE (Phân loại)

- **0. Orient (đọc TRƯỚC khi phân loại):** Đọc `docs/KNOWLEDGE_INDEX.md` để nắm
  Purpose + Top-Level Structure của repo trước khi chọn Type và đếm Risk Flags
  (hiểu repo giúp phân loại đúng). Giới hạn freshness của `knowledge check`
  (đỏ/xanh nghĩa là gì): xem nguồn duy nhất ở `00-AGENTS.md` §1.
- **1. Chọn Type:** `New spec`, `Spec slice`, `Change request`,
  `New initiative`, `Maintenance request`, `Harness improvement`.
  - **Map Type → Đích đến (artifact):** `New spec` → `docs/product/*` +
    candidate epics + decisions; `Spec slice` → 1 story packet; `Change request`
    → story packet hoặc patch; `New initiative` → initiative note + nhiều story;
    `Maintenance request` → story / validation / decision; `Harness improvement`
    → cập nhật docs hoặc backlog.
- **2. Đếm Rủi ro (Risk Flags):** (1) Auth, (2) Authorization, (3) Data model,
  (4) Audit/security, (5) External systems, (6) Public contracts, (7)
  Cross-platform, (8) Existing behavior, (9) Weak proof, (10) Multi-domain.
- **3. Hard Gates (Rào cản cứng):** Auth, Authorization, Data loss/migration,
  Audit/security, External provider, Làm yếu validation.
- **4. Thuật toán Lane:**
  - `IF` [Dính >= 1 Hard Gate] HOẶC [>= 4 Flags]: **Lane = high-risk** (NGOẠI
    LỆ: nếu con người chủ động thu hẹp phạm vi rõ ràng, được phép hạ lane).
  - `IF` [2-3 Flags]: **Lane = normal**.
  - `IF` [0-1 Flags] VÀ [Sửa docs/copy/setup cơ bản]: **Lane = tiny**.
  - `IF` [0-1 Flags] VÀ [Đổi logic code]: **Lane = normal**.
  - **[Lưu ý setup/health]:** Việc setup ban đầu hoặc thêm health/smoke endpoint
    là _smoke proof_, KHÔNG tự động tính là cờ "Public contracts" → đừng leo
    thang lane chỉ vì health endpoint.
- **5. Hành động:** Chạy
  `harness-cli intake --type "<loại>" --summary "<text>" --lane <lane>`.
- **[Quy tắc cấm]:** KHÔNG ĐƯỢC tạo hoặc mở rộng một file `SPEC.md` nguyên khối.
  Mọi thay đổi phải được xé nhỏ vào `docs/product/` và `docs/stories/`.

---

## GIAI ĐOẠN 2: PLANNING (Lập kế hoạch - DOCS FIRST)

- **Retrieval Triggers (Kích hoạt lấy Context):**
  - `IF` chạm database schema, durable records, migrations: Đọc
    `scripts/schema/`.
  - `IF` chạm CLI/installer: Đọc `crates/harness-cli/*`.
  - `IF` liên quan đến maturity, benchmark, trace quality: tra cứu tài liệu tham
    chiếu sâu trong `docs/*` (xem `00-AGENTS.md` §1).
  - `IF` đổi public API shape / hành vi người dùng: Đọc `docs/product/*` và
    story liên quan TRƯỚC khi sửa.
  - `IF` phát hiện doc/record cũ, mâu thuẫn, hoặc lặp lại nhầm lẫn: Ghi
    `friction` (GĐ5) và cân nhắc thêm backlog.
  - `IF` một bước CÓ THỂ dùng công cụ ngoài (linter, code-graph, deploy-check):
    tra theo _capability_ — `query tools --capability <name> --status present` —
    KHÔNG tham chiếu tên tool. Áp **Degrade Ladder** (xem
    `docs/TOOL_REGISTRY.md`): không có provider nào đăng ký ⇒ capability
    _inactive_ → skip sạch (KHÔNG phải drift); đăng ký nhưng `missing`/thiếu một
    phần ⇒ _degraded_ → chạy với phần resolve được + bật cờ `Weak proof` + ghi
    gap; tất cả `present` ⇒ Full. Chạy `tool check` đầu intake để `status` phản
    ánh thực tế. Công cụ dự án chưa đăng ký thì
    `tool register --kind <k> --capability <cap> [--scan <path|url>]`.
- **Xử lý theo Input Type (DOCS FIRST):**
  - `IF [Type == New spec]`: Coi spec là _input material_, KHÔNG giữ làm spec
    sống. Xé nhỏ vào `docs/product/*` và tạo candidate epics/stories +
    decisions. (Vẫn áp dụng [Quy tắc cấm] ở cuối GĐ này: không mở rộng spec
    nguyên khối.)
  - `IF [Type == New initiative]` HOẶC product area lớn: Tạo 1 _initiative note_
    gồm: goal, docs ảnh hưởng, candidate stories, validation shape, open
    decisions, exit criteria (thay vì tạo spec nguyên khối thứ hai).
- **Cập nhật Product & Tạo Story:**
  - `IF [Lane == tiny]`: Bỏ qua Story.
  - `IF [Lane == normal]`: Cập nhật `docs/product/*`. Tạo 1 file sao chép từ
    `docs/templates/story.md` VÀ lưu theo chuẩn
    `docs/stories/epics/EXX-<domain>/US-YYY-<title>.md`.
  - `IF [Lane == high-risk]`: Cập nhật `docs/product/*`. Tạo folder mới theo
    chuẩn `docs/stories/epics/EXX-<domain>/US-YYY-<title>/`. BẮT BUỘC điền đủ 4
    neo nội dung:
    - `overview.md`: (Phải có Current/Target Behavior, Affected Users,
      Non-Goals).
    - `execplan.md`: (Phải có Scope, Work Phases, Stop Conditions).
    - `design.md`: (Phải có Domain Model, Interface Contract, Data Model).
    - `validation.md`: (Phải có Test Plan, Fixtures).
- **Decisions:** Nếu đổi Auth, API shape, Security, Data ownership -> BẮT BUỘC
  tạo file `docs/decisions/NNNN-*.md` VÀ chạy
  `harness-cli decision add --id <NNNN-id> --title "<Tên>" --doc docs/decisions/<file>.md`.
- **[STOP] Hard Gate:** KHÔNG ĐƯỢC phép viết hoặc sửa mã nguồn ứng dụng nếu
  Story Packet chưa được viết xong. (NGOẠI LỆ: Lane tiny bỏ qua Story — được sửa
  trực tiếp, nhưng CHỈ trong phạm vi docs/copy/setup cơ bản đã phân loại ở GĐ1;
  nếu trong lúc làm phát hiện phải đổi logic code, DỪNG LẠI và leo thang lane về
  normal.) Nếu hướng đi mông lung, DỪNG LẠI hỏi ý kiến con người.

---

## GIAI ĐOẠN 3: IMPLEMENTATION (Triển khai - CODE LATER)

- **Quy tắc cứng:** Chỉ bắt đầu viết code khi Giai đoạn 2 đã hoàn tất. Tuân thủ
  tuyệt đối "Dependency Rule" và "Parse-First Boundary" (Tra cứu tại
  `02-STANDARDS.md`). Bám sát chính xác những gì đã thiết kế trong `execplan.md`
  hoặc `design.md`.
- **Vừa code vừa giữ chuẩn (shift-left):** code theo ba ràng buộc — _Quality_
  (Dependency Rule, Parse-First, đúng `design.md`), _Security_ (validate input
  biên, KHÔNG lộ secret/credential, để ý Hard Gate), _Maintainability_
  (naming/coupling gọn, test theo Test Matrix). Kiểm chứng độc lập để cho Cửa ải
  Review.
- **TDD (on-demand):** `IF [task khóa-behavior]`: nạp `skills/tdd-red-green.md`
  (RED → GREEN → REFACTOR) TRƯỚC khi viết code logic. Danh sách nhóm task +
  ngoại lệ: xem Trigger của skill / registry `_harness/04-SKILLS.md`.
- **[STOP] Cửa ải Review (GĐ3→GĐ4):** Trước khi sang Giai đoạn 4, Agent BẮT BUỘC
  nạp và chạy skill `skills/quality-gate-review.md` — một vòng review độc lập 3
  lens (Quality&Architecture / Security&Risk / Maintainability&Proof). KHÔNG
  sang GĐ4 sign-off (đánh proof `1`) khi còn finding `blocking` chưa xử lý: hoặc
  sửa code rồi `story verify` lại pass, hoặc ghi backlog (GĐ6). Xem hợp đồng +
  cách nạp skill ở `_harness/04-SKILLS.md`.

---

## GIAI ĐOẠN 4: VALIDATION (Xác thực)

- **Validation Ladder:** `validate:quick`, `test:integration`, `test:e2e`,
  `test:platform`, `test:release`. KHÔNG báo cáo PASS nếu lệnh chưa tồn tại.
- **Batch verify trước mốc lớn:** Trước khi merge, claim maturity (H4+), hoặc
  chạy benchmark, BẮT BUỘC chạy `harness-cli story verify-all` để verify hàng
  loạt mọi story có `verify_command` (thoát `1` nếu có story fail).
- **Story Status:** `planned`, `in_progress`, `implemented` (đã code VÀ có
  proof), `changed`, `retired`.
- **Hành động CLI:**
  1. Gắn verify command:
     `harness-cli story update --id <ID> --verify "<command>"`.
  2. Chạy xác thực: `harness-cli story verify <ID>`. _(Lệnh thoát mã 0=pass,
     1=fail. Nếu fail, Agent VẪN ĐƯỢC sang Giai đoạn 5 để ghi nhận tác vụ dở
     dang)._
  3. Cập nhật matrix: `harness-cli story update --id <ID> --unit 1 ...` (Dùng
     1/0).
- **[STOP] Cửa ải Bằng chứng:** BẮT BUỘC phải đọc log output (stdout/stderr) của
  lệnh `verify` trước khi đánh dấu `1` (pass) vào matrix. Cấm tự suy diễn kết
  quả. Nếu `quality-gate-review` (cổng GĐ3→4) vừa chạy `story verify` và code
  KHÔNG đổi từ đó → tái dùng log, KHÔNG chạy lại. (Nếu verify fail, vẫn được
  sang Giai đoạn 5 để ghi Trace partial/failed).

---

## GIAI ĐOẠN 5: TRACE & INTERVENTION (Ghi dấu vết)

- **Kiểm tra File:** BẮT BUỘC chạy lệnh `git status --short` để lấy chính xác
  danh sách file trước khi ghi nhận.
- **Outcome:** Chọn một trong: `completed`, `blocked`, `partial`, hoặc `failed`.
- **Tier Rules & Cú pháp CLI:** (CHÚ Ý: Lệnh CLI nhận danh sách ngăn cách bằng
  DẤU PHẨY, KHÔNG truyền ngoặc vuông JSON array).
  - `Minimal` (Tiny): Cần `task_summary` (>10 ký tự), `outcome`.
  - `Standard` (Normal): Minimal + `intake_id`, `story_id`, `agent`,
    `actions_taken` (dấu phẩy), `files_read` (dấu phẩy), `files_changed` (dấu
    phẩy), `errors` hoặc `friction`.
  - `Detailed` (High-Risk): Standard + `decisions_made` (dấu phẩy), `errors`
    (ghi 'none' nếu không có), `duration_seconds`, `token_estimate`.
- **Friction & Failure Attribution:** Friction phải NÊU ĐÍCH DANH VẤN ĐỀ (ghi
  'none' nếu đã kiểm tra và không có vấn đề).
  `IF [Outcome == failed OR partial]`, BẮT BUỘC quy gán lỗi vào 1 trong 11
  Responsibilities (VD: _Task specification_, _Data model_...).
- **Khi nào BẮT BUỘC ghi Friction:** (1) phải suy đoán một luật/nguồn-sự-thật
  còn thiếu; (2) validation không rõ, không chạy được, hoặc quá tốn kém; (3)
  doc/record/story cũ hoặc mâu thuẫn; (4) lộ ra bước thủ công lặp lại nên thành
  template/lệnh/checklist; (5) thay đổi out-of-scope nhưng quan trọng về sau.
- **[Lưu ý] Decisions ≠ Decision record:** Trường `decisions` trong trace chỉ là
  bằng chứng, KHÔNG thay thế decision record bền vững ở
  `docs/decisions/NNNN-*.md` (xem GĐ2).
- **Intervention (tách khỏi trace):** Khi human / reviewer / CI / agent khác
  **sửa, ghi đè, leo thang, hoặc duyệt** công việc, ghi bằng
  `harness-cli intervention add --trace <id> --type <type> --description "<text>" --source <human|reviewer|ci|agent>`.
  Intervention lưu RIÊNG trace và là đầu vào cho `propose` (GĐ6).
- **Context score (advisory):** Có thể chạy
  `harness-cli score-context <trace-id>` để đối chiếu `files_read` với context
  rules; chỉ để tự kiểm, KHÔNG đổi trace.

---

## GIAI ĐOẠN 6: GROWTH (Tiến hóa)

- `IF` [Có Friction hoặc thiếu capability]: Thêm vào Backlog qua CLI.
- **Backlog Protocol:** BẮT BUỘC dùng `--predicted "<kết quả dự đoán>"`. Khi
  đóng ticket dùng `--outcome "<thực tế>"`. (Risk chỉ được chọn `tiny`,
  `normal`, `high-risk`).
- **Vòng tự cải tiến:**
  `friction + interventions + audit -> propose -> backlog`.
  - Chạy `harness-cli audit` để lấy nhóm drift + điểm entropy (thấp là tốt;
    trọng số ở `docs/HARNESS_AUDIT.md`).
  - Chạy `harness-cli propose` để sinh đề xuất tất định từ
    friction/intervention/audit; `propose --commit` CHỈ tạo backlog item
    `proposed`, KHÔNG tự sửa policy hay tự duyệt.
  - Con người duyệt proposal (`query backlog --open`). Đề xuất đổi source
    hierarchy / kiến trúc / validation / risk policy PHẢI tạo decision record
    trước khi áp dụng (xem `docs/IMPROVEMENT_PROTOCOL.md`).

---

## GIAI ĐOẠN 7: DONE (Hoàn thành)

Một tác vụ chỉ được coi là xong khi: Đổi code xong (hoặc block đã log),
Docs/Matrix cập nhật, Validation đã chạy, Trace đã lưu.

- **Cửa ải Quản trị (BẮT BUỘC xin phép người trước khi):** đổi hướng kiến trúc;
  gỡ hoặc làm yếu yêu cầu validation; đổi source-of-truth hierarchy; đổi luật
  phân loại rủi ro (lane/hard gate); thay thế chính workflow này.
- **Rào cản Maturity (Anti-Hallucination):** (tra `docs/HARNESS_MATURITY.md`;
  phân biệt rõ claim "partial" với "full").
  - KHÔNG claim H3 nếu chưa có đối chiếu benchmark và quy gán lỗi theo
    Component.
  - H4 = batch verification: KHÔNG claim H4 nếu `story verify-all` chưa chạy
    được.
  - H5 = tự cải tiến: chỉ claim H5 _partial_ khi `audit` + `propose` +
    `intervention` đã có và chạy được; KHÔNG claim H5 _full_ cho tới khi
    benchmark/trace chứng minh vòng propose tạo delta dương (hoặc bị revert).
- **Hành động:** Trả lời User, tóm tắt rõ ID, thay đổi, và những gì không được
  thử.
