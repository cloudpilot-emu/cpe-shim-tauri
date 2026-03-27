use std::ffi::c_void;

pub type NetRpcResultCb = unsafe extern "C" fn(u32, *const u8, usize, *mut c_void);

unsafe extern "C" {
    pub fn net_setRpcCallback(result_cb: NetRpcResultCb, context: *mut c_void);
    pub fn net_openSession() -> u32;
    pub fn net_closeSession(session_id: u32);
    pub fn net_closeAllSessions();
    pub fn net_dispatchRpc(session_id: u32, data: *const u8, len: usize) -> bool;
}
