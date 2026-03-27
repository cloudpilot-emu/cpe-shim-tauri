use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::network_ffi;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static NET_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Clone, Serialize)]
struct NetRpcResultPayload {
    session_id: u32,
    rpc_data: Vec<u8>,
}

unsafe extern "C" fn rpc_result_callback(
    session_id: u32,
    data: *const u8,
    len: usize,
    _ctx: *mut c_void,
) {
    let rpc_data = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
    if let Some(handle) = APP_HANDLE.get() {
        let _ = handle.emit(
            "net-rpc-result",
            NetRpcResultPayload {
                session_id,
                rpc_data,
            },
        );
    }
}

pub fn init(handle: &AppHandle) {
    APP_HANDLE.set(handle.clone()).ok();
    unsafe {
        network_ffi::net_setRpcCallback(rpc_result_callback, std::ptr::null_mut());
    }
}

#[tauri::command]
pub async fn net_open_session() -> u32 {
    let _lock = NET_MUTEX.lock().unwrap();
    unsafe { network_ffi::net_openSession() }
}

#[tauri::command]
pub async fn net_close_session(session_id: u32) {
    let _lock = NET_MUTEX.lock().unwrap();
    unsafe { network_ffi::net_closeSession(session_id) }
}

#[tauri::command]
pub async fn net_dispatch_rpc(session_id: u32, rpc_data: Vec<u8>) -> bool {
    let _lock = NET_MUTEX.lock().unwrap();
    unsafe { network_ffi::net_dispatchRpc(session_id, rpc_data.as_ptr(), rpc_data.len()) }
}

pub fn net_close_all_sessions() {
    let _lock = NET_MUTEX.lock().unwrap();
    unsafe { network_ffi::net_closeAllSessions() }
}
