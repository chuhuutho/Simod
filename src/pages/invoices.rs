//! S08/S09 – Lịch sử hóa đơn & chi tiết hóa đơn (UC11)
use crate::api::fetch;
use crate::ctx::*;
use crate::icons::Icon;
use crate::ui::*;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn InvoicesPage() -> impl IntoView {
    let c = ctx();
    let items = RwSignal::new(Vec::<InvoiceRow>::new());
    let search = RwSignal::new(String::new());
    let from = RwSignal::new(today_str());
    let to = RwSignal::new(today_str());

    Effect::new(move |_| {
        let req = InvoiceQuery { search: None, from: Some(from.get()), to: Some(to.get()) };
        fetch(c, "list_invoices", req, move |v: Vec<InvoiceRow>| items.set(v));
    });
    let rows = move || {
        let q = search.get();
        items.get().into_iter().filter(|r| matches_query(&q, &[&r.code, &r.created_by_name])).collect::<Vec<_>>()
    };
    let sum = move || rows().iter().map(|r| r.total).sum::<i64>();

    view! {
        <PageHead title="Lịch sử hóa đơn"><span/></PageHead>
        <div class="toolbar">
            <input class="search" type="search" placeholder="Tìm theo mã hóa đơn…" prop:value=move || search.get()
                on:input=move |ev| search.set(event_target_value(&ev))/>
            <label class="inline">"Từ ngày"<input type="date" prop:value=move || from.get() on:input=move |ev| from.set(event_target_value(&ev))/></label>
            <label class="inline">"Đến ngày"<input type="date" prop:value=move || to.get() on:input=move |ev| to.set(event_target_value(&ev))/></label>
        </div>
        <div class="card table-card">
            <table class="tbl clickable">
                <thead><tr><th>"Mã hóa đơn"</th><th>"Thời gian"</th><th>"Người lập"</th><th>"Hình thức"</th><th class="num">"Tổng tiền (VNĐ)"</th><th>"Trạng thái"</th></tr></thead>
                <tbody>
                    {move || rows().into_iter().map(|r| {
                        let id = r.id;
                        view! {
                            <tr on:click=move |_| c.go(Page::Invoice(id))>
                                <td class="mono">{r.code.clone()}</td>
                                <td>{fmt_dt(&r.created_at)}</td>
                                <td>{r.created_by_name.clone()}</td>
                                <td>{method_label(&r.method)}</td>
                                <td class="num">{fmt_money(r.total)}</td>
                                <td><StatusBadge text=r.status.clone()/></td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
            <NoResult show=Signal::derive(move || rows().is_empty())/>
        </div>
        <div class="summary">{move || format!("{} hóa đơn · Tổng: {} VNĐ", rows().len(), fmt_money(sum()))}</div>
    }
}

#[component]
pub fn InvoiceDetailPage(id: i64) -> impl IntoView {
    let c = ctx();
    let detail = RwSignal::new(None::<InvoiceDetail>);
    fetch(c, "get_invoice", IdReq { id }, move |d: InvoiceDetail| detail.set(Some(d)));
    view! {
        <PageHead title="Chi tiết hóa đơn">
            <button class="btn" on:click=move |_| c.go(Page::Invoices)><Icon name="arrow-left" size=16/>"Quay lại"</button>
            <button class="btn primary" on:click=move |_| print_page()><Icon name="printer" size=16/>"In hóa đơn"</button>
        </PageHead>
        {move || detail.get().map(|d| view! { <div class="card narrow"><InvoiceView detail=d/></div> })}
    }
}
