use std::{ffi::c_void, ops::DerefMut, sync::Mutex, time::Instant};

use serde::Serialize;
use tauri::{ipc::Channel, AppHandle, Manager, Runtime};
use tauri_plugin_dialog::{
    Dialog, DialogExt, MessageDialogButtons::OkCancelCustom, MessageDialogResult,
};
use tauri_plugin_notification::{Notification, NotificationExt, PermissionState};
use tokio::sync::oneshot;

use crate::{
    network_ffi,
    state::{NetworkSessionConsent, State},
};

enum OpenSessionError {
    NotificationPermissionRequired = -1,
    Other = -2,
}

enum RequestNotificationPermissionResult {
    Granted,
    Denied,
    Failed,
}

const CONSENT_ALLOW_TIMEOUT_SECONDS: u64 = 60 * 5;
const CONSENT_DENY_TIMEOUT_SECONDS: u64 = 5;

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
    let previous_consent = app_handle
        .state::<Mutex<State>>()
        .inner()
        .lock()
        .unwrap()
        .network_session_consent;

    let consent = get_consent(previous_consent, app_handle.dialog()).await;

    app_handle
        .state::<Mutex<State>>()
        .inner()
        .lock()
        .unwrap()
        .deref_mut()
        .network_session_consent = consent;

    if let Some((true, _)) = consent {
    } else {
        return OpenSessionError::Other as isize;
    }

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

fn request_notification_permission<R: Runtime>(
    notification: &Notification<R>,
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

async fn get_consent<R: Runtime>(
    previous_consent: NetworkSessionConsent,
    dialog: &Dialog<R>,
) -> NetworkSessionConsent {
    if let Some((consent, timestamp)) = previous_consent {
        let age_seconds = Instant::now().duration_since(timestamp).as_secs();

        if consent && (age_seconds <= CONSENT_ALLOW_TIMEOUT_SECONDS) {
            return previous_consent;
        }

        if !consent && (age_seconds <= CONSENT_DENY_TIMEOUT_SECONDS) {
            return previous_consent;
        }
    }

    let (tx, rx) = oneshot::channel::<bool>();
    dialog
        .message("PalmOS is trying to access the network.")
        .title("Network access")
        .buttons(OkCancelCustom("Allow".into(), "Deny".into()))
        .show_with_result(|result| {
            let _ = tx.send(result == MessageDialogResult::Custom("Allow".into()));
        });

    let consent = rx.await;

    Some((consent.unwrap(), Instant::now()))
}
