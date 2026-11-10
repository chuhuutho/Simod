# Simod – Quản lý cửa hàng tạp hóa

Phần mềm desktop viết bằng **Rust**: giao diện **Leptos** (CSR/WASM) + lõi **Tauri 2** + **SQLite**.
Hiện thực theo tài liệu SRS (bản gốc trích ở `docs/SRS.txt`).

## Chạy
Cần: Rust, target `wasm32-unknown-unknown`, `trunk`, `tauri-cli` (v2).

```
cargo tauri dev                  # chạy dev
cargo tauri build                # đóng gói bản cài đặt
cargo test -p simod --test ipc   # kiểm thử nghiệp vụ (IPC + SQLite)
```

Tài khoản mẫu (đổi mật khẩu sau khi dùng): `admin / admin123` (Quản lý), `nhanvien / nhanvien123` (Nhân viên).

## Cấu trúc
- `shared/` – kiểu dữ liệu, thông báo MSGxx, định dạng VNĐ/ngày dùng chung
- `src/` – giao diện Leptos (mỗi màn hình S01–S21 là một page trong `src/pages`)
- `src-tauri/src/` – lõi: `auth`, `catalog` (tài khoản/sản phẩm/NCC), `sales`, `purchase`, `stock` (cảnh báo, kiểm kê, báo cáo, nhật ký), `inv` (tồn kho theo lô), `db`
- Dữ liệu: `%APPDATA%\vn.simod.minimart\simod.db`, sao lưu mỗi ngày vào `backups\` (giữ 30 bản)

## Đã hiện thực
UC01–UC20: đăng nhập (khóa 30' sau 5 lần sai), đổi mật khẩu, quản lý nhân viên, sản phẩm/danh mục, bán hàng + thanh toán + in hóa đơn,
lịch sử hóa đơn, nhà cung cấp, phiếu nhập (chờ xác nhận → cộng kho), tồn kho, cảnh báo sắp hết/cận date (số ngày cấu hình được),
kiểm kê + duyệt điều chỉnh, báo cáo doanh thu ngày/tháng, nhật ký hoạt động (BR-03). Phân quyền theo vai trò ở cả UI lẫn backend.
