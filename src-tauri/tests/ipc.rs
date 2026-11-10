//! Kiểm thử tích hợp qua IPC giả của Tauri: chạy các luồng nghiệp vụ chính trên SQLite thật.
use simod_lib::{db, handler, AppState};
use tauri::Manager;
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{WebviewWindow, WebviewWindowBuilder};

fn setup(name: &str) -> WebviewWindow<MockRuntime> {
    let path = std::env::temp_dir().join(format!("simod-test-{}-{name}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let conn = db::open(&path).unwrap();
    let bk = std::env::temp_dir().join(format!("simod-bk-{}-{name}", std::process::id()));
    db::backup_daily(&conn, &bk);
    let app = mock_builder()
        .manage(AppState::new(conn, bk))
        .invoke_handler(handler())
        .build(mock_context(noop_assets()))
        .unwrap();
    WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap()
}

fn call(w: &WebviewWindow<MockRuntime>, cmd: &str, req: Value) -> Result<Value, String> {
    let res = get_ipc_response(
        w,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: InvokeBody::Json(json!({ "req": req })),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    );
    match res {
        Ok(b) => Ok(b.deserialize::<Value>().unwrap()),
        Err(e) => Err(e.as_str().map(String::from).unwrap_or_else(|| e.to_string())),
    }
}

fn login(w: &WebviewWindow<MockRuntime>, u: &str, p: &str) -> Result<Value, String> {
    call(w, "login", json!({ "username": u, "password": p }))
}

fn product_by_code(w: &WebviewWindow<MockRuntime>, code: &str) -> Value {
    let all = call(w, "list_products", json!({ "search": null, "category_id": null, "only_active": false })).unwrap();
    all.as_array().unwrap().iter().find(|p| p["code"] == code).unwrap().clone()
}

#[test]
fn auth_roles_and_lockout() {
    let w = setup("auth");
    // Chưa đăng nhập
    assert!(call(&w, "list_products", json!({"search":null,"category_id":null,"only_active":true})).is_err());
    // Sai mật khẩu 5 lần -> khóa 30 phút (BR-02)
    for _ in 0..4 {
        assert_eq!(login(&w, "nhanvien", "sai").unwrap_err(), simod_shared::msg::MSG09);
    }
    assert_eq!(login(&w, "nhanvien", "sai").unwrap_err(), simod_shared::msg::MSG12);
    assert_eq!(login(&w, "nhanvien", "nhanvien123").unwrap_err(), simod_shared::msg::MSG12);
    assert_eq!(login(&w, "", "").unwrap_err(), simod_shared::msg::MSG10);
    // Quản lý mở khóa
    let admin = login(&w, "admin", "admin123").unwrap();
    assert_eq!(admin["role"], "Manager");
    let users = call(&w, "list_users", json!({"search":null})).unwrap();
    let staff = users.as_array().unwrap().iter().find(|u| u["username"] == "nhanvien").unwrap();
    assert!(!staff["locked_until"].is_null());
    call(&w, "set_user_active", json!({"id": staff["id"], "active": true})).unwrap();
    // Nhân viên không có quyền quản lý (BR-01)
    login(&w, "nhanvien", "nhanvien123").unwrap();
    assert_eq!(call(&w, "list_users", json!({"search":null})).unwrap_err(), simod_shared::msg::MSG08);
    assert_eq!(call(&w, "revenue_report", json!({"from":"2020-01-01","to":"2030-01-01","group":"day"})).unwrap_err(), simod_shared::msg::MSG08);
    // Đổi mật khẩu
    assert_eq!(call(&w, "change_password", json!({"current":"x","new":"abcdef"})).unwrap_err(), simod_shared::msg::MSG13);
    call(&w, "change_password", json!({"current":"nhanvien123","new":"abcdef"})).unwrap();
    call(&w, "logout", json!({})).unwrap();
    login(&w, "nhanvien", "abcdef").unwrap();
}

#[test]
fn sales_flow() {
    let w = setup("sales");
    login(&w, "nhanvien", "nhanvien123").unwrap();
    let p = product_by_code(&w, "NUOC001");
    assert_eq!(p["stock"], 48);
    let id = p["id"].clone();

    // MSG15 / MSG16 / MSG19 / MSG18
    let bad = |qty: i64, paid: i64| call(&w, "checkout", json!({"lines":[{"product_id":id,"quantity":qty}],"method":"cash","paid":paid}));
    assert_eq!(bad(0, 100000).unwrap_err(), simod_shared::msg::MSG15);
    assert!(bad(999, 100000).unwrap_err().contains("chỉ còn 48"));
    assert!(bad(2, 5000).unwrap_err().contains("Còn thiếu 7.000"));
    assert_eq!(
        call(&w, "checkout", json!({"lines":[],"method":"cash","paid":0})).unwrap_err(),
        simod_shared::msg::MSG18
    );
    assert_eq!(product_by_code(&w, "NUOC001")["stock"], 48); // lỗi -> không trừ kho

    // Thanh toán thành công: 2 x 6.000 = 12.000, khách trả 20.000, thừa 8.000
    let inv = bad(2, 20000).unwrap();
    assert_eq!(inv["row"]["total"], 12000);
    assert_eq!(inv["change"], 8000);
    assert_eq!(inv["row"]["status"], "Đã thanh toán");
    assert!(inv["row"]["code"].as_str().unwrap().starts_with("HD-"));
    assert_eq!(product_by_code(&w, "NUOC001")["stock"], 46);

    // Nhân viên chỉ thấy hóa đơn của mình; Quản lý thấy tất cả + báo cáo
    let q = json!({"search":null,"from":null,"to":null});
    assert_eq!(call(&w, "list_invoices", q.clone()).unwrap().as_array().unwrap().len(), 1);
    login(&w, "admin", "admin123").unwrap();
    call(&w, "checkout", json!({"lines":[{"product_id":id,"quantity":1}],"method":"transfer","paid":6000})).unwrap();
    assert_eq!(call(&w, "list_invoices", q).unwrap().as_array().unwrap().len(), 2);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let rep = call(&w, "revenue_report", json!({"from":"2000-01-01","to":"2999-01-01","group":"month"})).unwrap();
    assert_eq!(rep["total_revenue"], 18000);
    assert_eq!(rep["total_invoices"], 2);
    assert_eq!(rep["rows"][0]["period"].as_str().unwrap().len(), 7);
    let _ = today;
}

#[test]
fn purchase_flow() {
    let w = setup("purchase");
    login(&w, "nhanvien", "nhanvien123").unwrap();
    let p = product_by_code(&w, "NUOC001");
    let pid = p["id"].clone();
    let future = (chrono::Utc::now() + chrono::Duration::days(200)).format("%Y-%m-%d").to_string();
    let line = |exp: Value| json!({"product_id":pid,"quantity":10,"unit_cost":4000,"expiry":exp});

    // Các lỗi kiểm tra
    assert_eq!(call(&w, "save_purchase", json!({"id":null,"supplier_id":null,"lines":[line(json!(future))]})).unwrap_err(), simod_shared::msg::MSG21);
    assert_eq!(call(&w, "save_purchase", json!({"id":null,"supplier_id":1,"lines":[]})).unwrap_err(), simod_shared::msg::MSG24);
    assert_eq!(call(&w, "save_purchase", json!({"id":null,"supplier_id":1,"lines":[line(json!("2001-01-01"))]})).unwrap_err(), simod_shared::msg::MSG23);

    // Tạo phiếu -> Chờ xác nhận, kho chưa đổi
    let saved = call(&w, "save_purchase", json!({"id":null,"supplier_id":1,"lines":[line(json!(future))]})).unwrap();
    let id = saved["id"].clone();
    assert_eq!(product_by_code(&w, "NUOC001")["stock"], 48);
    // Nhân viên không được xác nhận
    assert_eq!(call(&w, "confirm_purchase", json!({"id":id})).unwrap_err(), simod_shared::msg::MSG08);

    // Quản lý từ chối -> Cần chỉnh sửa -> nhân viên sửa lại -> quản lý xác nhận
    login(&w, "admin", "admin123").unwrap();
    assert!(call(&w, "reject_purchase", json!({"id":id,"reason":""})).is_err());
    call(&w, "reject_purchase", json!({"id":id,"reason":"Sai giá"})).unwrap();
    assert_eq!(call(&w, "get_purchase", json!({"id":id})).unwrap()["row"]["status"], "Cần chỉnh sửa");
    login(&w, "nhanvien", "nhanvien123").unwrap();
    call(&w, "save_purchase", json!({"id":id,"supplier_id":1,"lines":[line(json!(future))]})).unwrap();
    login(&w, "admin", "admin123").unwrap();
    call(&w, "confirm_purchase", json!({"id":id})).unwrap();
    assert_eq!(product_by_code(&w, "NUOC001")["stock"], 58); // 48 + 10 (BR-19)
    // Đã xác nhận: không sửa/hủy/xác nhận lại (BR-21)
    assert!(call(&w, "cancel_purchase", json!({"id":id})).is_err());
    assert!(call(&w, "confirm_purchase", json!({"id":id})).is_err());
    assert!(call(&w, "save_purchase", json!({"id":id,"supplier_id":1,"lines":[line(json!(future))]})).is_err());
}

#[test]
fn stocktake_flow_and_alerts() {
    let w = setup("stocktake");
    login(&w, "nhanvien", "nhanvien123").unwrap();
    // Cảnh báo: BANH001 (tồn 8 <= 10) sắp hết; NUOC002 (HSD 20 ngày) cận date
    let a = call(&w, "get_alerts", json!({})).unwrap();
    assert!(a["low"].as_array().unwrap().iter().any(|i| i["code"] == "BANH001"));
    assert!(a["near_expiry"].as_array().unwrap().iter().any(|i| i["code"] == "NUOC002"));

    let st = call(&w, "create_stocktake", json!({})).unwrap();
    let id = st["id"].clone();
    let d = call(&w, "get_stocktake", json!({"id":id})).unwrap();
    let lines = d["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 8);
    let target = lines.iter().find(|l| l["code"] == "BANH001").unwrap()["product_id"].clone();
    let mk = |bump: i64, reason: &str| -> Value {
        let ls: Vec<Value> = lines
            .iter()
            .map(|l| {
                let sys = l["system_qty"].as_i64().unwrap();
                if l["product_id"] == target {
                    json!({"product_id":l["product_id"],"actual":sys+bump,"reason":reason})
                } else {
                    json!({"product_id":l["product_id"],"actual":sys,"reason":""})
                }
            })
            .collect();
        json!({"id":id,"lines":ls,"complete":true})
    };
    // Chênh lệch mà không có lý do -> MSG29 (BR-25)
    assert_eq!(call(&w, "save_stocktake", mk(-3, "")).unwrap_err(), simod_shared::msg::MSG29);
    let r = call(&w, "save_stocktake", mk(-3, "Hao hụt")).unwrap();
    assert_eq!(r["status"], "Chờ duyệt");
    assert_eq!(r["diff_count"], 1);
    assert_eq!(product_by_code(&w, "BANH001")["stock"], 8); // chưa đổi
    // Nhân viên không được duyệt (BR-26)
    assert_eq!(call(&w, "approve_stocktake", json!({"id":id})).unwrap_err(), simod_shared::msg::MSG08);
    login(&w, "admin", "admin123").unwrap();
    call(&w, "approve_stocktake", json!({"id":id})).unwrap();
    assert_eq!(product_by_code(&w, "BANH001")["stock"], 5);
    assert_eq!(call(&w, "get_stocktake", json!({"id":id})).unwrap()["row"]["status"], "Đã điều chỉnh");
    // Nhật ký ghi lại điều chỉnh (BR-03)
    let logs = call(&w, "list_logs", json!({"search":"Điều chỉnh"})).unwrap();
    assert_eq!(logs.as_array().unwrap().len(), 1);
}

#[test]
fn products_and_expiry_rules() {
    let w = setup("products");
    login(&w, "admin", "admin123").unwrap();
    let cats = call(&w, "list_categories", json!({})).unwrap();
    let cat = cats[0]["id"].clone();
    let base = |code: &str, price: i64| json!({"id":null,"code":code,"name":"SP thử","category_id":cat,"unit":"cái","price":price,"min_stock":1,"has_expiry":false,"initial_stock":5,"initial_expiry":null});
    assert!(call(&w, "save_product", base("X1", 0)).is_err()); // BR-04: giá > 0
    call(&w, "save_product", base("X1", 1000)).unwrap();
    assert!(call(&w, "save_product", base("x1", 1000)).unwrap_err().contains("đã tồn tại")); // mã duy nhất
    assert_eq!(product_by_code(&w, "X1")["stock"], 5);

    // Hàng đã hết hạn không bán được: chèn lô hết hạn trực tiếp vào DB rồi bán
    let state = w.state::<AppState>();
    let pid = product_by_code(&w, "NUOC001")["id"].as_i64().unwrap();
    {
        let c = state.db();
        c.execute("UPDATE lo_hang SET expiry='2001-01-01' WHERE product_id=?1", [pid]).unwrap();
    }
    let p = product_by_code(&w, "NUOC001");
    assert_eq!(p["sellable"], 0);
    let err = call(&w, "checkout", json!({"lines":[{"product_id":pid,"quantity":1}],"method":"cash","paid":6000})).unwrap_err();
    assert!(err.contains("đã hết hạn sử dụng"));
    // Cảnh báo cận date có hàng quá hạn (days_left < 0)
    let a = call(&w, "get_alerts", json!({})).unwrap();
    assert!(a["near_expiry"].as_array().unwrap().iter().any(|i| i["code"] == "NUOC001" && i["days_left"].as_i64().unwrap() < 0));
    // Ngừng kinh doanh => không bán được (BR-05)
    call(&w, "set_product_active", json!({"id":pid,"active":false})).unwrap();
    assert!(call(&w, "checkout", json!({"lines":[{"product_id":pid,"quantity":1}],"method":"cash","paid":6000})).is_err());
}
