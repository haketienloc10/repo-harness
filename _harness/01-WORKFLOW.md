# Trục Xương Sống: Quy trình 7 Giai đoạn (Harness Workflow)

## ĐỊNH MỨC TOKEN (Context Budget)

- **Phạm vi định mức (đọc trước khi áp số):** Con số mỗi lane bên dưới là TỔNG
  Harness context theo read-shape của lane (không tách "tầng nền" riêng) — khớp
  định nghĩa ở `_harness/docs/CONTEXT_RULES.md` (nguồn chuẩn). Nguyên tắc vận hành: ưu
  tiên `rg` có mục tiêu thay vì đọc hàng loạt; đọc section nhỏ nhất trả lời câu
  hỏi của giai đoạn hiện tại; escalate context khi một retrieval trigger fire;
  ngừng đọc history không liên quan khi đã rõ lane + file ảnh hưởng + validation
  path. Mô hình phase × lane (Must/Should/Skip) chi tiết:
  `_harness/docs/CONTEXT_RULES.md` (on-demand).
- **Tài liệu dùng chung (Luôn có thể truy xuất):** Bất cứ khi nào cần tương tác
  với `harness.db`, Agent luôn được phép đọc `_harness/03-CLI_REFERENCE.md`
  (cheatsheet gọn) để lấy cú pháp; chi tiết sâu hơn nằm ở
  `_harness/docs/CLI_REFERENCE.md` và `harness-cli <cmd> --help` — chỉ tra on-demand,
  KHÔNG preload. Để biết lệnh/công cụ nào đang có, dùng
  `harness-cli query tools --summary` (tool registry, xem
  `_harness/docs/TOOL_REGISTRY.md`) thay vì đoán.
- **Skill (nạp on-demand):** KHÔNG preload `skills/*`. Tới giai đoạn có trigger
  khớp trong registry `_harness/04-SKILLS.md`, mới đọc ĐÚNG file skill đó.
- **Tầng nền (MỌI lane, đọc ĐẦU TIÊN):** `docs/KNOWLEDGE_INDEX.md` — bản đồ
  onboarding cô đọng (ROUTER, không phải nguồn sự thật; xem `00-AGENTS.md` §1).
  Rẻ hơn crawl `docs/`; dùng nó để chọn đúng file cần đọc tiếp cho lane hiện
  tại. Giới hạn freshness của `knowledge check`: xem nguồn duy nhất ở
  `00-AGENTS.md` §1.
- **Tiny Lane:** ~2,000 tokens Harness context. Đọc: `00-AGENTS.md`, intake
  docs, matrix query, và đúng file cần sửa.
- **Normal Lane:** ~5,000 tokens Harness context. Đọc thêm product/story docs
  liên quan, architecture (nếu cần đổi cấu trúc), validation expectations, và
  trace spec ở cuối.
- **High-Risk Lane:** ~10,000 tokens Harness context. Đọc toàn bộ intake,
  architecture, quyết định liên quan, templates rủi ro cao, product/validation
  docs, trace spec, và component/maturity docs.

---

## GIAI ĐOẠN 1: INTAKE (Phân loại)

- **0. Orient (đọc TRƯỚC khi phân loại):** Đọc `docs/KNOWLEDGE_INDEX.md` để nắm
  Purpose + Top-Level Structure của repo trước khi chọn Type và đếm Risk Flags
  (hiểu repo giúp phân loại đúng). Giới hạn freshness của `knowledge check`
  (đỏ/xanh nghĩa là gì): xem nguồn duy nhất ở `00-AGENTS.md` §1.
- **0b. State digest (CÓ ĐIỀU KIỆN):** Nếu lượt này CHẠY workflow 7-GĐ (sẽ ghi
  durable state) HOẶC câu hỏi là về _trạng thái dự án_ ("đang/đã/cần làm gì",
  "story X tới đâu") ⇒ chạy `harness-cli query status` để định hướng (đang làm /
  cần proof / resume / backlog / intervention / hoạt động gần). Khi tiếp tục một
  story dài, thêm `harness-cli query recap --story <id>` để tiêu hóa lịch sử
  trace. Nếu là hỏi-đáp về _nội dung_ code/doc một-bước (không chạy workflow) ⇒
  BỎ QUA (chung predicate với Execution Tracker — xem `00-AGENTS.md` §3). Mơ hồ
  nhưng đụng workflow ⇒ chạy (bất đối xứng chi phí: chạy thừa = vài trăm token;
  quên = chọn nhầm story / nhân bản backlog).
