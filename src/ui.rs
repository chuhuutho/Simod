//! Thành phần giao diện dùng chung.
use crate::icons::Icon;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn Modal(
    #[prop(into)] title: String,
    #[prop(into)] on_close: Callback<()>,
    #[prop(optional)] wide: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="overlay">
            <div class="modal" class:wide=wide>
                <div class="modal-head">
                    <h3>{title}</h3>
                    <button class="icon-btn" on:click=move |_| on_close.run(())><Icon name="x" size=16/></button>
                </div>
                <div class="modal-body">{children()}</div>
            </div>
        </div>
    }
}

/// Hộp thoại xác nhận (MSG07 và tương tự).
#[component]
pub fn Confirm(
    #[prop(into)] message: String,
    #[prop(into)] on_ok: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
    #[prop(optional, into)] ok_label: Option<String>,
) -> impl IntoView {
    let label = ok_label.unwrap_or_else(|| "Đồng ý".to_string());
    view! {
        <Modal title="Xác nhận" on_close=on_cancel>
            <p class="confirm-msg">{message}</p>
            <div class="actions">
                <button class="btn" on:click=move |_| on_cancel.run(())>"Hủy bỏ"</button>
                <button class="btn primary" on:click=move |_| on_ok.run(())>{label}</button>
            </div>
        </Modal>
    }
}

#[component]
pub fn PageHead(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <div class="page-head">
            <h2>{title}</h2>
            <div class="page-actions">{children()}</div>
        </div>
    }
}

/// Dòng thông báo "không có kết quả" (MSG01)
#[component]
pub fn NoResult(#[prop(into)] show: Signal<bool>) -> impl IntoView {
    view! { <Show when=move || show.get()><p class="empty">{msg::MSG01}</p></Show> }
}

#[component]
pub fn StatusBadge(#[prop(into)] text: String) -> impl IntoView {
    let class = match text.as_str() {
        status::PN_CONFIRMED | status::KK_ADJUSTED | status::KK_DONE | status::HD_PAID => "badge green",
        status::PN_PENDING | status::KK_PENDING => "badge amber",
        status::PN_REJECTED | status::KK_RECOUNT => "badge red",
        status::PN_CANCELLED => "badge gray",
        _ => "badge blue",
    };
    view! { <span class=class>{text}</span> }
}

/// Hóa đơn (S09) — dùng cho cả màn hình chi tiết lẫn sau khi thanh toán; có thể in.
#[component]
pub fn InvoiceView(detail: InvoiceDetail) -> impl IntoView {
    let d = detail;
    view! {
        <div class="receipt printable">
            <h3 class="center">"CỬA HÀNG TẠP HÓA"</h3>
            <p class="center muted">"HÓA ĐƠN BÁN HÀNG"</p>
            <div class="kv"><span>"Số hóa đơn"</span><b>{d.row.code.clone()}</b></div>
            <div class="kv"><span>"Ngày lập"</span><span>{fmt_dt(&d.row.created_at)}</span></div>
            <div class="kv"><span>"Thu ngân"</span><span>{d.row.created_by_name.clone()}</span></div>
            <table class="tbl compact">
                <thead>
                    <tr><th>"Sản phẩm"</th><th class="num">"SL"</th><th class="num">"Đơn giá"</th><th class="num">"Thành tiền"</th></tr>
                </thead>
                <tbody>
                    {d.lines.iter().map(|l| view! {
                        <tr>
                            <td>{l.name.clone()}<div class="muted small">{l.code.clone()}</div></td>
                            <td class="num">{l.quantity}" "{l.unit.clone()}</td>
                            <td class="num">{fmt_money(l.unit_price)}</td>
                            <td class="num">{fmt_money(l.amount)}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
            <div class="kv total"><span>"Tổng tiền hàng"</span><b>{fmt_money(d.row.total)}" VNĐ"</b></div>
            <div class="kv"><span>"Hình thức"</span><span>{method_label(&d.row.method)}</span></div>
            <div class="kv"><span>"Tiền khách trả"</span><span>{fmt_money(d.paid)}" VNĐ"</span></div>
            <div class="kv"><span>"Tiền thừa"</span><span>{fmt_money(d.change)}" VNĐ"</span></div>
            <p class="center muted">"Cảm ơn quý khách!"</p>
        </div>
    }
}
