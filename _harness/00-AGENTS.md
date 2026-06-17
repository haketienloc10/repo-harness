# Giai đoạn 0: Entrypoint & Triết lý cốt lõi

Dự án sử dụng **Harness** — hệ điều hành cấp repository giúp Agent biến yêu cầu
thành thay đổi an toàn.

## 1. Triết lý cốt lõi

- **Harness v0 Scope:** Hệ thống hiện tại cố tình KHÔNG BAO GỒM stack ứng dụng,
  mã nguồn scaffold, hoặc spec dự án nguyên khối. Các thành phần này chỉ xuất
  hiện khi một Story cần đến chúng.
- **Bản đồ Orient (ĐỌC TRƯỚC — là ROUTER, KHÔNG phải nguồn sự thật):** Luôn mở
  `docs/KNOWLEDGE_INDEX.md` đầu tiên để định hướng (Purpose, Top-Level
  Structure, Key Technologies, Key Concepts) và biết đi tiếp vào đâu trong
  Hierarchy bên dưới. KHI MÂU THUẪN, nguồn trong Hierarchy THẮNG — index không
  bao giờ ghi đè nguồn bền vững.
  - **Giới hạn freshness (QUAN TRỌNG — NGUỒN DUY NHẤT, nơi khác chỉ trỏ về
    đây):** `harness-cli knowledge check` chỉ là cổng THÔ — bắt: thiếu section,
    còn `TODO`, và thêm/bớt mục ở cấp repo-root + Key Subdirectories (thư mục
    con cấp 2). Nó KHÔNG bắt: thay đổi sâu hơn cấp 2, hay Purpose / Key Concepts
    / mô tả đã lỗi thời (ngữ nghĩa), hay tech-list sai. ⇒ **check ĐỎ = chắc chắn
    cũ** → làm mới NGAY qua skill `generate-knowledge-index` (GĐ2/GĐ6); **check
    XANH = CHƯA chắc tươi** → chỉ coi index là điểm orient, luôn ưu tiên
    Hierarchy, và chủ động làm mới (scaffold + soạn lại Purpose/Concepts) ở
    GĐ2/GĐ6 khi cấu trúc / tech stack đổi — đừng đợi check đỏ.
- **Read-Model (tầng đọc-định-hướng — VIEW, KHÔNG phải nguồn sự thật):**
  `harness-cli query status` (đang/đã/cần làm gì) và `harness-cli query recap`
  (rollup trace) là VIEW dẫn xuất tất định từ Durable Layer — KHÔNG có schema
  riêng, mọi byte truy về được `story`/`trace`/`backlog`/`intervention`. Dùng
  để orient ở GĐ0 (xem luật skip §3 + `01-WORKFLOW.md` GĐ0 bước 0b).
- **Hierarchy (Phân cấp Nguồn sự thật):** Spec người dùng -> `docs/product/*` ->
  `docs/stories/*` -> `_harness/bin/harness-cli query matrix` ->
  `docs/decisions/*`. KHÔNG mở rộng một file Spec nguyên khối; hãy cập nhật các
  file docs nhỏ hơn.
- **Durable Layer:** Chính sách nằm ở file Markdown, còn dữ liệu vận hành
  (intake, story, quyết định, trace, **intervention**, **tool registry**,
  **evidence**) PHẢI lưu bằng SQLite (`harness.db`) thông qua CLI. BẮT BUỘC tra cứu
  `_harness/03-CLI_REFERENCE.md` (cheatsheet: quy ước giá trị + cú pháp mọi lệnh
  `harness-cli`); flag đầy đủ qua `harness-cli <cmd> --help`; ngữ nghĩa/ví dụ
  sâu ở `_harness/docs/CLI_REFERENCE.md` (on-demand, chỉ đọc mục cần).
- **Tài liệu tham chiếu sâu (khi cần):** `_harness/*` là bộ khung thực thi
  chính. Khi cần chi tiết hơn (mô hình tổng thể, lý do, taxonomy, maturity), tra
  cứu `docs/*` — KHÔNG bắt buộc đọc hết; entrypoint thực thi vẫn là
  `_harness/01-WORKFLOW.md`. Tra cứu thêm khi cần: `_harness/docs/TOOL_REGISTRY.md` (tool
  access), `_harness/docs/HARNESS_AUDIT.md` (entropy/drift),
  `_harness/docs/IMPROVEMENT_PROTOCOL.md` (vòng `propose`), `_harness/docs/HARNESS_COMPONENTS.md`
  (11 Responsibilities).
- **Skill chuyên biệt (nạp on-demand):** `_harness/04-SKILLS.md` là hợp đồng +
  bảng đăng ký skill. KHÔNG preload; chỉ nạp file skill khi trigger khớp ở đúng
  giai đoạn (xem ĐỊNH MỨC TOKEN trong `01-WORKFLOW.md`).

## 2. Đầu ra của một tác vụ

Mỗi tác vụ tạo ra một trong hai (hoặc cả hai) kết quả:

- **Product Delta:** Thay đổi về code, test, API shape, data model, hoặc product
  docs.
- **Harness Delta:** Thay đổi về docs, templates, validation, backlog items,
  hoặc decision records để giúp tác vụ sau dễ dàng hơn.

**[HÀNH ĐỘNG TIẾP THEO]:** Agent BẮT BUỘC chuyển sang đọc
`_harness/01-WORKFLOW.md` để bắt đầu quy trình.

## 3. Định dạng Giao tiếp (UI Output)

Khi đang THỰC THI một tác vụ nhiều bước theo workflow (GĐ1–GĐ7), mở đầu phản hồi
bằng khối hiển thị tiến độ hành động hiện tại (Execution Tracker) theo định dạng
dưới đây. BỎ QUA khối này khi: (a) phản hồi thuần hỏi-đáp / một bước, không chạy
workflow; hoặc (b) host harness đã có cơ chế hiển thị tiến độ / task-tracking
riêng — khi đó dùng cơ chế của host và trả lời thẳng kết quả trước.

**Predicate chung với Read-Model:** điều kiện bật/tắt khối Execution Tracker ở
trên dùng CHUNG một predicate với `query status` (GĐ0 bước 0b): lượt CÓ chạy
workflow 7-GĐ ⇒ hiện tracker + chạy `query status`; lượt hỏi-đáp một-bước ⇒ bỏ
cả hai. Một predicate điều khiển cả hai, không tạo trục phân loại mới.

```text
Action Plan
  └ ✔ [Tên bước đã hoàn thành 1]
    ✔ [Tên bước đã hoàn thành 2]
    ...
    □ [Tên bước đang/sắp thực hiện 10]
    □ [Tên bước sắp thực hiện 11]
```