- **1. Chọn Type** (dùng khi → đích đến artifact):
  - `New spec` — biến spec người dùng thành docs harness-ready →
    `docs/product/*` kèm candidate epics và decisions.
  - `Spec slice` — triển khai một hành vi đã chọn từ spec đã chấp nhận → 1 story
    packet.
  - `Change request` — đổi/sửa/tinh chỉnh hành vi đã chấp nhận → story packet
    hoặc patch trực tiếp.
  - `New initiative` — thêm mảng sản phẩm lớn cần nhiều story → initiative
    note + nhiều story packet.
  - `Maintenance request` — đổi hành vi kỹ thuật/vận hành/dependency → story /
    validation report / decision.
  - `Harness improvement` — cải thiện cách người + agent cộng tác → cập nhật
    docs hoặc `harness-cli backlog add`.
- **2. Đếm Rủi ro (Risk Flags) — đánh 1 cờ cho mỗi mục công việc CHẠM tới:**
  - (1) **Auth** — login, logout, session, JWT, password, refresh token.
  - (2) **Authorization** — role, permission, tenant hoặc company scope.
  - (3) **Data model** — schema, migration, uniqueness, deletion, retention.
  - (4) **Audit/security** — audit log, privacy, dữ liệu nhạy cảm, access log.
  - (5) **External systems** — email, payment, cloud service, provider SDK,
    queue, webhook.
  - (6) **Public contracts** — API shape, response envelope, hành vi client thấy
    được.
  - (7) **Cross-platform** — phân tách desktop/mobile/browser, native shell,
    deep link.
  - (8) **Existing behavior** — đổi hành vi đã triển khai hoặc đã có test phủ.
  - (9) **Weak proof** — test quanh vùng ảnh hưởng còn mơ hồ hoặc thiếu.
  - (10) **Multi-domain** — hơn một product domain đổi cùng lúc.
- **3. Hard Gates (Rào cản cứng):** Auth, Authorization, Data loss/migration,
  Audit/security, External provider, Làm yếu validation.
- **4. Thuật toán Lane:**
  - `IF` [>= 4 Flags]: **Lane = high-risk** (KHÔNG có ngoại lệ hạ lane cho nhánh
    này).
  - `IF` [Dính >= 1 Hard Gate]: **Lane = high-risk** (NGOẠI LỆ DUY NHẤT: nếu con
    người chủ động thu hẹp phạm vi rõ ràng, được phép hạ lane — ngoại lệ này CHỈ
    áp cho hard gate, KHÔNG áp cho nhánh `>= 4 Flags`).
  - `IF` [2-3 Flags]: **Lane = normal** (validation mạnh hơn).
  - `IF` [0-1 Flags] VÀ [Sửa docs/copy/setup cơ bản]: **Lane = tiny**.
  - `IF` [0-1 Flags] VÀ [Đổi logic code]: **Lane = normal**.
  - **[Lưu ý setup/health — ranh giới Tiny]:** Setup ban đầu CHỈ thuộc tiny khi
    giới hạn ở: cài dependency đã khai báo, wiring server entrypoint, thêm
    health/smoke endpoint, hoặc mở kết nối DB dev cục bộ — và KHÔNG tạo domain
    schema, CRUD, auth, authorization, provider integration, hay data migration.
    Health/smoke endpoint là _smoke proof_, KHÔNG tự tính là cờ "Public
    contracts" → đừng leo thang lane chỉ vì health endpoint. Nhưng nếu setup
    chạm bất kỳ thứ nào trong danh sách loại trừ trên (schema, auth,
    migration…), nó KHÔNG còn là tiny → đếm flag/hard gate bình thường.
