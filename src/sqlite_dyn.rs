#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::Path;
use anyhow::{bail, Context, Result};
use crate::config::{AppConfig, LightConfig};

type Sqlite3 = c_void;
type Sqlite3Stmt = c_void;

type FnOpen = unsafe extern "C" fn(filename: *const c_char, ppDb: *mut *mut Sqlite3) -> c_int;
type FnClose = unsafe extern "C" fn(pDb: *mut Sqlite3) -> c_int;
type FnPrepareV2 = unsafe extern "C" fn(
    pDb: *mut Sqlite3,
    zSql: *const c_char,
    nByte: c_int,
    ppStmt: *mut *mut Sqlite3Stmt,
    pzTail: *mut *const c_char,
) -> c_int;
type FnStep = unsafe extern "C" fn(pStmt: *mut Sqlite3Stmt) -> c_int;
type FnColumnText = unsafe extern "C" fn(pStmt: *mut Sqlite3Stmt, iCol: c_int) -> *const c_char;
type FnColumnInt = unsafe extern "C" fn(pStmt: *mut Sqlite3Stmt, iCol: c_int) -> c_int;
type FnFinalize = unsafe extern "C" fn(pStmt: *mut Sqlite3Stmt) -> c_int;

#[cfg(windows)]
struct DynLib {
    handle: *mut c_void,
}

#[cfg(windows)]
impl DynLib {
    unsafe fn load() -> Result<Self> {
        let name = CString::new("winsqlite3.dll").unwrap();
        let handle = windows_sys::Win32::System::LibraryLoader::LoadLibraryA(name.as_ptr() as *const u8);
        if handle.is_null() {
            let fallback = CString::new("sqlite3.dll").unwrap();
            let handle_fb = windows_sys::Win32::System::LibraryLoader::LoadLibraryA(fallback.as_ptr() as *const u8);
            if handle_fb.is_null() {
                bail!("Failed to load winsqlite3.dll or sqlite3.dll");
            }
            return Ok(Self { handle: handle_fb as *mut c_void });
        }
        Ok(Self { handle: handle as *mut c_void })
    }

    unsafe fn get_sym<T>(&self, sym: &str) -> Result<T> {
        let c_sym = CString::new(sym).unwrap();
        let proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(
            self.handle as _,
            c_sym.as_ptr() as *const u8,
        );
        if let Some(p) = proc {
            Ok(std::mem::transmute_copy(&p))
        } else {
            bail!("Symbol '{}' not found in SQLite library", sym)
        }
    }
}

#[cfg(windows)]
impl Drop for DynLib {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                windows_sys::Win32::System::LibraryLoader::FreeLibrary(self.handle as _);
            }
        }
    }
}

#[cfg(not(windows))]
struct DynLib {
    handle: *mut c_void,
}

#[cfg(not(windows))]
impl DynLib {
    unsafe fn load() -> Result<Self> {
        let candidates = [
            "libsqlite3.so.0\0",
            "libsqlite3.so\0",
            "libsqlite3.dylib\0",
            "/usr/lib/libsqlite3.dylib\0",
        ];
        for name in candidates {
            let handle = libc::dlopen(name.as_ptr() as *const libc::c_char, libc::RTLD_LAZY);
            if !handle.is_null() {
                return Ok(Self { handle });
            }
        }
        bail!("Failed to dynamically load libsqlite3 from system libraries");
    }

    unsafe fn get_sym<T>(&self, sym: &str) -> Result<T> {
        let c_sym = CString::new(sym).unwrap();
        let sym_ptr = libc::dlsym(self.handle, c_sym.as_ptr());
        if !sym_ptr.is_null() {
            Ok(std::mem::transmute_copy(&sym_ptr))
        } else {
            bail!("Symbol '{}' not found in libsqlite3", sym)
        }
    }
}

#[cfg(not(windows))]
impl Drop for DynLib {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                libc::dlclose(self.handle);
            }
        }
    }
}

