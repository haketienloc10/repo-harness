# Tiêu chuẩn Kỹ thuật & Kiểm thử

## 1. QUY TẮC KIẾN TRÚC (ARCHITECTURE)

**Discovery Before Shape:** Trước khi định hình code/kiến trúc, BẮT BUỘC xác
định: (1) product surfaces (browser/mobile/desktop/CLI/API/worker); (2) runtime
stack (ngôn ngữ, framework, DB, queue, provider, hosting); (3) core domains; (4)
boundary inputs (request, env, webhook, file, credential); (5) validation
ladder. Chỉ tạo folder/scaffold thật khi một Story bước vào triển khai.

```text
domain <- application <- infrastructure <- interface <- app surfaces
```

**Dependency Rule (Quy tắc phụ thuộc):** Inner layers KHÔNG phụ thuộc outer
layers.

| Tầng (Layer)       | Được phép phụ thuộc vào                  | KHÔNG được phụ thuộc vào                           |
| ------------------ | ---------------------------------------- | -------------------------------------------------- |
| **domain**         | Không gì cả (trừ pure utilities)         | framework, database, UI, provider, process/env     |
| **application**    | domain                                   | framework, UI, provider, database concrete clients |
| **infrastructure** | domain, application                      | interface controllers hoặc UI                      |
| **interface**      | tất cả backend layers (domain/app/infra) | UI state hoặc platform shell assumptions           |
| **app surfaces**   | API contracts và app-facing clients      | domain internals trực tiếp                         |

- **Parse-First Boundary:** Dữ liệu chưa rõ định dạng (HTTP request, env vars,
  rows từ DB, webhooks,...) PHẢI được parse thành typed DTO/Command trước khi
  vào application/domain. Code bên trong thao tác bằng Type (`UserId`,
  `DateRange`), không thao tác bằng raw string.
- **Command/Query Separation:** Command xử lý ghi (đổi trạng thái, sinh audit
  log). Query xử lý đọc và format cho consumer. Luật domain dùng chung nằm ở
  tầng domain/application, KHÔNG nằm ở controller.
- **Observability Contract:** Một request = Một dòng JSON log chuẩn gồm:
  `timestamp`, `level`, `request_id`, `user_id`, `action`, `duration_ms`,
  `status_code`, `message`. Audit log là _product record_, application log là
  _operational record_ — KHÔNG dùng loại này thay loại kia.

> Chi tiết sâu (on-demand): danh sách đầy đủ boundary inputs phải parse
> (identity claims, signed URLs/token, shell payloads…), target flow
> `unknown input → parser → typed DTO → use case → domain object`, và cây
> Candidate Structure xem `_harness/docs/ARCHITECTURE.md`.

## 2. MA TRẬN KIỂM THỬ (TEST MATRIX)

KHÔNG đánh dấu row là `implemented` nếu chưa có test hoặc bằng chứng validation.

- **Status vocabulary:** `planned` (đã chấp nhận là hành vi mục tiêu, chưa làm),
  `in_progress` (đang xây), `implemented` (đã làm VÀ có proof), `changed` (hợp
  đồng đổi sau khi đã implement), `retired` (không còn thuộc product contract).
- **Evidence rule:** một story CÓ THỂ là `implemented` mà không đủ mọi cột proof
  NẾU story packet giải thích lý do.

- **Unit:** Kiểm chứng pure domain và application rules.
- **Integration:** Kiểm chứng backend enforcement, data integrity, provider
  behavior, jobs, hoặc service contracts.
- **E2E:** Kiểm chứng user-visible browser flows.
- **Platform:** Kiểm chứng shell, deployment, mobile, desktop, hoặc runtime
  behavior — những thứ không test được ở tầng dưới.

## 3. KINH TẾ CODE (Code Economy — Lazy Senior Lens)

Code không-viết là code tốt nhất. Áp ở GĐ3 (Implementation), TRƯỚC khi gõ logic.
Lens này CHẶN code thừa — KHÔNG thay thế hay hạ bất kỳ chuẩn nào ở §1/§2.

**Nấc thang quyết định — dừng ở nấc ĐẦU TIÊN đứng vững:**

1. Có cần xây không? (YAGNI — khớp `00-AGENTS.md` §1: thành phần chỉ xuất hiện
   khi một Story cần đến.)
2. Thư viện chuẩn (stdlib) đã làm việc này chưa? Dùng nó.
3. Nền tảng/runtime có sẵn tính năng phủ được? Dùng nó.
4. Dependency đã cài có giải quyết được? Dùng nó. (KHÔNG thêm dep mới nếu tránh
   được — thêm dep ngoài chạm cờ _External systems_ ở GĐ1.)
5. Gói được thành một dòng đúng đắn? Gói một dòng.
6. Chỉ khi đó: viết lượng code tối thiểu chạy được.

**Ràng buộc (lazy = ÍT CODE, KHÔNG phải ẩu):**

- Xóa hơn thêm; nhàm chán hơn khôn lỏi; ít file nhất.
- KHÔNG tạo abstraction không được yêu cầu rõ. **NGOẠI LỆ:** layering theo
  Dependency Rule (§1) là abstraction BẮT BUỘC của kiến trúc — KHÔNG tính là
  "abstraction thừa", không được bỏ để "lazy".
- Khi hai cách stdlib cùng kích thước, chọn cách ĐÚNG edge-case — lazy nghĩa là
  ít code, KHÔNG phải thuật toán mỏng manh hơn.
- **KHÔNG lazy ở:** validate input biên (Parse-First §1), error handling chống
  mất dữ liệu, security, accessibility, hiệu chỉnh phần cứng/clock/sensor thật,
  và bất cứ thứ gì được yêu cầu rõ ràng.

**Đánh dấu shortcut có chủ đích — `harness:` comment + backlog:** khi cố ý dùng
shortcut có trần đã biết (global lock, O(n²) scan, heuristic ngây thơ), để lại
một dòng `harness:` comment TẠI CHỖ nêu trần + đường nâng cấp — cảnh báo người /
agent đọc code sau rằng đây là shortcut cố ý, đừng "sửa nhầm" tưởng là bug. Khi
trần ĐÁNG KỂ → ngoài comment, BẮT BUỘC thêm
`harness-cli backlog add --predicted "<trần + lối nâng cấp>"` (GĐ6): comment cho
người đọc tại chỗ, backlog cho harness theo dõi nghĩa vụ. KHÔNG để một nghĩa vụ
bền vững chỉ sống trong comment inline (comment dễ mục, durable layer thì
không).

**Proof tối thiểu (SÀN, không phải trần):** logic không-trivial để lại ÍT NHẤT
một check chạy được. ĐÂY CHỈ LÀ SÀN cho lane tiny — lane normal/high-risk vẫn
theo Validation Ladder + Test Matrix (§2, GĐ4); "one runnable check, no
framework" KHÔNG được dùng để hạ bar proof của lane cao hơn. One-liner trivial
khỏi cần test.