- **5. Hành động:** Chạy
  `harness-cli intake --type "<loại>" --summary "<text>" --lane <lane> --flags "<csv các flag đã đếm ở §2>"`.
  Truyền `--flags` để durable layer giữ bằng chứng đằng sau lane (cần cho ngoại
  lệ hạ lane ở §4 + `audit`/`propose` về sau). LƯU lại `intake_id` lệnh in ra —
  dùng cho `--intake` ở GĐ4 và `trace --intake` ở GĐ5 (tránh orphan, drift ×10).
- **[Quy tắc cấm]:** KHÔNG ĐƯỢC tạo hoặc mở rộng một file `SPEC.md` nguyên khối.
  Mọi thay đổi phải được xé nhỏ vào `docs/product/` và `docs/stories/`.

---

## GIAI ĐOẠN 2: PLANNING (Lập kế hoạch - DOCS FIRST)

- **Retrieval Triggers (Kích hoạt lấy Context):**
  - `IF` chạm database schema, durable records, migrations: Đọc
    `_harness/schema/`.
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
    `_harness/docs/TOOL_REGISTRY.md`): không có provider nào đăng ký ⇒ capability
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
    `_harness/docs/templates/story.md` VÀ lưu theo chuẩn
    `docs/stories/epics/EXX-<domain>/US-YYY-<title>.md`.
  - `IF [Lane == high-risk]`: Cập nhật `docs/product/*`. Tạo folder mới theo
    chuẩn `docs/stories/epics/EXX-<domain>/US-YYY-<title>/`. BẮT BUỘC điền đủ 4
    neo nội dung:
    - `overview.md`: (Phải có Current/Target Behavior, Affected Users,
      Non-Goals).
    - `execplan.md`: (Phải có Scope, Work Phases, Stop Conditions).
    - `design.md`: (Phải có Domain Model, Interface Contract, Data Model).
    - `validation.md`: (Phải có Test Plan, Fixtures).
- **[BẮT BUỘC normal/high-risk] Ghi story vào durable layer — NGAY sau khi tạo
  packet, TRƯỚC khi sang GĐ3:** chạy `harness-cli story add` (cú pháp + flag đầy
  đủ: `03-CLI_REFERENCE.md` §GĐ2). Bỏ bước này thì `story update`/`story verify`
  ở GĐ4 sẽ lỗi `story '<ID>' not found` (CLI không tự tạo row từ file packet).
  Lane tiny KHÔNG có story → bỏ qua.
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

- **Validation Ladder (nội dung từng nấc):**
  - `validate:quick` — format, lint, typecheck, unit test, architecture check.
  - `test:integration` — backend, database, provider, hoặc service check tùy
    stack.
  - `test:e2e` — luồng end-to-end người dùng thấy được.
  - `test:platform` — shell, mobile, desktop, hoặc deployment smoke check tùy
    stack.
  - `test:release` — full suite, log check, và performance smoke.
  - KHÔNG báo cáo PASS cho một nấc nếu lệnh chưa tồn tại VÀ chưa thực sự chạy.
- **Batch verify trước mốc lớn:** Trước khi merge, claim maturity (H4+), hoặc
  chạy benchmark, BẮT BUỘC chạy `harness-cli story verify-all` để verify hàng
  loạt mọi story có `verify_command` (thoát `1` nếu có story fail).
- **Story Status:** `planned`, `in_progress`, `implemented` (đã code VÀ có
  proof), `changed`, `retired`.
- **[Lane tiny — không có story]:** BỎ mọi action `story ...` dưới đây. Chạy
  `validate:quick` thủ công, ĐỌC log (stdout/stderr), rồi sang GĐ5 ghi `trace`
  tier Minimal. Cửa ải Bằng chứng vẫn áp dụng cho log đã đọc.
- **Hành động CLI (normal/high-risk)** _(cú pháp + flag đầy đủ:
  `03-CLI_REFERENCE.md` §GĐ4)_:
  1. Gắn verify command: `story update --id <ID> --verify "<command>"`.
  2. Chạy xác thực: `story verify <ID>` _(thoát 0=pass, 1=fail; nếu fail Agent
     VẪN ĐƯỢC sang GĐ5 để ghi tác vụ dở dang)_.
  3. Cập nhật matrix: `story update --id <ID> --unit 1 ...` (dùng 1/0).
