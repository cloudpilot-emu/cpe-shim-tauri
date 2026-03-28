use std::{ffi::c_void, sync::Mutex};

use serde::Serialize;
use tauri::{ipc::Channel, AppHandle, Runtime};
use tauri_plugin_notification::{Notification, NotificationExt, PermissionState};

use crate::network_ffi;

enum OpenSessionError {
    NotificationPermissionRequired = -1,
    Other = -2,
}

enum RequestNotificationPermissionResult {
    Granted,
    Denied,
    Failed,
}

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
pub async fn net_open_session(app_handle: AppHandle) -> isize {
    match request_notification_permission(&app_handle.notification()) {
        RequestNotificationPermissionResult::Failed => return OpenSessionError::Other as isize,
        RequestNotificationPermissionResult::Denied => {
            return OpenSessionError::NotificationPermissionRequired as isize
        }
        RequestNotificationPermissionResult::Granted => (),
    }

    if let Err(err) = app_handle
        .notification()
        .builder()
        .title("Network activity")
        .body("CloudpilotEmu started a network session")
        .show()
    {
        println!("failed to show network session notification {}", err);
        return OpenSessionError::Other as isize;
    }

    println!("opening network session");

    unsafe { network_ffi::net_openSession() as isize }
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

fn request_notification_permission<T: Runtime>(
    notification: &Notification<T>,
) -> RequestNotificationPermissionResult {
    match notification.permission_state() {
        Err(err) => {
            println!("querying notification permission failed: {}", err);
            return RequestNotificationPermissionResult::Failed;
        }
        Ok(PermissionState::Denied) => {
            println!("notification permission denied");
            return RequestNotificationPermissionResult::Denied;
        }
        Ok(PermissionState::Granted) => return RequestNotificationPermissionResult::Granted,
        Ok(PermissionState::Prompt) | Ok(PermissionState::PromptWithRationale) => (),
    };

    match notification.request_permission() {
        Err(err) => {
            println!("requesting notification permission failed: {}", err);
            RequestNotificationPermissionResult::Failed
        }
        Ok(PermissionState::Granted) => RequestNotificationPermissionResult::Granted,
        _ => RequestNotificationPermissionResult::Denied,
    }
}
