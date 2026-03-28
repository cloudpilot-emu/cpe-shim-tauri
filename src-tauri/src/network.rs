use std::{ffi::c_void, sync::Mutex};

use serde::Serialize;
use tauri::ipc::Channel;

use crate::network_ffi;

#[derive(Clone, Serialize)]
pub struct NetRpcResultPayload {
    session_id: u32,
    rpc_data: Vec<u8>,
}

static RPC_RESULT_CHANNEL: Mutex<Option<Channel<NetRpcResultPayload>>> = Mutex::new(None);

unsafe extern "C" fn rpc_result_callback(
    session_id: u32,
    data: *const u8,
    len: usize,
    _ctx: *mut c_void,
) {
    let rpc_data = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
    let payload = NetRpcResultPayload {
        session_id,
        rpc_data,
    };

    if let Some(channel) = RPC_RESULT_CHANNEL.lock().unwrap().as_ref() {
        if let Err(err) = channel.send(payload) {
            println!("failed to send RPC result to webview: {}", err);
        }
    } else {
        println!("failed to send RPC result to webview: channel not configured");
    }
}

pub fn init() {
    unsafe {
        network_ffi::net_setRpcCallback(rpc_result_callback, std::ptr::null_mut());
    }
}

#[tauri::command]
pub fn net_set_rpc_result_channel(channel: Channel<NetRpcResultPayload>) {
    *RPC_RESULT_CHANNEL.lock().unwrap() = Some(channel.clone());
}

#[tauri::command]
pub async fn net_open_session() -> u32 {
    unsafe { network_ffi::net_openSession() }
}

#[tauri::command]
pub async fn net_close_session(session_id: u32) {
    unsafe { network_ffi::net_closeSession(session_id) }
}

#[tauri::command]
pub async fn net_dispatch_rpc(session_id: u32, rpc_data: Vec<u8>) -> bool {
    unsafe { network_ffi::net_dispatchRpc(session_id, rpc_data.as_ptr(), rpc_data.len()) }
}

pub fn net_close_all_sessions() {
    unsafe { network_ffi::net_closeAllSessions() }
}