- **[STOP] Cửa ải Bằng chứng:** BẮT BUỘC phải đọc log output (stdout/stderr) của
  lệnh `verify` trước khi đánh dấu `1` (pass) vào matrix. Cấm tự suy diễn kết
  quả. Nếu `quality-gate-review` (cổng GĐ3→4) vừa chạy `story verify` và code
  KHÔNG đổi từ đó → tái dùng log, KHÔNG chạy lại. (Nếu verify fail, vẫn được
  sang Giai đoạn 5 để ghi Trace partial/failed).
- **Bằng chứng bền vững (auto-capture, default-on):** `story verify <id>` TỰ ghi
  stdout+stderr vào evidence store (`kind='log'`, kèm `command` + `result`),
  dedup keep-last theo `(story, kind, result)` + sha256; in evidence id ở output.
  Proof boolean `1` LUÔN có log tươi đỡ lưng — đây là điều `done-check` (GĐ7)
  enforce. Cờ `--no-capture` chỉ cho lần chạy nháp. Artifact phi-verify
  (screenshot E2E, report) ghi qua `harness-cli evidence add` và nêu evidence id
  ở `--notes` của trace (GĐ5).

---

## GIAI ĐOẠN 5: TRACE & INTERVENTION (Ghi dấu vết)

- **Kiểm tra File:** BẮT BUỘC chạy lệnh `git status --short` để lấy chính xác
  danh sách file trước khi ghi nhận.
- **Outcome:** Chọn một trong: `completed`, `blocked`, `partial`, hoặc `failed`.
- **Tier Rules & Cú pháp CLI:** (CHÚ Ý: Lệnh CLI nhận danh sách ngăn cách bằng
  DẤU PHẨY, KHÔNG truyền ngoặc vuông JSON array).
  - `Minimal` (Tiny): Cần `task_summary` (>10 ký tự), `outcome`. **KHÔNG hợp lệ
    cho:** việc normal/high-risk; HOẶC bất kỳ việc nào phát hiện friction,
    errors, hay thiếu validation path → khi đó BẮT BUỘC nâng lên `Standard`.
  - `Standard` (Normal): Minimal + `agent` + `actions_taken` (dấu phẩy) +
    `files_read` (dấu phẩy) + `files_changed` (dấu phẩy) + ít nhất một trong
    `errors`/`friction`; thêm `intake_id` **khi đã ghi intake**, `story_id`
    **khi việc map gọn vào một story** (nếu một trace phủ nhiều story → dùng
    story chính, liệt kê phần còn lại ở `notes`).
  - `Detailed` (High-Risk): Standard + `decisions_made` (dấu phẩy) + `errors`
    (ghi 'none' nếu không có) + `friction` (Detailed LUÔN ghi; 'none' chỉ sau
    khi đã kiểm tra) + `duration_seconds` **hoặc** note giải thích vì sao không
    có + `token_estimate` **hoặc** note giải thích vì sao không có + `notes`
    **khi** một trace phủ nhiều story / nhiều risk flag / có bỏ qua validation.
  - **Nâng tier theo lane:** Tiny→Minimal, Normal→Standard, High-Risk→Detailed.
    NGOẠI LỆ bắt buộc: việc Tiny mà đổi Harness instruction/validation
    expectation/durable record, HOẶC phát hiện friction → nâng lên `Standard`.
- **Friction & Failure Attribution:** Friction phải NÊU ĐÍCH DANH VẤN ĐỀ (ghi
  'none' nếu đã kiểm tra và không có vấn đề).
  `IF [Outcome == failed OR partial]`, BẮT BUỘC quy gán lỗi vào **1 trong 11
  Responsibilities** (Runtime Substrate) dưới đây (VD: _Task specification_,
  _Verification_...). LƯU Ý: "Data model" KHÔNG phải Responsibility — nó là Risk
  Flag (GĐ1).
  - **11 Responsibilities (nguồn quy gán cho mọi GĐ — GĐ5, cổng review, H3):**
    (1) _Task specification_, (2) _Context selection_, (3) _Tool access_, (4)
    _Project memory_, (5) _Task state_, (6) _Observability_, (7) _Failure
    attribution_, (8) _Verification_, (9) _Permissions_, (10) _Entropy
    auditing_, (11) _Intervention recording_. (Mô tả/trạng thái sâu:
    `_harness/docs/HARNESS_COMPONENTS.md`.) `query recap` gom Friction theo đúng 11
    Responsibilities này.
