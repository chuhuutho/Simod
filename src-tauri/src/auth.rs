use crate::util::{dberr, log, now, now_plus_minutes, Res};
use crate::AppState;
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rusqlite::{params, OptionalExtension};
use simod_shared::*;
use tauri::State;

pub fn hash_password(p: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(p.as_bytes(), &salt)
        .expect("hash mật khẩu")
        .to_string()
}

fn verify_password(p: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| Argon2::default().verify_password(p.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

#[tauri::command]
pub fn login(state: State<AppState>, req: LoginReq) -> Res<UserInfo> {
    let username = req.username.trim().to_string();
    if username.is_empty() || req.password.is_empty() {
        return Err(msg::MSG10.into());
    }
    let conn = state.db();
    let row = conn
        .query_row(
            "SELECT id,username,full_name,password_hash,role,active,failed_count,locked_until
             FROM nguoi_dung WHERE username=?1",
            [&username],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)? != 0,
                    r.get::<_, i64>(6)?,
                    r.get::<_, Option<String>>(7)?,
                ))
            },
        )
        .optional()
        .map_err(dberr)?;
    let Some((id, uname, full_name, hash, role, active, failed, locked)) = row else {
        return Err(msg::MSG09.into());
    };
    if !active {
        return Err(msg::MSG11.into());
    }
    if let Some(until) = &locked {
        if until.as_str() > now().as_str() {
            return Err(msg::MSG12.into());
        }
    }
    if verify_password(&req.password, &hash) {
        conn.execute(
            "UPDATE nguoi_dung SET failed_count=0, locked_until=NULL WHERE id=?1",
            [id],
        )
        .map_err(dberr)?;
        let user = UserInfo { id, username: uname, full_name, role: Role::from_db(&role) };
        log(&conn, &user, "Đăng nhập", "Đăng nhập thành công");
        *state.session() = Some(user.clone());
        Ok(user)
    } else {
        let failed = failed + 1;
        if failed >= 5 {
            conn.execute(
                "UPDATE nguoi_dung SET failed_count=0, locked_until=?1 WHERE id=?2",
                params![now_plus_minutes(30), id],
            )
            .map_err(dberr)?;
            Err(msg::MSG12.into())
        } else {
            conn.execute("UPDATE nguoi_dung SET failed_count=?1 WHERE id=?2", params![failed, id])
                .map_err(dberr)?;
            Err(msg::MSG09.into())
        }
    }
}

#[tauri::command]
pub fn logout(state: State<AppState>, _req: Empty) -> Res<()> {
    if let Some(u) = state.session().take() {
        log(&state.db(), &u, "Đăng xuất", "");
    }
    Ok(())
}

#[tauri::command]
pub fn current_user(state: State<AppState>, _req: Empty) -> Res<Option<UserInfo>> {
    Ok(state.session().clone())
}

#[tauri::command]
pub fn change_password(state: State<AppState>, req: ChangePasswordReq) -> Res<()> {
    let user = state.user()?;
    if req.new.chars().count() < 6 {
        return Err("Mật khẩu mới phải có ít nhất 6 ký tự.".into());
    }
    let conn = state.db();
    let hash: String = conn
        .query_row("SELECT password_hash FROM nguoi_dung WHERE id=?1", [user.id], |r| r.get(0))
        .map_err(dberr)?;
    if !verify_password(&req.current, &hash) {
        return Err(msg::MSG13.into());
    }
    conn.execute(
        "UPDATE nguoi_dung SET password_hash=?1 WHERE id=?2",
        params![hash_password(&req.new), user.id],
    )
    .map_err(dberr)?;
    log(&conn, &user, "Đổi mật khẩu", "");
    Ok(())
}
