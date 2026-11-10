//! S10/S11 – Nhà cung cấp (UC12)
use crate::api::{fetch, fetch_or};
use crate::ctx::*;
use crate::ui::*;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn SuppliersPage() -> impl IntoView {
    let c = ctx();
    let items = RwSignal::new(Vec::<Supplier>::new());
    let search = RwSignal::new(String::new());
    let tick = RwSignal::new(0u32);
    let editing = RwSignal::new(None::<Option<Supplier>>);

    Effect::new(move |_| {
        tick.get();
        fetch(c, "list_suppliers", SupplierQuery { search: None, only_active: false }, move |v: Vec<Supplier>| items.set(v));
    });
    let rows = move || {
        let q = search.get();
        items.get().into_iter().filter(|s| matches_query(&q, &[&s.name, &s.phone, &s.address])).collect::<Vec<_>>()
    };
    let toggle = move |s: Supplier| {
        let active = !s.active;
        fetch(c, "set_supplier_active", SetActiveReq { id: s.id, active }, move |_: ()| {
            c.ok(if active { "Đã kích hoạt nhà cung cấp." } else { "Đã ngừng hoạt động nhà cung cấp." });
            tick.update(|t| *t += 1);
        });
    };

    view! {
        <PageHead title="Nhà cung cấp">
            <button class="btn primary" on:click=move |_| editing.set(Some(None))>"+ Thêm nhà cung cấp"</button>
        </PageHead>
        <div class="toolbar">
            <input class="search" type="search" placeholder="Tìm theo tên, số điện thoại, địa chỉ…" prop:value=move || search.get()
                on:input=move |ev| search.set(event_target_value(&ev))/>
        </div>
        <div class="card table-card">
            <table class="tbl">
                <thead><tr><th>"Tên nhà cung cấp"</th><th>"Số điện thoại"</th><th>"Địa chỉ"</th><th>"Trạng thái"</th><th></th></tr></thead>
                <tbody>
                    {move || rows().into_iter().map(|s| {
                        let (s_edit, s_tog) = (s.clone(), s.clone());
                        view! {
                            <tr class:dim=!s.active>
                                <td>{s.name.clone()}</td><td>{s.phone.clone()}</td><td>{s.address.clone()}</td>
                                <td>{if s.active { view! { <span class="badge green">"Hoạt động"</span> } } else { view! { <span class="badge gray">"Ngừng hoạt động"</span> } }}</td>
                                <td class="row-actions">
                                    <button class="btn small" on:click=move |_| editing.set(Some(Some(s_edit.clone())))>"Sửa"</button>
                                    <button class="btn small" on:click=move |_| toggle(s_tog.clone())>{if s.active { "Ngừng" } else { "Kích hoạt" }}</button>
                                </td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
            <NoResult show=Signal::derive(move || rows().is_empty())/>
        </div>
        {move || editing.get().map(|s| view! {
            <SupplierForm supplier=s on_close=move |_| editing.set(None)
                on_saved=move |_| { editing.set(None); tick.update(|t| *t += 1); }/>
        })}
    }
}

#[component]
fn SupplierForm(
    supplier: Option<Supplier>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_saved: Callback<()>,
) -> impl IntoView {
    let c = ctx();
    let id = supplier.as_ref().map(|s| s.id);
    let name = RwSignal::new(supplier.as_ref().map(|s| s.name.clone()).unwrap_or_default());
    let phone = RwSignal::new(supplier.as_ref().map(|s| s.phone.clone()).unwrap_or_default());
    let address = RwSignal::new(supplier.as_ref().map(|s| s.address.clone()).unwrap_or_default());
    let error = RwSignal::new(String::new());
    let save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let req = SaveSupplierReq { id, name: name.get(), phone: phone.get(), address: address.get() };
        fetch_or("save_supplier", req, move |r: Result<i64, String>| match r {
            Ok(_) => {
                c.ok(if id.is_some() { "Cập nhật nhà cung cấp thành công." } else { "Thêm nhà cung cấp thành công." });
                on_saved.run(());
            }
            Err(e) => error.set(e),
        });
    };
    view! {
        <Modal title=if id.is_some() { "Cập nhật nhà cung cấp" } else { "Thêm nhà cung cấp" } on_close=on_close>
            <form class="form" on:submit=save>
                <label>"Tên nhà cung cấp *"<input type="text" prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))/></label>
                <label>"Số điện thoại"<input type="text" prop:value=move || phone.get() on:input=move |ev| phone.set(event_target_value(&ev))/></label>
                <label>"Địa chỉ"<input type="text" prop:value=move || address.get() on:input=move |ev| address.set(event_target_value(&ev))/></label>
                <Show when=move || !error.get().is_empty()><div class="inline-err">{move || error.get()}</div></Show>
                <div class="actions">
                    <button type="button" class="btn" on:click=move |_| on_close.run(())>"Hủy bỏ"</button>
                    <button type="submit" class="btn primary">"Lưu"</button>
                </div>
            </form>
        </Modal>
    }
}