- **Next-action / Resume (BẮT BUỘC khi việc còn dở):** `IF [Outcome ∈
  {partial, blocked, failed}]` ⇒ trace BẮT BUỘC có `--next-action "<việc kế
  tiếp>"` (CLI từ chối rỗng, exit != 0). Nếu trace có `--story`, hint ghi luôn
  vào `story.next_action` (con trỏ WIP sống) và nổi lên ở RESUME của `query
  status`. `IF [Outcome == completed]` và có `--story` ⇒ `story.next_action` tự
  được clear (resume hint không sống quá việc đã xong).
- **Khi nào BẮT BUỘC ghi Friction:** (1) phải suy đoán một luật/nguồn-sự-thật
  còn thiếu; (2) validation không rõ, không chạy được, hoặc quá tốn kém; (3)
  doc/record/story cũ hoặc mâu thuẫn; (4) lộ ra bước thủ công lặp lại nên thành
  template/lệnh/checklist; (5) thay đổi out-of-scope nhưng quan trọng về sau;
  (6) một benchmark/review fail mà KHÔNG quy được về một Component. Nếu friction
  nên trở thành công việc → thêm/cập nhật backlog item (GĐ6).
- **[Lưu ý] Decisions ≠ Decision record:** Trường `decisions` trong trace chỉ là
  bằng chứng, KHÔNG thay thế decision record bền vững ở
  `docs/decisions/NNNN-*.md` (xem GĐ2).
- **Intervention (tách khỏi trace):** Khi human / reviewer / CI / agent khác
  **sửa, ghi đè, leo thang, hoặc duyệt** công việc, ghi bằng
  `harness-cli intervention add --type <correction|override|escalation|approval> --description "<text>" --source <human|reviewer|ci|agent> [--trace <id>] [--story <id>]`.
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
  - Chạy `harness-cli audit` để lấy nhóm drift + điểm entropy (THẤP là tốt).
    - **6 nhóm drift (trọng số):** Orphaned stories — story planned/in_progress
      không có trace liên kết (**10**); Broken tools — tool đã đăng ký nhưng
      command không tìm thấy trên disk/PATH (**8**); Unverified stories — story
      có `verify_command` nhưng chưa có kết quả verify (**5**); Unverified
      decisions — decision có `verify_command` nhưng chưa verify (**5**); Stale
      stories — story chưa implement mà trace mới nhất > 30 ngày (**3**); Open
      backlog without outcomes — backlog `implemented` có `--predicted` nhưng
      thiếu `--outcome` (**2**).
    - **Điểm:** `score = Σ (số lượng × trọng số)`, **cap 100**. Diễn giải: `0`
      hoàn hảo; `1-25` lành mạnh (housekeeping nhỏ); `26-50` cần chú ý (drift
      tích tụ); `51-100` phải hành động (state cũ làm xói mòn giá trị Harness).
  - Chạy `harness-cli propose` để sinh đề xuất tất định (rule-based). Nó tìm:
    **trace friction lặp lại**, **pattern intervention lặp lại**, **audit
    category khác 0**. Mỗi proposal gồm 8 trường: title, component, evidence,
    predicted impact, risk, suggested action, validation plan, confidence.
    `propose --commit` CHỈ tạo backlog item `proposed`, KHÔNG tự sửa policy hay
    tự duyệt.
  - **Review rules theo risk của proposal** (con người duyệt qua
    `query backlog --open`):
    - `tiny` — được implement trực tiếp KHI chỉ làm rõ docs.
    - `normal` — cần một story packet hoặc backlog acceptance rõ ràng.
    - `high-risk` — PHẢI tạo decision record bền vững TRƯỚC khi đổi source
      hierarchy / kiến trúc / validation requirement / risk policy.
    - Hoàn tất một proposal = đóng backlog item kèm `--outcome` (bằng chứng thực
      tế).
  - **Validation sau implementation:** đối chiếu predicted vs actual bằng
    `harness-cli audit`, `query friction`, `query interventions` (và benchmark
    trace quality nếu áp dụng). Chi tiết quy trình:
    `_harness/docs/IMPROVEMENT_PROTOCOL.md`.

