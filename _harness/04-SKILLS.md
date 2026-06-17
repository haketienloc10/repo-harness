# Skill chuyên biệt: Hợp đồng & Bảng đăng ký (Registry)

Skill = procedure mô-đun, nạp theo trigger, kiểm chứng được. Skill chỉ _điều
phối_ bước + artifact + bằng chứng sẵn có. KHÁC policy (`02-STANDARDS.md`) và
tra cứu lệnh (`03-CLI_REFERENCE.md`). KHÔNG nhân bản `docs/`.

## 1. Tiêu chí thêm skill (ĐỦ cả ba)

1. Đổi hành vi Agent sẽ không tự làm. KHÔNG thêm skill chỉ kể lại luật đã có
   trong workflow / `02-STANDARDS.md`.
2. Có `VERIFY` (cổng cơ học) + `EXIT` rõ. Không kiểm chứng được → không phải
   skill.
3. Nạp theo trigger, ≤ ~1 trang, trỏ tới `docs/` thay vì sao chép.

## 2. Nạp Skill (on-demand)

- Tới Giai đoạn ở cột "Giai đoạn" của registry → quét bảng §5 → đọc ĐÚNG 1 file
  skill khớp trigger.
- KHÔNG preload `skills/*`.

## 3. Vòng đời

- **Thêm:** chỉ qua GĐ6 (Growth) khi friction #4 (bước thủ công lặp lại) hoặc
  thiếu capability. Tạo từ `skills/_TEMPLATE.md`. Là _Harness Delta_.
- **Sửa/Loại bỏ:** _Harness Delta_ — ghi lý do (decision/backlog). Skill không
  còn đổi hành vi / không đo được → loại bỏ.
- **Ràng buộc:** ≤ ~1 trang; BẮT BUỘC có `VERIFY` + `EXIT`.

## 4. Khuôn

Theo `skills/_TEMPLATE.md`: header (Trigger, Lane, Giai đoạn) + 6 mục `INPUT` →
`STEPS` → `VERIFY` → `ARTIFACTS` → `FRICTION HOOKS` → `EXIT`. Tên file
kebab-case `động từ-danh từ`.

## 5. Bảng đăng ký (Registry)

| Trigger                                                                                       | Skill file                           |
| --------------------------------------------------------------------------------------------- | ------------------------------------ |
| `IF [GĐ3, TRƯỚC khi viết code logic, task khóa-behavior]` (normal/high; bỏ tiny/UI/prototype) | `skills/tdd-red-green.md`            |
| `IF [GĐ3 xong code logic, TRƯỚC khi sang GĐ4]` (normal/high)                                  | `skills/quality-gate-review.md`      |
| `IF [tạo/làm mới docs/KNOWLEDGE_INDEX.md, hoặc knowledge check báo lỗi]` (mọi lane; GĐ2/GĐ6)  | `skills/generate-knowledge-index.md` |

> Chỉ thêm dòng khi procedure NẶNG/ĐẶC THÙ đổi được hành vi. Map
> `Trigger → skills/<tên>.md`.

> **Bản đồ tên (knowledge index — một quy trình, ba tầng):** procedure thật là
> skill harness `generate-knowledge-index`
> (`skills/generate-knowledge-index.md`, dòng đăng ký ở trên). Host có thể bày
> nó ra dưới tên skill/slash-command `knowledge-index` (vd `.claude/skills/`,
> `.agents/skills/`) — đó chỉ là VỎ ủy thác về file trên, KHÔNG nhân bản nội
> dung. Cả hai đều điều phối lệnh CLI `harness-cli knowledge scaffold` +
> `knowledge check`. Khi sửa quy trình, sửa ở file skill harness; vỏ host chỉ
> trỏ về.

## 6. Durable Layer (CLI)

- Trace (GĐ5): nêu skill ở `--actions`/`--notes` (vd
  `"skill: quality-gate-review"`).
- Finding `blocking` → `backlog` (GĐ6) với `--predicted`.
- Lỗi thật → `--friction` quy về 1 trong 11 Responsibilities (danh sách ở
  `01-WORKFLOW.md` GĐ5; mô tả sâu `_harness/docs/HARNESS_COMPONENTS.md`).
- Friction/intervention lặp lại có thể được `harness-cli propose` (GĐ6) tổng hợp
  tất định thành backlog item `proposed`; con người vẫn là cổng duyệt.
