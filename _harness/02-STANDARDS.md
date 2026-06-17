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