---

## GIAI ĐOẠN 7: DONE (Hoàn thành)

Một tác vụ chỉ được coi là xong khi: Đổi code xong (hoặc block đã log),
Docs/Matrix cập nhật, Validation đã chạy, Trace đã lưu.

- **[STOP] Cửa ải Done-check (cơ học, lane-aware):** BẮT BUỘC chạy
  `harness-cli done-check --story <id>` (hoặc `--intake <id>`). Exit `!= 0` ⇒
  CHƯA done: đọc checklist `✘`, sửa cho xanh hoặc ghi backlog (GĐ6) rồi mới
  tuyên bố xong. Gate kiểm theo lane: tiny chỉ cần ≥1 trace liên kết;
  normal/high-risk thêm `status=implemented`, `verify pass`, có evidence `log`
  pass, ≥1 proof flag, `next_action` đã clear; high-risk thêm 4 neo packet. KHÔNG
  tự khẳng định "done" bằng văn xuôi khi `done-check` chưa exit 0.
- **Smoke audit (BẮT BUỘC trước khi đóng tác vụ):** chạy `harness-cli audit`.
  Nếu xuất hiện **Orphaned stories** (×10) hoặc **Unverified stories** (×5) do
  chính tác vụ này tạo ra → xử lý NGAY (link trace / verify / ghi backlog GĐ6)
  trước khi báo done. Rẻ, chặn đúng 2 nhóm drift nặng nhất. Entropy còn lại
  thuộc housekeeping → để vòng GĐ6.
- **Cửa ải Quản trị (BẮT BUỘC xin phép người trước khi):** đổi hướng kiến trúc;
  gỡ hoặc làm yếu yêu cầu validation; đổi source-of-truth hierarchy; đổi luật
  phân loại rủi ro (lane/hard gate); thay thế chính workflow này.
- **Thang Maturity H0–H5 (một mức chỉ "achieved" khi tiêu chí kiểm chứng được
  trong file/durable record/benchmark):**
  - **H0 Bare Environment** — không có harness; chỉ nhận prompt và sinh patch.
  - **H1 Scaffolding & Policy** — có instruction tĩnh, template, risk lane,
    source-of-truth rule; durable state có thể còn thủ công.
  - **H2 Durable State & Observability** — `harness-cli` ghi được
    intake/story/decision/backlog/trace vào `harness.db`; có TRACE_SPEC,
    CONTEXT_RULES, COMPONENTS, MATURITY.
  - **H3 Active Observability & Evolution** — chấm điểm trace lặp lại được;
    friction nhóm theo Component; backlog có predicted+outcome; benchmark quy
    được responsibility nào dịch chuyển/regress.
  - **H4 Automated Verification** — chạy/điều phối proof nhất quán; story có
    `verify_command`; cảnh báo khi story liên kết chưa verify pass; lộ thiếu
    proof trước final response.
  - **H5 Self-Improving Harness** — dùng trace/benchmark/backlog outcome để đề
    xuất/áp cải tiến an toàn; thay đổi high-risk dừng xin phép người.
- **Rào cản Maturity (Anti-Hallucination)** — phân biệt rõ claim "partial" với
  "full":
  - KHÔNG claim H3 _full_ nếu chưa có đối chiếu benchmark VÀ quy gán lỗi theo
    Component (11 Responsibilities, xem GĐ5).
  - H4 = batch verification: KHÔNG claim H4 nếu `story verify-all` chưa chạy
    được.
  - H5 = tự cải tiến: chỉ claim H5 _partial_ khi `audit` + `propose` +
    `intervention` đã có và chạy được; KHÔNG claim H5 _full_ cho tới khi
    benchmark/trace chứng minh vòng propose tạo delta dương (hoặc bị revert).
  - (Tiêu chí/required files/benchmark indicators đầy đủ:
    `_harness/docs/HARNESS_MATURITY.md`.)
- **Hành động:** Trả lời User, tóm tắt rõ ID, thay đổi, và những gì không được
  thử.
