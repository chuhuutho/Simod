//! S16/S17/S18 – Phiếu kiểm kê, kiểm kê kho, kết quả & điều chỉnh (UC18, UC19)
use crate::api::{fetch, fetch_or};
use crate::ctx::*;
use crate::ui::*;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn StocktakesPage() -> impl IntoView {
    let c = ctx();
    let items = RwSignal::new(Vec::<StocktakeRow>::new());
    fetch(c, "list_stocktakes", Empty {}, move |v: Vec<StocktakeRow>| items.set(v));

    let create = move |_| {
        fetch(c, "create_stocktake", Empty {}, move |s: Saved| c.go(Page::Stocktake(s.id)));
    };

    view! {
        <PageHead title="Phiếu kiểm kê kho">
            <button class="btn primary" on:click=create>"+ Tạo phiếu kiểm kê"</button>
        </PageHead>
        <div class="card table-card">
            <table class="tbl clickable">
                <thead><tr><th>"Mã phiếu"</th><th>"Người lập"</th><th>"Ngày lập"</th><th class="num">"SP chênh lệch"</th><th>"Người duyệt"</th><th>"Trạng thái"</th></tr></thead>
                <tbody>
                    {move || items.get().into_iter().map(|r| {
                        let id = r.id;
                        view! {
                            <tr on:click=move |_| c.go(Page::Stocktake(id))>
                                <td class="mono">{r.code.clone()}</td><td>{r.created_by_name.clone()}</td><td>{fmt_dt(&r.created_at)}</td>
                                <td class="num">{r.diff_count}</td>
                                <td>{r.approved_by_name.clone().unwrap_or_else(|| "—".into())}</td>
                                <td><StatusBadge text=r.status.clone()/></td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
            <NoResult show=Signal::derive(move || items.get().is_empty())/>
        </div>
    }
}

#[derive(Clone)]
struct KLine {
    product_id: i64,
    code: String,
    name: String,
    unit: String,
    system_qty: i64,
    actual: ArcRwSignal<String>,
    reason: ArcRwSignal<String>,
}

#[component]
pub fn StocktakePage(id: i64) -> impl IntoView {
    let c = ctx();
    let manager = c.is_manager();
    let detail = RwSignal::new(None::<StocktakeDetail>);
    let lines = RwSignal::new(Vec::<KLine>::new());
    let search = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let recount_modal = RwSignal::new(false);
    let reason = RwSignal::new(String::new());

    fetch(c, "get_stocktake", IdReq { id }, move |d: StocktakeDetail| {
        lines.set(
            d.lines
                .iter()
                .map(|l| KLine {
                    product_id: l.product_id,
                    code: l.code.clone(),
                    name: l.name.clone(),
                    unit: l.unit.clone(),
                    system_qty: l.system_qty,
                    actual: ArcRwSignal::new(l.actual_qty.map(|a| a.to_string()).unwrap_or_default()),
                    reason: ArcRwSignal::new(l.reason.clone()),
                })
                .collect(),
        );
        detail.set(Some(d));
    });

    let build = move |complete: bool| -> Result<SaveStocktakeReq, String> {
        let mut out = Vec::new();
        for l in lines.get_untracked() {
            let raw = l.actual.get_untracked();
            let actual = if raw.trim().is_empty() {
                None
            } else {
                match parse_num(&raw) {
                    Some(a) if a >= 0 => Some(a),
                    _ => return Err(format!("{} — {}", l.name, msg::MSG28)),
                }
            };
            if complete {
                let Some(a) = actual else { return Err(format!("{} — {}", l.name, msg::MSG28)) };
                if a != l.system_qty && l.reason.get_untracked().trim().is_empty() {
                    return Err(format!("{} — {}", l.name, msg::MSG29));
                }
            }
            out.push(StocktakeLineIn { product_id: l.product_id, actual, reason: l.reason.get_untracked() });
        }
        Ok(SaveStocktakeReq { id, lines: out, complete })
    };

    let save = move |complete: bool| {
        error.set(String::new());
        match build(complete) {
            Err(e) => error.set(e),
            Ok(req) => fetch_or("save_stocktake", req, move |r: Result<StocktakeSaved, String>| match r {
                Ok(s) if complete => {
                    c.ok(msg::msg30(s.diff_count));
                    c.go(Page::Stocktakes);
                }
                Ok(_) => c.ok("Đã lưu tạm phiếu kiểm kê."),
                Err(e) => error.set(e),
            }),
        }
    };

    let approve = move |_| {
        fetch(c, "approve_stocktake", IdReq { id }, move |_: Saved| {
            c.ok(msg::MSG31);
            c.go(Page::Stocktakes);
        });
    };
    let recount = move |_| {
        fetch_or("request_recount", ReasonReq { id, reason: reason.get() }, move |r: Result<Saved, String>| match r {
            Ok(_) => {
                c.ok("Đã yêu cầu kiểm lại.");
                c.go(Page::Stocktakes);
            }
            Err(e) => c.err(e),
        });
    };

    view! {
        <PageHead title="Kiểm kê kho">
            <button class="btn" on:click=move |_| c.go(Page::Stocktakes)>"← Danh sách phiếu"</button>
        </PageHead>
        {move || detail.get().map(|d| {
            let editable = d.row.status == status::KK_COUNTING || d.row.status == status::KK_RECOUNT;
            let pending = d.row.status == status::KK_PENDING;
            view! {
                <div class="card info">
                    <div class="kv"><span>"Mã phiếu"</span><b>{d.row.code.clone()}</b></div>
                    <div class="kv"><span>"Trạng thái"</span><StatusBadge text=d.row.status.clone()/></div>
                    <div class="kv"><span>"Người lập"</span><span>{d.row.created_by_name.clone()}" · "{fmt_dt(&d.row.created_at)}</span></div>
                    {d.row.approved_by_name.clone().map(|n| view! {
                        <div class="kv"><span>"Người duyệt"</span><span>{n}" · "{d.row.approved_at.as_deref().map(fmt_dt).unwrap_or_default()}</span></div>
                    })}
                    {d.row.note.clone().map(|n| view! { <div class="kv"><span>"Yêu cầu kiểm lại"</span><span class="low">{n}</span></div> })}
                </div>
                {editable.then(|| view! { <p class="muted">"Đếm hàng thực tế trên kệ và trong kho rồi nhập số lượng (không nhập theo ước lượng). Sản phẩm có chênh lệch phải ghi lý do."</p> })}
                <div class="toolbar">
                    <input class="search" type="search" placeholder="Lọc sản phẩm theo tên hoặc mã…" prop:value=move || search.get()
                        on:input=move |ev| search.set(event_target_value(&ev))/>
                </div>
                <div class="card table-card">
                    <table class="tbl">
                        <thead><tr><th>"Sản phẩm"</th><th class="num">"Số lượng hệ thống"</th><th class="num">"Số lượng thực tế"</th><th class="num">"Chênh lệch"</th><th>"Lý do"</th></tr></thead>
                        <tbody>
                            <For each=move || lines.get() key=|l| l.product_id children=move |l| {
                                let (a1, a2, a3) = (l.actual.clone(), l.actual.clone(), l.actual.clone());
                                let (r1, r2, r3) = (l.reason.clone(), l.reason.clone(), l.reason.clone());
                                let sys = l.system_qty;
                                let (n, cd) = (l.name.clone(), l.code.clone());
                                let hide = move || !matches_query(&search.get(), &[&n, &cd]);
                                let diff = move || parse_num(&a3.get()).map(|a| a - sys);
                                view! {
                                    <tr class:hidden=hide>
                                        <td>{l.name.clone()}<div class="muted small">{l.code.clone()}" · "{l.unit.clone()}</div></td>
                                        <td class="num">{sys}</td>
                                        <td class="num">{if editable {
                                            view! { <input class="cell narrow-cell" type="text" inputmode="numeric" prop:value=move || a1.get()
                                                on:input=move |ev| a2.set(event_target_value(&ev))/> }.into_any()
                                        } else { view! { <span>{move || a1.get()}</span> }.into_any() }}</td>
                                        <td class="num">{move || match diff() {
                                            Some(0) => view! { <span class="muted">"0"</span> }.into_any(),
                                            Some(x) if x > 0 => view! { <b class="pos">{format!("+{x}")}</b> }.into_any(),
                                            Some(x) => view! { <b class="low">{x}</b> }.into_any(),
                                            None => view! { <span class="muted">"—"</span> }.into_any(),
                                        }}</td>
                                        <td>{if editable {
                                            view! { <input class="cell" type="text" placeholder="Hao hụt, hư hỏng, hết hạn, nhầm lẫn…" prop:value=move || r1.get()
                                                on:input=move |ev| r2.set(event_target_value(&ev))/> }.into_any()
                                        } else { view! { <span>{move || r3.get()}</span> }.into_any() }}</td>
                                    </tr>
                                }
                            }/>
                        </tbody>
                    </table>
                </div>
                <Show when=move || !error.get().is_empty()><div class="inline-err">{move || error.get()}</div></Show>
                {editable.then(|| view! {
                    <div class="actions">
                        <button class="btn" on:click=move |_| save(false)>"Lưu tạm"</button>
                        <button class="btn primary" on:click=move |_| save(true)>"Hoàn tất kiểm kê"</button>
                    </div>
                })}
                {(pending && manager).then(|| view! {
                    <div class="actions">
                        <button class="btn danger" on:click=move |_| recount_modal.set(true)>"Yêu cầu kiểm lại"</button>
                        <button class="btn primary" on:click=approve>"Duyệt điều chỉnh tồn kho"</button>
                    </div>
                })}
                {(pending && !manager).then(|| view! { <p class="muted">"Phiếu đang chờ Quản lý duyệt. Tồn kho chưa thay đổi."</p> })}
            }
        })}
        {move || recount_modal.get().then(|| view! {
            <Modal title="Yêu cầu kiểm lại" on_close=move |_| recount_modal.set(false)>
                <label class="form">"Lý do *"
                    <textarea rows="3" prop:value=move || reason.get() on:input=move |ev| reason.set(event_target_value(&ev))></textarea>
                </label>
                <div class="actions">
                    <button class="btn" on:click=move |_| recount_modal.set(false)>"Hủy bỏ"</button>
                    <button class="btn danger" on:click=recount>"Yêu cầu kiểm lại"</button>
                </div>
            </Modal>
        })}
    }
}
