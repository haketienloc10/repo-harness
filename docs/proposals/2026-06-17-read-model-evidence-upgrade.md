# Spec: Tầng Read-Model + Evidence — Nâng cấp Workflow Harness

- **Trạng thái:** ĐÃ XÉ & TRIỂN KHAI — đóng băng làm bản ghi thiết kế. Đã xé thành
  epic `docs/stories/epics/E02-read-model/` (US-002..US-008) + 2 decision records
  (`docs/decisions/0001-*`, `0002-*`) + product doc `docs/product/read-model.md`.
  Intake durable `#3`. Code ở `crates/harness-cli/*`, migrations `006`/`007`.
- **Đính chính nhỏ (§6.2):** ví dụ Friction nhóm theo "Data model" là không chính
  xác — "Data model" là một Risk Flag (GĐ1), KHÔNG nằm trong 11 Responsibilities.
  `query recap` gom Friction theo đúng 11 Responsibilities (`Task specification`,
  `Verification`, ...), bucket còn lại là `Unattributed`.
- **Ngày:** 2026-06-17
- **Loại intake dự kiến:** `harness improvement` (xé thành nhiều story, xem §8).
- **Phạm vi:** 5 đề xuất nâng cấp workflow theo hướng _agent-first_, mục tiêu để
  agent hiểu rõ project: **đã làm gì / đang làm gì / cần làm gì**, có **bằng
  chứng + artifact**.

> **Lưu ý quy ước (Quy tắc cấm — `01-WORKFLOW.md` GĐ1/GĐ2):** File này là _tài
> liệu đề xuất_, KHÔNG phải `SPEC.md` nguyên khối sống. Nội dung phải được xé nhỏ
> thành `docs/product/*` + story packets + decisions trước khi triển khai (kế
> hoạch xé ở §8). Sau khi xé xong, file này đóng băng làm bản ghi thiết kế.

---

## 0. Bối cảnh & Insight gốc

Harness hiện tối ưu cho **write-path của một-task** (ghi intake → story → trace →
proof rất kỹ). Mục tiêu "agent hiểu project" lại là bài toán **read-path
xuyên-task**. Đây là hai trục khác nhau, và read-path đang thiếu một tầng.

Hiện có đúng **3 mặt phẳng đọc**, đang lẫn vai:

| Mặt phẳng           | Trả lời                              | Tính chất         | Hiện trạng |
| ------------------- | ----------------------------------- | ----------------- | ---------- |
| `KNOWLEDGE_INDEX.md`| Repo _là cái gì_ (cấu trúc, tech)   | Tĩnh, router      | ✅ có      |
| `query stats`       | _Bao nhiêu_ (đếm tổng)              | Động, số liệu thô | ✅ có      |
| **Read-Model**      | _Đang/đã/cần làm gì_ (hành động)    | Động, xếp hạng    | ❌ trống   |

5 đề xuất dưới đây lấp mặt phẳng thứ ba và đóng vòng bằng-chứng:

| ID  | Tên                          | Lệnh mới                    | Schema mới | GĐ cắm vào |
| --- | ---------------------------- | --------------------------- | ---------- | ---------- |
| P1  | Read-Model / Session Brief   | `query status`              | Không      | GĐ0        |
| P2  | Evidence / Artifact store    | `evidence add/list`         | 007        | GĐ4 + GĐ5  |
| P3  | Next-action / Resume         | (trường trace/story)        | 006        | GĐ5 + GĐ0  |
| P4  | Done-check gate              | `done-check`                | Không      | GĐ7        |
| P5  | Recap rollup                 | `query recap`               | Không      | GĐ0 + GĐ6  |

---

## 1. Nguyên tắc thiết kế chung (bất biến toàn spec)

1. **Read-Model là VIEW dẫn xuất, KHÔNG là store.** Mọi byte `status`/`recap`
   hiển thị phải truy về được `intake`/`story`/`trace`/`backlog`/`intervention`.
   Vi phạm = tạo nguồn-sự-thật thứ hai → cấm. P1 và P5 KHÔNG có schema.
