//! S12/S13 – Danh sách phiếu nhập, tạo / chi tiết / xác nhận phiếu nhập (UC13–UC15)
use crate::api::{fetch, fetch_or};
use crate::ctx::*;
use crate::icons::Icon;
use crate::ui::*;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn PurchasesPage() -> impl IntoView {
    let c = ctx();
    let items = RwSignal::new(Vec::<PurchaseRow>::new());
    let status_f = RwSignal::new(String::new());
    let from = RwSignal::new(String::new());
    let to = RwSignal::new(String::new());
    let search = RwSignal::new(String::new());
    let supplier_f = RwSignal::new(0i64);
    let suppliers = RwSignal::new(Vec::<Supplier>::new());
    fetch(c, "list_suppliers", SupplierQuery { search: None, only_active: false }, move |v: Vec<Supplier>| suppliers.set(v));

    Effect::new(move |_| {
        let req = PurchaseQuery {
            status: Some(status_f.get()).filter(|s| !s.is_empty()),
            supplier_id: Some(supplier_f.get()).filter(|id| *id > 0),
            from: Some(from.get()).filter(|s| !s.is_empty()),
            to: Some(to.get()).filter(|s| !s.is_empty()),
        };
        fetch(c, "list_purchases", req, move |v: Vec<PurchaseRow>| items.set(v));
    });
    let rows = move || {
        let q = search.get();
        items.get().into_iter().filter(|r| matches_query(&q, &[&r.code, &r.supplier_name])).collect::<Vec<_>>()
    };

    view! {
        <PageHead title="Phiếu nhập hàng">
            <button class="btn primary" on:click=move |_| c.go(Page::Purchase(None))>"+ Tạo phiếu nhập"</button>
        </PageHead>
        <div class="toolbar">
            <input class="search" type="search" placeholder="Tìm theo mã phiếu hoặc nhà cung cấp…" prop:value=move || search.get()
                on:input=move |ev| search.set(event_target_value(&ev))/>
            <select on:change=move |ev| status_f.set(event_target_value(&ev))>
                <option value="">"Tất cả trạng thái"</option>
                <option value=status::PN_PENDING>{status::PN_PENDING}</option>
                <option value=status::PN_CONFIRMED>{status::PN_CONFIRMED}</option>
                <option value=status::PN_REJECTED>{status::PN_REJECTED}</option>
                <option value=status::PN_CANCELLED>{status::PN_CANCELLED}</option>
            </select>
            <select on:change=move |ev| supplier_f.set(event_target_value(&ev).parse().unwrap_or(0))>
                <option value="0">"Tất cả nhà cung cấp"</option>
                {move || suppliers.get().into_iter().map(|s| view! { <option value=s.id.to_string()>{s.name}</option> }).collect_view()}
            </select>
            <label class="inline">"Từ"<input type="date" prop:value=move || from.get() on:input=move |ev| from.set(event_target_value(&ev))/></label>
            <label class="inline">"Đến"<input type="date" prop:value=move || to.get() on:input=move |ev| to.set(event_target_value(&ev))/></label>
        </div>
        <div class="card table-card">
            <table class="tbl clickable">
                <thead><tr><th>"Mã phiếu"</th><th>"Nhà cung cấp"</th><th>"Người lập"</th><th>"Ngày lập"</th><th class="num">"Tổng tiền (VNĐ)"</th><th>"Trạng thái"</th></tr></thead>
                <tbody>
                    {move || rows().into_iter().map(|r| {
                        let id = r.id;
                        view! {
                            <tr on:click=move |_| c.go(Page::Purchase(Some(id)))>
                                <td class="mono">{r.code.clone()}</td><td>{r.supplier_name.clone()}</td><td>{r.created_by_name.clone()}</td>
                                <td>{fmt_dt(&r.created_at)}</td><td class="num">{fmt_money(r.total)}</td>
                                <td><StatusBadge text=r.status.clone()/></td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
            <NoResult show=Signal::derive(move || rows().is_empty())/>
        </div>
    }
}

#[derive(Clone)]
struct LineForm {
    key: u32,
    product_id: i64,
    code: String,
    name: String,
    unit: String,
    has_expiry: bool,
    qty: ArcRwSignal<String>,
    cost: ArcRwSignal<String>,
    expiry: ArcRwSignal<String>,
}

#[component]
pub fn PurchasePage(id: Option<i64>) -> impl IntoView {
    let c = ctx();
    let manager = c.is_manager();
    let detail = RwSignal::new(None::<PurchaseDetail>);
    let editing = RwSignal::new(id.is_none());
    let suppliers = RwSignal::new(Vec::<Supplier>::new());
    let products = RwSignal::new(Vec::<Product>::new());
    let supplier_id = RwSignal::new(None::<i64>);
    let lines = RwSignal::new(Vec::<LineForm>::new());
    let next_key = RwSignal::new(0u32);
    let psearch = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let reject_modal = RwSignal::new(false);
    let reason = RwSignal::new(String::new());
    let cancel_confirm = RwSignal::new(false);

    fetch(c, "list_suppliers", SupplierQuery { search: None, only_active: true }, move |v: Vec<Supplier>| suppliers.set(v));
    fetch(c, "list_products", ProductQuery { search: None, category_id: None, only_active: true }, move |v: Vec<Product>| products.set(v));

    let fill_form = move |d: &PurchaseDetail| {
        supplier_id.set(Some(d.supplier_id));
        let mut k = next_key.get_untracked();
        let ls = d
            .lines
            .iter()
            .map(|l| {
                k += 1;
                LineForm {
                    key: k,
                    product_id: l.product_id,
                    code: l.code.clone(),
                    name: l.name.clone(),
                    unit: l.unit.clone(),
                    has_expiry: l.has_expiry,
                    qty: ArcRwSignal::new(l.quantity.to_string()),
                    cost: ArcRwSignal::new(l.unit_cost.to_string()),
                    expiry: ArcRwSignal::new(l.expiry.clone().unwrap_or_default()),
                }
            })
            .collect();
        next_key.set(k);
        lines.set(ls);
    };

    let load = move || {
        if let Some(id) = id {
            fetch(c, "get_purchase", IdReq { id }, move |d: PurchaseDetail| {
                fill_form(&d);
                detail.set(Some(d));
            });
        }
    };
    load();

    let add_product = move |p: Product| {
        let exists = lines.get_untracked().iter().any(|l| l.product_id == p.id && !l.has_expiry);
        if exists {
            return c.err(format!("Sản phẩm {} đã có trong phiếu.", p.name));
        }
        let k = next_key.get_untracked() + 1;
        next_key.set(k);
        lines.update(|v| {
            v.push(LineForm {
                key: k,
                product_id: p.id,
                code: p.code.clone(),
                name: p.name.clone(),
                unit: p.unit.clone(),
                has_expiry: p.has_expiry,
                qty: ArcRwSignal::new("1".into()),
                cost: ArcRwSignal::new("0".into()),
                expiry: ArcRwSignal::new(String::new()),
            })
        });
        psearch.set(String::new());
    };

    let matches = move || {
        let q = psearch.get();
        if q.trim().is_empty() {
            return Vec::new();
        }
        products.get().into_iter().filter(|p| matches_query(&q, &[&p.name, &p.code])).take(8).collect::<Vec<_>>()
    };

    let total = move || {
        lines
            .get()
            .iter()
            .map(|l| parse_num(&l.qty.get()).unwrap_or(0) * parse_num(&l.cost.get()).unwrap_or(0))
            .sum::<i64>()
    };

    let save = move |_| {
        error.set(String::new());
        let ls = lines.get_untracked();
        if supplier_id.get_untracked().is_none() {
            return error.set(msg::MSG21.into());
        }
        if ls.is_empty() {
            return error.set(msg::MSG24.into());
        }
        let mut out = Vec::new();
        for l in &ls {
            let Some(q) = parse_num(&l.qty.get_untracked()).filter(|q| *q > 0) else { return error.set(msg::MSG15.into()) };
            let Some(cost) = parse_num(&l.cost.get_untracked()).filter(|c| *c >= 0) else {
                return error.set("Giá nhập phải là số nguyên không âm.".into());
            };
            let exp = l.expiry.get_untracked();
            if l.has_expiry {
                if exp.is_empty() {
                    return error.set(format!("Vui lòng nhập hạn sử dụng cho sản phẩm {}.", l.name));
                }
                if exp.as_str() <= today_str().as_str() {
                    return error.set(msg::MSG23.into());
                }
            }
            out.push(PurchaseLineIn {
                product_id: l.product_id,
                quantity: q,
                unit_cost: cost,
                expiry: if l.has_expiry { Some(exp) } else { None },
            });
        }
        let req = SavePurchaseReq { id, supplier_id: supplier_id.get_untracked(), lines: out };
        fetch_or("save_purchase", req, move |r: Result<Saved, String>| match r {
            Ok(s) => {
                c.ok(msg::msg25(&s.code));
                c.go(Page::Purchases);
            }
            Err(e) => error.set(e),
        });
    };

    let confirm = move |_| {
        let Some(id) = id else { return };
        fetch_or("confirm_purchase", IdReq { id }, move |r: Result<Saved, String>| match r {
            Ok(s) => {
                c.ok(msg::msg26(&s.code));
                c.go(Page::Purchases);
            }
            Err(e) => c.err(e),
        });
    };
    let reject = move |_| {
        let Some(id) = id else { return };
        fetch_or("reject_purchase", ReasonReq { id, reason: reason.get() }, move |r: Result<Saved, String>| match r {
            Ok(s) => {
                c.ok(msg::msg27(&s.code));
                c.go(Page::Purchases);
            }
            Err(e) => c.err(e),
        });
    };
    let cancel = move |_| {
        let Some(id) = id else { return };
        cancel_confirm.set(false);
        fetch(c, "cancel_purchase", IdReq { id }, move |_: Saved| {
            c.ok("Đã hủy phiếu nhập.");
            c.go(Page::Purchases);
        });
    };

    view! {
        <PageHead title=match id { None => "Tạo phiếu nhập hàng", Some(_) => "Phiếu nhập hàng" }>
            <button class="btn" on:click=move |_| c.go(Page::Purchases)>"← Danh sách phiếu"</button>
        </PageHead>

        // Thông tin phiếu (khi xem)
        {move || detail.get().map(|d| view! {
            <div class="card info">
                <div class="kv"><span>"Mã phiếu"</span><b>{d.row.code.clone()}</b></div>
                <div class="kv"><span>"Trạng thái"</span><StatusBadge text=d.row.status.clone()/></div>
                <div class="kv"><span>"Nhà cung cấp"</span><span>{d.row.supplier_name.clone()}</span></div>
                <div class="kv"><span>"Người lập"</span><span>{d.row.created_by_name.clone()}" · "{fmt_dt(&d.row.created_at)}</span></div>
                {d.row.confirmed_by_name.clone().map(|n| view! {
                    <div class="kv"><span>"Người xác nhận"</span><span>{n}" · "{d.row.confirmed_at.as_deref().map(fmt_dt).unwrap_or_default()}</span></div>
                })}
                {d.row.note.clone().map(|n| view! { <div class="kv"><span>"Lý do từ chối"</span><span class="low">{n}</span></div> })}
            </div>
        })}

        // Chế độ nhập liệu
        {move || editing.get().then(|| view! {
            <div class="card">
                <label class="form">"Nhà cung cấp *"
                    <select on:change=move |ev| supplier_id.set(event_target_value(&ev).parse().ok())>
                        <option value="" selected=move || supplier_id.get().is_none()>"— Chọn nhà cung cấp —"</option>
                        {move || suppliers.get().into_iter().map(|s| { let sid = s.id; view! {
                            <option value=s.id.to_string() selected=move || supplier_id.get() == Some(sid)>{s.name}</option>
                        }}).collect_view()}
                    </select>
                </label>
                <div class="adder">
                    <input class="search" type="search" placeholder="Tìm sản phẩm cần nhập theo tên hoặc mã…" prop:value=move || psearch.get()
                        on:input=move |ev| psearch.set(event_target_value(&ev))/>
                    <div class="dropdown">
                        {move || matches().into_iter().map(|p| { let pp = p.clone(); view! {
                            <button class="pick-item" on:click=move |_| add_product(pp.clone())>
                                <div class="pick-name">{p.name.clone()}</div>
                                <div class="muted small">{p.code.clone()}" · "{p.unit.clone()}{if p.has_expiry { " · có HSD" } else { "" }}</div>
                            </button>
                        }}).collect_view()}
                    </div>
                </div>
                <table class="tbl">
                    <thead><tr><th>"Sản phẩm"</th><th>"Số lượng thực nhận"</th><th>"Giá nhập (VNĐ)"</th><th>"Hạn sử dụng"</th><th class="num">"Thành tiền"</th><th></th></tr></thead>
                    <tbody>
                        <For each=move || lines.get() key=|l| l.key children=move |l| {
                            let (q1, q2) = (l.qty.clone(), l.qty.clone());
                            let (c1, c2) = (l.cost.clone(), l.cost.clone());
                            let e1 = l.expiry.clone();
                            let (qa, ca) = (l.qty.clone(), l.cost.clone());
                            let key = l.key;
                            view! {
                                <tr>
                                    <td>{l.name.clone()}<div class="muted small">{l.code.clone()}" · "{l.unit.clone()}</div></td>
                                    <td><input class="cell" type="text" inputmode="numeric" prop:value=move || q1.get() on:input=move |ev| q2.set(event_target_value(&ev))/></td>
                                    <td><input class="cell" type="text" inputmode="numeric" prop:value=move || c1.get() on:input=move |ev| c2.set(event_target_value(&ev))/></td>
                                    <td>{if l.has_expiry {
                                        let e2 = e1.clone();
                                        view! { <input class="cell" type="date" prop:value=move || e1.get() on:input=move |ev| e2.set(event_target_value(&ev))/> }.into_any()
                                    } else { view! { <span class="muted">"Không có HSD"</span> }.into_any() }}</td>
                                    <td class="num">{move || fmt_money(parse_num(&qa.get()).unwrap_or(0) * parse_num(&ca.get()).unwrap_or(0))}</td>
                                    <td><button class="icon-btn" on:click=move |_| lines.update(|v| v.retain(|x| x.key != key))><Icon name="trash" size=16/></button></td>
                                </tr>
                            }
                        }/>
                    </tbody>
                </table>
                <Show when=move || lines.get().is_empty()><p class="empty">{msg::MSG24}</p></Show>
                <div class="cart-total"><span>"Tổng tiền nhập"</span><b>{move || fmt_money(total())}" VNĐ"</b></div>
                <Show when=move || !error.get().is_empty()><div class="inline-err">{move || error.get()}</div></Show>
                <div class="actions">
                    <button class="btn" on:click=move |_| c.go(Page::Purchases)>"Hủy bỏ"</button>
                    <button class="btn primary" on:click=save>"Lưu phiếu"</button>
                </div>
            </div>
        })}

        // Chế độ xem
        {move || (!editing.get()).then(|| detail.get().map(|d| {
            let status = d.row.status.clone();
            let pending = status == status::PN_PENDING;
            let rejected = status == status::PN_REJECTED;
            view! {
                <div class="card table-card">
                    <table class="tbl">
                        <thead><tr><th>"Sản phẩm"</th><th class="num">"Số lượng"</th><th class="num">"Giá nhập"</th><th>"Hạn sử dụng"</th><th class="num">"Thành tiền"</th></tr></thead>
                        <tbody>{d.lines.iter().map(|l| view! {
                            <tr><td>{l.name.clone()}<div class="muted small">{l.code.clone()}</div></td>
                            <td class="num">{l.quantity}" "{l.unit.clone()}</td><td class="num">{fmt_money(l.unit_cost)}</td>
                            <td>{l.expiry.as_deref().map(fmt_date).unwrap_or_else(|| "—".into())}</td>
                            <td class="num">{fmt_money(l.quantity * l.unit_cost)}</td></tr>
                        }).collect_view()}</tbody>
                    </table>
                    <div class="cart-total"><span>"Tổng tiền nhập"</span><b>{fmt_money(d.row.total)}" VNĐ"</b></div>
                </div>
                <div class="actions">
                    {(pending || rejected).then(|| view! { <button class="btn danger" on:click=move |_| cancel_confirm.set(true)>"Hủy phiếu"</button> })}
                    {rejected.then(|| view! { <button class="btn" on:click=move |_| editing.set(true)>"Chỉnh sửa phiếu"</button> })}
                    {(pending && manager).then(|| view! {
                        <button class="btn danger" on:click=move |_| reject_modal.set(true)>"Từ chối"</button>
                        <button class="btn primary" on:click=confirm>"Xác nhận phiếu & cập nhật kho"</button>
                    })}
                </div>
                {(pending && !manager).then(|| view! { <p class="muted">"Phiếu đang chờ Quản lý xác nhận. Tồn kho chỉ được cộng sau khi phiếu được xác nhận."</p> })}
            }
        }))}

        {move || reject_modal.get().then(|| view! {
            <Modal title="Từ chối phiếu nhập" on_close=move |_| reject_modal.set(false)>
                <label class="form">"Lý do từ chối *"
                    <textarea rows="3" prop:value=move || reason.get() on:input=move |ev| reason.set(event_target_value(&ev))></textarea>
                </label>
                <div class="actions">
                    <button class="btn" on:click=move |_| reject_modal.set(false)>"Hủy bỏ"</button>
                    <button class="btn danger" on:click=reject>"Từ chối phiếu"</button>
                </div>
            </Modal>
        })}
        {move || cancel_confirm.get().then(|| view! {
            <Confirm message="Bạn có chắc chắn muốn hủy phiếu nhập này?" on_cancel=move |_| cancel_confirm.set(false) on_ok=cancel/>
        })}
    }
}
