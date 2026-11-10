//! S20/S21 – Tài khoản nhân viên (UC04) và nhật ký hoạt động (BR-03)
use crate::api::{fetch, fetch_or};
use crate::ctx::*;
use crate::icons::Icon;
use crate::ui::*;
use leptos::prelude::*;
use simod_shared::*;

#[component]
pub fn UsersPage() -> impl IntoView {
    let c = ctx();
    let me = c.user.get_untracked().map(|u| u.id).unwrap_or(0);
    let items = RwSignal::new(Vec::<UserRow>::new());
    let search = RwSignal::new(String::new());
    let tick = RwSignal::new(0u32);
    let editing = RwSignal::new(None::<Option<UserRow>>);

    Effect::new(move |_| {
        tick.get();
        fetch(c, "list_users", SearchReq { search: None }, move |v: Vec<UserRow>| items.set(v));
    });
    let rows = move || {
        let q = search.get();
        items.get().into_iter().filter(|u| matches_query(&q, &[&u.username, &u.full_name])).collect::<Vec<_>>()
    };
    let toggle = move |u: UserRow| {
        let active = !u.active || u.locked_until.is_some();
        fetch(c, "set_user_active", SetActiveReq { id: u.id, active }, move |_: ()| {
            c.ok(if active { "Đã mở khóa tài khoản." } else { "Đã khóa tài khoản." });
            tick.update(|t| *t += 1);
        });
    };

    view! {
        <PageHead title="Tài khoản nhân viên">
            <button class="btn primary" on:click=move |_| editing.set(Some(None))>"+ Tạo tài khoản"</button>
        </PageHead>
        <div class="toolbar">
            <input class="search" type="search" placeholder="Tìm theo tên đăng nhập hoặc họ tên…" prop:value=move || search.get()
                on:input=move |ev| search.set(event_target_value(&ev))/>
        </div>
        <div class="card table-card">
            <table class="tbl">
                <thead><tr><th>"Tên đăng nhập"</th><th>"Họ và tên"</th><th>"Vai trò"</th><th>"Trạng thái"</th><th></th></tr></thead>
                <tbody>
                    {move || rows().into_iter().map(|u| {
                        let (u_edit, u_tog) = (u.clone(), u.clone());
                        let locked = u.locked_until.is_some();
                        let blocked = !u.active || locked;
                        let is_me = u.id == me;
                        view! {
                            <tr class:dim=!u.active>
                                <td class="mono">{u.username.clone()}</td><td>{u.full_name.clone()}</td><td>{u.role.label()}</td>
                                <td>{if !u.active { view! { <span class="badge gray">"Đã khóa"</span> } }
                                     else if locked { view! { <span class="badge amber">"Tạm khóa (đăng nhập sai)"</span> } }
                                     else { view! { <span class="badge green">"Hoạt động"</span> } }}</td>
                                <td class="row-actions">
                                    <button class="btn small" on:click=move |_| editing.set(Some(Some(u_edit.clone())))>"Sửa"</button>
                                    {(!is_me).then(|| view! {
                                        <button class="btn small" on:click=move |_| toggle(u_tog.clone())>{if blocked { "Mở khóa" } else { "Khóa" }}</button>
                                    })}
                                </td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
            <NoResult show=Signal::derive(move || rows().is_empty())/>
        </div>
        {move || editing.get().map(|u| view! {
            <UserForm user=u on_close=move |_| editing.set(None) on_saved=move |_| { editing.set(None); tick.update(|t| *t += 1); }/>
        })}
    }
}

#[component]
fn UserForm(
    user: Option<UserRow>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_saved: Callback<()>,
) -> impl IntoView {
    let c = ctx();
    let id = user.as_ref().map(|u| u.id);
    let username = RwSignal::new(user.as_ref().map(|u| u.username.clone()).unwrap_or_default());
    let full_name = RwSignal::new(user.as_ref().map(|u| u.full_name.clone()).unwrap_or_default());
    let role = RwSignal::new(user.as_ref().map(|u| u.role).unwrap_or(Role::Staff));
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());

    let save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let req = SaveUserReq {
            id,
            username: username.get(),
            full_name: full_name.get(),
            role: role.get(),
            password: Some(password.get()).filter(|p| !p.is_empty()),
        };
        fetch_or("save_user", req, move |r: Result<i64, String>| match r {
            Ok(_) => {
                c.ok(if id.is_some() { "Cập nhật tài khoản thành công." } else { "Tạo tài khoản thành công." });
                on_saved.run(());
            }
            Err(e) => error.set(e),
        });
    };

    view! {
        <Modal title=if id.is_some() { "Cập nhật tài khoản" } else { "Tạo tài khoản" } on_close=on_close>
            <form class="form" on:submit=save>
                <label>"Tên đăng nhập *"<input type="text" prop:value=move || username.get() on:input=move |ev| username.set(event_target_value(&ev))/></label>
                <label>"Họ và tên *"<input type="text" prop:value=move || full_name.get() on:input=move |ev| full_name.set(event_target_value(&ev))/></label>
                <label>"Vai trò"
                    <select on:change=move |ev| role.set(if event_target_value(&ev) == "manager" { Role::Manager } else { Role::Staff })>
                        <option value="staff" selected=move || role.get() == Role::Staff>"Nhân viên"</option>
                        <option value="manager" selected=move || role.get() == Role::Manager>"Quản lý"</option>
                    </select>
                </label>
                <label>{if id.is_some() { "Đặt lại mật khẩu (để trống nếu không đổi)" } else { "Mật khẩu *" }}
                    <input type="password" autocomplete="new-password" prop:value=move || password.get() on:input=move |ev| password.set(event_target_value(&ev))/>
                </label>
                <Show when=move || !error.get().is_empty()><div class="inline-err">{move || error.get()}</div></Show>
                <div class="actions">
                    <button type="button" class="btn" on:click=move |_| on_close.run(())>"Hủy bỏ"</button>
                    <button type="submit" class="btn primary">"Lưu"</button>
                </div>
            </form>
        </Modal>
    }
}

#[component]
pub fn LogsPage() -> impl IntoView {
    let c = ctx();
    let items = RwSignal::new(Vec::<LogRow>::new());
    let search = RwSignal::new(String::new());
    let tick = RwSignal::new(0u32);
    Effect::new(move |_| {
        tick.get();
        fetch(c, "list_logs", SearchReq { search: None }, move |v: Vec<LogRow>| items.set(v));
    });
    let backup = move |_| {
        fetch(c, "backup_now", Empty {}, move |p: String| {
            c.ok(format!("Đã sao lưu dữ liệu: {p}"));
            tick.update(|t| *t += 1);
        });
    };
    let rows = move || {
        let q = search.get();
        items.get().into_iter().filter(|l| matches_query(&q, &[&l.username, &l.action, &l.detail])).collect::<Vec<_>>()
    };
    view! {
        <PageHead title="Nhật ký hoạt động">
            <button class="btn" on:click=backup><Icon name="shield" size=16/>"Sao lưu dữ liệu ngay"</button>
        </PageHead>
        <div class="toolbar">
            <input class="search" type="search" placeholder="Tìm theo người dùng, thao tác, nội dung…" prop:value=move || search.get()
                on:input=move |ev| search.set(event_target_value(&ev))/>
        </div>
        <div class="card table-card">
            <table class="tbl">
                <thead><tr><th>"Thời gian"</th><th>"Người thực hiện"</th><th>"Thao tác"</th><th>"Nội dung"</th></tr></thead>
                <tbody>{move || rows().into_iter().map(|l| view! {
                    <tr><td>{fmt_dt(&l.at)}</td><td class="mono">{l.username.clone()}</td><td>{l.action.clone()}</td><td>{l.detail.clone()}</td></tr>
                }).collect_view()}</tbody>
            </table>
            <NoResult show=Signal::derive(move || rows().is_empty())/>
        </div>
    }
}