2. **First-class-có-điều-kiện.** Lệnh đọc-định-hướng (P1) gắn cứng vào luật skip
   của `00-AGENTS.md §3`: lượt CÓ chạy workflow 7-GĐ ⇒ chạy; lượt hỏi-đáp
   một-bước (không chạy workflow) ⇒ skip. KHÔNG tạo trục phân loại mới.
3. **Determinism.** Mọi output (status, recap, done-check) phải tất định với cùng
   trạng thái db — không LLM, không random, không thời gian-phụ-thuộc ngoài
   `created_at` sẵn có. Đồng nhất với `audit`/`propose`.
4. **Token-aware.** `status` mục tiêu ≤ ~1k token: mỗi section có **trần dòng**
   (mặc định 5) và **báo số đã cắt** ("no silent caps"). Không bao giờ đổ raw.
5. **Additive migration.** Theo đúng `scripts/schema/`: `ALTER TABLE ... ADD
   COLUMN` cho thay đổi cộng thêm, `CREATE TABLE` cho bảng mới, kèm
   `INSERT INTO schema_version`. Không phá cột/row cũ.
6. **Repo sạch (chốt với người dùng).** Artifact nặng (P2) nằm local & gitignore;
   db chỉ giữ **con trỏ + hash + digest**. Reviewer thấy con trỏ, không thấy file
   thô. Đánh đổi đã chấp nhận: mất tái-dựng artifact qua máy khác/CI.

---

## 2. P1 — Read-Model / Session Brief (`harness-cli query status`)

### 2.1 Vấn đề

Khi agent (hoặc session mới) bắt đầu, nó phải tự ghép `query matrix` +
`query backlog --open` + `query traces` + lọc story `in_progress`. Không có "bức
tranh hiện trạng" duy nhất ⇒ agent chọn nhầm story, bỏ sót WIP, nhân bản backlog.

### 2.2 Thiết kế