pub fn extract_credentials_from_db(db_path: &Path) -> Result<AppConfig> {
    if !db_path.exists() {
        bail!("Database file does not exist at {:?}", db_path);
    }

    let path_str = db_path.to_str().context("invalid unicode in database path")?;
    let c_path = CString::new(path_str).context("null byte in database path")?;

    unsafe {
        let lib = DynLib::load()?;
        let sqlite_open: FnOpen = lib.get_sym("sqlite3_open")?;
        let sqlite_close: FnClose = lib.get_sym("sqlite3_close")?;
        let sqlite_prepare: FnPrepareV2 = lib.get_sym("sqlite3_prepare_v2")?;
        let sqlite_step: FnStep = lib.get_sym("sqlite3_step")?;
        let sqlite_col_text: FnColumnText = lib.get_sym("sqlite3_column_text")?;
        let sqlite_col_int: FnColumnInt = lib.get_sym("sqlite3_column_int")?;
        let sqlite_finalize: FnFinalize = lib.get_sym("sqlite3_finalize")?;

        let mut db: *mut Sqlite3 = std::ptr::null_mut();
        if sqlite_open(c_path.as_ptr(), &mut db) != 0 || db.is_null() {
            bail!("sqlite3_open failed for {:?}", db_path);
        }

        struct DbGuard<'a>(*mut Sqlite3, FnClose, &'a DynLib);
        impl<'a> Drop for DbGuard<'a> {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    unsafe { (self.1)(self.0); }
                }
            }
        }
        let _guard = DbGuard(db, sqlite_close, &lib);

        // 1. Query mesh table
        let mesh_sql = CString::new("SELECT uuid, net_key, app_key FROM mesh LIMIT 1;").unwrap();
        let mut mesh_stmt: *mut Sqlite3Stmt = std::ptr::null_mut();
        if sqlite_prepare(db, mesh_sql.as_ptr(), -1, &mut mesh_stmt, std::ptr::null_mut()) != 0 {
            bail!("Failed to prepare mesh query");
        }

        let step_res = sqlite_step(mesh_stmt);
        if step_res != 100 { // SQLITE_ROW = 100
            sqlite_finalize(mesh_stmt);
            bail!("No mesh credentials found in database");
        }

        let mesh_uuid = CStr::from_ptr(sqlite_col_text(mesh_stmt, 0)).to_string_lossy().to_string();
        let net_key   = CStr::from_ptr(sqlite_col_text(mesh_stmt, 1)).to_string_lossy().to_string();
        let app_key   = CStr::from_ptr(sqlite_col_text(mesh_stmt, 2)).to_string_lossy().to_string();
        sqlite_finalize(mesh_stmt);

        // 2. Query fixtures table
        let fix_sql = CString::new("SELECT name, mac_address, node_address, device_key, device_uuid, code FROM fixtures;").unwrap();
        let mut fix_stmt: *mut Sqlite3Stmt = std::ptr::null_mut();
        let mut lights = Vec::new();

        if sqlite_prepare(db, fix_sql.as_ptr(), -1, &mut fix_stmt, std::ptr::null_mut()) == 0 {
            while sqlite_step(fix_stmt) == 100 {
                let name = CStr::from_ptr(sqlite_col_text(fix_stmt, 0)).to_string_lossy().to_string();
                let mac  = CStr::from_ptr(sqlite_col_text(fix_stmt, 1)).to_string_lossy().to_string();
                let address = sqlite_col_int(fix_stmt, 2) as u16;

                let dev_key_ptr = sqlite_col_text(fix_stmt, 3);
                let dev_key = if !dev_key_ptr.is_null() {
                    Some(CStr::from_ptr(dev_key_ptr).to_string_lossy().to_string())
                } else {
                    None
                };

                let dev_uuid_ptr = sqlite_col_text(fix_stmt, 4);
                let dev_uuid = if !dev_uuid_ptr.is_null() {
                    Some(CStr::from_ptr(dev_uuid_ptr).to_string_lossy().to_string())
                } else {
                    None
                };

                let code_ptr = sqlite_col_text(fix_stmt, 5);
                let code = if !code_ptr.is_null() {
                    Some(CStr::from_ptr(code_ptr).to_string_lossy().to_string())
                } else {
                    None
                };

                lights.push(LightConfig {
                    name,
                    mac,
                    address,
                    device_key: dev_key,
                    device_uuid: dev_uuid,
                    code,
                });
            }
            sqlite_finalize(fix_stmt);
        }

        if lights.is_empty() {
            bail!("No fixtures found in database");
        }

        Ok(AppConfig {
            mesh_uuid,
            net_key,
            app_key,
            lights,
        })
    }
}