**Schema:** KHÔNG. Pure read view (bất biến #1).

**CLI:**

```bash
harness-cli query status [--json] [--lane tiny|normal|high_risk] [--limit <n>] [--full]
```

- `--json`: output máy-đọc (cho host/agent parse).
- `--limit <n>`: trần dòng mỗi section (mặc định 5).
- `--full`: bỏ trần (in hết) — dùng khi cần audit thủ công.
- `--lane`: lọc theo lane.

**Output (text mode), theo thứ tự ưu tiên hành động:**

```text
HARNESS STATUS  (db: harness.db · drift: <entropy>/<n groups>)

▌ ĐANG LÀM (in_progress)            <m> story
  • US-031  Read-model status cmd   normal   2d   → next: wire query matrix
  • ...
  (+3 nữa — dùng --full)

▌ CẦN PROOF (implemented, chưa pass)  <m>
  • US-029  Evidence store          verify=fail (last 1h)  unit=1 integ=0

▌ RESUME (partial/blocked)          <m>
  • trace#142  blocked              → next: chờ quyết định gitignore evidence

▌ BACKLOG MỞ (high-risk trước)       <m>
  • #57 [high-risk] done-check gate  pred: giảm false-done

▌ INTERVENTION gần đây               <m>
  • #12 override (reviewer) trên trace#140

▌ HOẠT ĐỘNG GẦN NHẤT                 <m>
  • trace#142 partial  "wire evidence digest"   (score 2/3)
```

**Nguồn từng section (mapping VIEW → bảng):**

| Section          | Truy vấn nguồn                                                              |
| ---------------- | -------------------------------------------------------------------------- |
| ĐANG LÀM         | `story WHERE status='in_progress'` ORDER BY created_at                      |
| CẦN PROOF        | `story WHERE status='implemented' AND (verify pass=false OR proof col=0)`   |
| RESUME           | `trace WHERE outcome IN ('partial','blocked')` + `story.next_action` (P3)   |
| BACKLOG MỞ       | `backlog WHERE status IN ('proposed','accepted')` ORDER BY risk desc        |
| INTERVENTION     | `intervention` ORDER BY created_at DESC                                     |
| HOẠT ĐỘNG GẦN    | `trace` ORDER BY created_at DESC + score (đã có)                            |
| drift header     | tái dùng logic `audit` (entropy score) — 1 dòng, không chạy lại full audit  |

### 2.3 Cắm vào workflow (GĐ0)

Thêm vào `01-WORKFLOW.md` GĐ0 (Orient), SAU `KNOWLEDGE_INDEX`:

> **0b. State digest (CÓ ĐIỀU KIỆN):** Nếu lượt này CHẠY workflow 7-GĐ (sẽ ghi
> durable state) HOẶC câu hỏi là về _trạng thái dự án_ ("đang/đã/cần làm gì",
> "story X tới đâu") ⇒ chạy `harness-cli query status` để định hướng. Nếu là hỏi-đáp về
> _nội dung_ code/doc một-bước (không chạy workflow) ⇒ BỎ QUA (xem luật skip
> `00-AGENTS.md §3`). Mơ hồ nhưng đụng workflow ⇒ chạy (bất đối xứng chi phí:
> chạy thừa = vài trăm token; quên = chọn nhầm/nhân bản).

Cập nhật `00-AGENTS.md §3`: nêu rõ `status` dùng CHUNG predicate với Execution
Tracker (một predicate điều khiển cả hai).

### 2.4 Acceptance criteria

- `harness-cli query status` chạy với db rỗng → in các section rỗng, exit 0 (không
  crash).
- Với db có ≥6 story `in_progress` và `--limit 5` → in 5 dòng + "(+N nữa)".
- `--json` trả object có khóa cho mọi section; mỗi phần tử truy về id nguồn.
- Verify command: `harness-cli query status --json | <assert có khóa active/needs_proof/resume/backlog/interventions/recent>`.

### 2.5 Open questions

- ~~`status` top-level verb hay `query status`?~~ → **ĐÃ CHỐT: `query status`**
  (subcommand dưới `query`, cùng họ với `query stats`/`matrix`/`backlog` — view
  thuần thuộc về `query`). Hệ quả: `recap` (cũng view thuần) theo cùng họ →
  `query recap`; `done-check` mang ngữ nghĩa gate/exit-code nên giữ top-level
  (cùng họ `story verify`/`verify-all`).
- Có cache drift/entropy để header không tốn 1 lần audit mỗi lần status? → v1:
  tính nhẹ inline; nếu chậm, cân nhắc cột cache ở lần `audit` cuối.

---

## 3. P2 — Evidence / Artifact store (`harness-cli evidence`)

### 3.1 Vấn đề

`story verify` chỉ lưu pass/fail (1/0); log stdout/stderr "bốc hơi" sau khi agent
đọc. "Cửa ải Bằng chứng" (GĐ4) bắt đọc log nhưng log không bền vững ⇒ mâu thuẫn
mục tiêu "có bằng chứng, có artifact". Story đã có cột `evidence TEXT` nhưng chỉ
là free-text, không có hash/đường dẫn/kiểm chứng lại.

### 3.2 Thiết kế

**Schema — migration `007-evidence.sql`:**

```sql
-- Harness v0 schema - migration 007
-- Durable pointer to local evidence artifacts (gitignored on disk).
CREATE TABLE evidence (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    story_id    TEXT REFERENCES story(id),
    trace_id    INTEGER REFERENCES trace(id),
    kind        TEXT NOT NULL
                CHECK(kind IN ('log','diff','screenshot','report','file')),
    path        TEXT NOT NULL,        -- đường dẫn local (gitignored)
    sha256      TEXT NOT NULL,        -- hash nội dung artifact
    bytes       INTEGER,
    digest      TEXT,                 -- trích ngắn (head/tail) cho người đọc nhanh
    command     TEXT,                 -- lệnh sinh ra artifact (nếu có)
    result      TEXT                  -- với kind='log' từ verify: pass/fail
                CHECK(result IN ('pass','fail') OR result IS NULL),
    source      TEXT NOT NULL DEFAULT 'agent'
                CHECK(source IN ('agent','human','ci','reviewer')),
    notes       TEXT
);
-- Dedup key cho auto-capture "keep-last-per-story": mỗi (story, kind, result)
-- giữ ĐÚNG 1 row mới nhất (xóa row+file cũ trước khi chèn mới). Index hỗ trợ tra.
CREATE INDEX idx_evidence_keeplast ON evidence(story_id, kind, result);
INSERT INTO schema_version (version) VALUES (7);
```

Quan hệ với `story.evidence` (free-text cũ): GIỮ để tương thích ngược; bảng mới
là nguồn có cấu trúc. `status`/`done-check`/`recap` đọc bảng `evidence`. Free-text
chỉ còn là ghi chú phụ (có thể deprecate ở decision riêng).

**CLI:**

```bash
harness-cli evidence add --kind <k> --path <p> \
   [--story <id>] [--trace <id>] [--command "<cmd>"] [--source <s>] [--notes "<t>"]
harness-cli evidence list [--story <id>] [--trace <id>] [--kind <k>] [--json]
```

Hành vi `evidence add`:

1. Đọc file `--path`, tính `sha256` + `bytes`.
2. Sinh `digest`: với text (`log`/`diff`/`report`) = N dòng đầu + N dòng cuối
   (mặc định 20+20); với nhị phân (`screenshot`/`file`) = `<mime/size>` only.
3. Copy artifact vào `_harness/evidence/<story-or-trace-key>/<created_at>-<kind><ext>`
   (đường dẫn chuẩn hóa) — thư mục gitignored.
4. Insert row trỏ tới đường dẫn đã chuẩn hóa.

**Auto-capture — DEFAULT-ON + keep-last-per-story (ĐÃ CHỐT):**

`harness-cli story verify <id>` **mặc định tự ghi** stdout+stderr vào evidence
store (không cần cờ). Lý do: P2 sinh ra để diệt failure mode "log bốc hơi do
agent quên lưu"; nếu để opt-in thì lại dựa vào kỷ luật agent — đúng vấn đề cũ.
Cờ `--no-capture` là cửa thoát cho lần chạy nháp.

Hành vi mỗi lần verify (mặc định):

1. Ghi log với `kind='log'`, `command=<verify_command>`, `source='agent'`,
   `result=<pass|fail>`, link `story_id`. Trả evidence id trong output để trace
   tham chiếu.
2. **Dedup `keep-last-per-story`:** trước khi chèn, XÓA row + file của cùng
   `(story_id, kind='log', result)` cũ. ⇒ mỗi (story, kết quả) chỉ giữ **bản mới
   nhất**, không tích lũy theo dev loop. NGOẠI LỆ: khi result CHUYỂN `fail→pass`,
   giữ cả hai (lưu lại bằng chứng đã sửa được).
3. **Dedup nội dung:** nếu `sha256` trùng row hiện có ⇒ không tạo file/row mới,
   chỉ cập nhật `created_at`.

⇒ Proof boolean `1` LUÔN có log tươi đỡ lưng (gate P4 thỏa được, không ma sát),
mà evidence store không phình. Bằng chứng là _mặc định_; sạch sẽ là _cơ chế tự
động_ — không đẩy gánh nặng cho agent.

**Thay đổi repo:** thêm `_harness/evidence/` vào `.gitignore`.

### 3.3 Cắm vào workflow

- **GĐ4 (Validation):** "Cửa ải Bằng chứng" cập nhật — verify với `--capture`;
  proof boolean `1` chỉ hợp lệ khi có evidence row `log` pass tương ứng (P4
  enforce).
- **GĐ5 (Trace):** artifact phi-verify (screenshot E2E, report) ghi qua
  `evidence add`; nêu evidence id ở `--notes` của trace.

### 3.4 Acceptance criteria

- `evidence add --kind log --path <f>` → row có sha256 đúng (đối chiếu
  `sha256sum`), file xuất hiện dưới `_harness/evidence/...`, `_harness/evidence/`
  nằm trong `.gitignore`.
- `story verify <id> --capture` fail → vẫn tạo evidence `log` với nội dung lỗi.
- `evidence list --story <id> --json` → mảng row truy về story.

### 3.5 Open questions

- ~~Default-on capture hay opt-in?~~ → **ĐÃ CHỐT: default-on + keep-last-per-story**
  (xem §3.2). Cờ `--no-capture` cho lần nháp. Overhead nhỏ vì verify thường chạy
  test (vốn chậm); keep-last + dedup sha256 chặn rác.
- Dọn artifact cũ (retention)? → v2: `evidence prune --older-than`. Ngoài scope v1.
- Khi `--path` là nhị phân lớn (screenshot) → chỉ hash + copy, digest = metadata.

---

## 4. P3 — Next-action / Resume continuity

### 4.1 Vấn đề

Trace `partial`/`blocked` không có con trỏ "tiếp tục từ đâu". Phiên sau mất ngữ
cảnh "đang làm gì". Đây là mảnh ghép WIP-handoff giữa các agent.

### 4.2 Thiết kế

**Schema — migration `006-next-action.sql`:**

```sql
-- Harness v0 schema - migration 006
-- Resume hint for WIP continuity across sessions.
ALTER TABLE story ADD COLUMN next_action TEXT;
ALTER TABLE story ADD COLUMN next_action_at TEXT;
ALTER TABLE trace ADD COLUMN next_action TEXT;
INSERT INTO schema_version (version) VALUES (6);
```

`story.next_action` = con trỏ WIP _sống_ (luôn là "việc kế tiếp" hiện tại).
`trace.next_action` = bản ghi bất biến tại thời điểm trace.

**CLI:**

- `trace` thêm `--next-action "<text>"`.
- `story update` thêm `--next-action "<text>"` (và set `next_action_at`).

**Enforcement (lifecycle — chống nghĩa địa TODO):**

1. `IF outcome IN ('partial','blocked')` ⇒ `--next-action` **BẮT BUỘC** (CLI từ
   chối rỗng, exit !=0). Nếu trace có `--story`, ghi luôn vào `story.next_action`.
2. `IF outcome == 'completed'` và trace có `--story` ⇒ tự **clear**
   `story.next_action` (set NULL). "Resume hint" không tồn tại quá việc đã xong.

**Surface:** section RESUME của `status` (P1) đọc `story.next_action` +
`trace.next_action` của trace partial/blocked gần nhất.

### 4.3 Cắm vào workflow

- **GĐ5 (Trace):** bổ sung luật "outcome partial/blocked ⇒ ghi next_action" vào
  Friction & Failure Attribution. Cập nhật `docs/TRACE_SPEC.md` tier rules.
- **GĐ0:** hint nổi lên qua `status`.

### 4.4 Acceptance criteria

- `trace --outcome blocked` không có `--next-action` → exit !=0, thông báo rõ.
- `trace --outcome completed --story US-X` → `story.next_action` về NULL.
- `status` hiển thị next_action ở RESUME.

### 4.5 Open questions

- Có cần next_action cho `failed` không? → Có, cùng nhóm partial/blocked (việc còn
  dở). Spec: enforce cho `partial|blocked|failed`, clear cho `completed`.

---

## 5. P4 — Done-check gate (`harness-cli done-check`)

### 5.1 Vấn đề

GĐ7 ("Done") hiện là văn xuôi; agent tự khẳng định đủ điều kiện ⇒ false-"done".
Các check rời (`verify-all`, cảnh báo trace) chưa được đóng gói thành một gate
lane-aware.

### 5.2 Thiết kế

**Schema:** KHÔNG (aggregator read + exit code). Đây là gói các check sẵn có +
P2/P3, KHÔNG thêm logic store.

**CLI:**

```bash
harness-cli done-check [--story <id>] [--intake <id>] [--json]
```

**Assertion lane-aware** (exit 0 = tất cả pass, 1 = bất kỳ fail):

| Check                                                            | tiny | normal | high-risk |
| --------------------------------------------------------------- | ---- | ------ | --------- |
| Có ≥1 trace link tới story/intake                               | ✔    | ✔      | ✔         |
| `story.status == 'implemented'`                                 | –    | ✔      | ✔         |
| `verify_command` set & `last_verified_result == 'pass'`         | –    | ✔      | ✔         |
| Có evidence `log` pass tương ứng verify (P2)                    | –    | ✔      | ✔         |
| Proof columns khai báo đều = 1 (theo Test Matrix)               | –    | ✔      | ✔         |
| `story.next_action` đã clear (không còn WIP treo)               | –    | ✔      | ✔         |
| 4 neo high-risk packet tồn tại (overview/execplan/design/valid.) | –   | –      | ✔         |

Output: checklist `✔/✘` từng dòng + lý do fail. `--json` cho host.

> **Lưu ý cân nhắc (open):** P4 _trùng một phần_ với `verify-all` + cảnh báo
> trace. Giá trị riêng = ĐÓNG GÓI lane-aware thành một gate GĐ7. Phương án rẻ
> hơn: chỉ nâng GĐ7 thành checklist cơ học trong doc + tái dùng `verify-all`,
> CHƯA cần lệnh mới. → Quyết định ở §8 (xếp ưu tiên thấp nhất).

### 5.3 Cắm vào workflow (GĐ7)

GĐ7 đổi từ văn xuôi "Một tác vụ chỉ xong khi..." thành: "BẮT BUỘC chạy
`harness-cli done-check --story <id>`; exit !=0 ⇒ chưa done, xử lý fan-out hoặc
ghi backlog."

### 5.4 Acceptance criteria

- Story normal thiếu proof → `done-check` exit 1, liệt kê dòng `✘`.
- Story tiny chỉ cần trace → exit 0.
- `--json` trả `{passed: bool, checks: [...]}`.

---

## 6. P5 — Recap rollup (`harness-cli query recap`)

### 6.1 Vấn đề

Khi trace tích lũy, agent không tiêu hóa nổi "đã làm gì". `HARNESS_COMPONENTS.md`
đã tự nhận "summarize old traces" là future work.

### 6.2 Thiết kế

**Schema:** KHÔNG (pure read view).

**CLI:**

```bash
harness-cli query recap [--story <id>] [--epic <prefix>] [--since <YYYY-MM-DD>] [--json]
```

**Output — rollup TẤT ĐỊNH, templated (KHÔNG văn tóm tắt ngữ nghĩa):**

```text
RECAP  story=US-029  (2026-06-01 → 2026-06-15, 7 traces)

Outcome:    completed 4 · partial 2 · blocked 1 · failed 0
Files đụng: src/foo.rs (5) · schema/007.sql (3) · ...   (top theo tần suất)
Friction:   Data model (3) · Task specification (1)      (gom theo 11 Component)
Decisions:  0007-improvement-rules, ...
Intervention: override 1 · approval 1
```

> **Giới hạn thành thật:** không có LLM ⇒ recap chỉ là rollup đếm/gom theo
> template. Tóm tắt ngữ nghĩa là việc của _agent đọc recap thô_, KHÔNG phải CLI.
> Giá trị: thay vì đọc 50 row trace, agent đọc 6 dòng (tiết kiệm token GĐ0).

### 6.3 Cắm vào workflow

- **GĐ0:** orient lịch sử khi tiếp tục story dài.
- **GĐ6 (Growth):** đầu vào review friction theo component trước `propose`.

### 6.4 Acceptance criteria

- `recap --story <id>` → counts khớp `query traces` lọc tay.
- `recap --epic E01` → gộp mọi story khớp prefix.
- `--json` tất định (cùng db → cùng output).

---

## 7. Tác động maturity & tài liệu phải cập nhật

**Maturity (`docs/HARNESS_MATURITY.md`):**

- P1+P5 củng cố **Observability** (read-model động) — bước tới H3 full (vẫn cần
  benchmark attribution, ngoài scope spec này).
- P2 củng cố **Verification** (proof có bằng chứng durable, không chỉ boolean).
- P4 củng cố **Verification gate** GĐ7 (chống false-done).
- P3 củng cố **Task state** (WIP continuity).
- KHÔNG được claim nâng level chỉ vì thêm lệnh — phải có trace/benchmark chứng
  minh (giữ Rào cản Maturity §GĐ7).

**Docs phải sửa khi triển khai:**

| File                          | Sửa gì                                                          |
| ----------------------------- | -------------------------------------------------------------- |
| `00-AGENTS.md`                | §3: `status` chung predicate skip; nêu tầng Read-Model         |
| `01-WORKFLOW.md`              | GĐ0 thêm bước 0b (status có-điều-kiện); GĐ4/5/7 cập nhật gate   |
| `03-CLI_REFERENCE.md`         | Thêm cú pháp `query status`/`query recap`/`evidence`/`done-check`+`--next-action` |
| `docs/CLI_REFERENCE.md`       | Ngữ nghĩa sâu + ví dụ từng lệnh                                |
| `docs/TRACE_SPEC.md`          | next_action enforcement theo tier; evidence id ở notes        |
| `docs/HARNESS_COMPONENTS.md`  | Cập nhật Observability/Verification status + file inventory    |
| `.gitignore`                  | Thêm `_harness/evidence/`                                      |
| `scripts/schema/`             | `006-next-action.sql`, `007-evidence.sql`                      |

**Decision records cần tạo (GĐ2 — vì đụng taxonomy CLI/observability):**

- Read-Model là tầng first-class-có-điều-kiện (đổi workflow GĐ0).
- Evidence gitignore + db giữ pointer/hash (đụng verification proof model).

---

## 8. Kế hoạch xé Story (slicing) & thứ tự triển khai

Xé theo phụ thuộc + giá trị. Mỗi dòng = 1 story packet.

| Thứ tự | Story (dự kiến)                         | Lane      | Phụ thuộc | Risk flags chính           |
| ------ | --------------------------------------- | --------- | --------- | -------------------------- |
| 1      | P3 schema 006 next_action + enforcement | normal    | —         | Data model                 |
| 2      | P2 schema 007 evidence + add/list       | high-risk | —         | Data model, weak proof     |
| 3      | P2 auto-capture trên `story verify`     | normal    | #2        | Existing behavior          |
| 4      | P1 `status` read-model (đọc 006/007)    | normal    | #1,#2     | Public contracts (CLI)     |
| 5      | P5 `recap` rollup                       | normal    | —         | Public contracts (CLI)     |
| 6      | P4 `done-check` gate (HOẶC doc-only)    | normal    | #2,#3     | Validation gate            |
| 7      | Doc sync (00/01/03 + maturity)          | tiny→norm | tất cả    | Existing behavior          |

**Ghi chú ưu tiên:** P1 (#4) là đòn bẩy lớn nhất nhưng phụ thuộc 006/007 nên xếp
sau. Bộ tối thiểu giá-trị-cao = **#1 + #2 + #4** (đã làm/đang làm/cần làm +
bằng chứng). P4 (#6) cân nhắc làm doc-only trước (rẻ), lên lệnh sau nếu cần.

**Lệnh khởi động (khi bắt đầu triển khai thật):**

```bash
harness-cli intake --type "harness improvement" \
  --summary "Read-model status + evidence store + resume/recap/done-check" --lane high_risk
# rồi tạo story packet cho từng dòng bảng trên (GĐ2), kèm 2 decision records §7.
```

---

## 9. Tóm tắt quyết định đã chốt với người dùng

1. **First-class-có-điều-kiện** cho Read-Model (P1) — gắn luật skip §3, KHÔNG
   đánh thuế lượt hỏi-đáp/hỏi-logic.
2. **Evidence gitignore** — db giữ pointer/hash/digest; artifact thô nằm local.
   Đánh đổi mất tái-dựng qua máy khác đã chấp nhận.
3. **Taxonomy lệnh đọc** — view thuần vào dưới `query`: `query status`,
   `query recap`. Gate `done-check` giữ top-level (ngữ nghĩa exit-code).
4. **Auto-capture P2: default-on + keep-last-per-story** — `story verify` tự ghi
   log; cờ `--no-capture` cho lần nháp; dedup `(story,kind,result)` + sha256.
