use crate::*;

/// debugging only
use near_sys::log_utf8;
pub fn log(message: &str) {
    unsafe {
        log_utf8(message.len() as _, message.as_ptr() as _);
    }
}

pub(crate) fn create_promise_batch(account_id: String, prev_id: Option<u64>) -> u64 {
    // then make another promise back receiver == current_account_id
    if let Some(prev_id) = prev_id {
        unsafe {
                near_sys::promise_batch_then(
                prev_id,
                account_id.len() as u64,
                account_id.as_ptr() as u64,
            )
        }
    } else {
        unsafe {
            near_sys::promise_batch_create(
                account_id.len() as u64,
                account_id.as_ptr() as u64,
            )
        }
    }
}

pub(crate) fn return_bytes_format(bytes: &[u8], json: bool) -> Vec<u8> {
    let mut ret_data = vec![];
    if json == true {
        let bytes_str = alloc::str::from_utf8(&bytes).ok().unwrap_or_else(|| sys::panic());
        ret_data.extend_from_slice(bytes_str
            // .replace("\"", "\\\"")
            .replace("|kP|", "")
            .replace("|kS|", "")
            .as_bytes()
        );
    } else {
        ret_data.extend_from_slice(&[DOUBLE_QUOTE_BYTE]);
        ret_data.extend_from_slice(bytes);
        ret_data.extend_from_slice(&[DOUBLE_QUOTE_BYTE]);
    }
    ret_data
}

pub(crate) fn return_value(bytes: &[u8]) {
    unsafe {
        near_sys::value_return(bytes.len() as u64, bytes.as_ptr() as u64);
    }
}

pub(crate) fn return_bytes(bytes: &[u8], json: bool) {
    return_value(&return_bytes_format(bytes, json));
}

pub(crate) fn swrite(key: &[u8], val: &[u8]) {
    //* SAFETY: Assumes valid storage_write implementation.
    unsafe {
        near_sys::storage_write(
            key.len() as u64,
            key.as_ptr() as u64,
            val.len() as u64,
            val.as_ptr() as u64,
            REGISTER_0,
        );
    }
}

pub(crate) fn storage_read_str(key: &[u8]) -> String {
    let data = storage_read(key);
	let data_str = alloc::str::from_utf8(&data).ok().unwrap_or_else(|| sys::panic());
    data_str.to_string()
}

pub(crate) fn storage_remove(key: &[u8]) {
    unsafe {
        near_sys::storage_remove(
            key.len() as u64,
            key.as_ptr() as u64,
            REGISTER_0
        );
    }
}

pub(crate) fn storage_read(key: &[u8]) -> Vec<u8> {
    let key_exists =
        unsafe { near_sys::storage_read(key.len() as u64, key.as_ptr() as u64, REGISTER_0) };
    if key_exists == 0 {
        // Return code of 0 means storage key had no entry.
        sys::panic()
    }
    register_read(REGISTER_0)
}

pub(crate) fn account_balance() -> Balance {
    let buffer = [0u8; 16];
    unsafe { near_sys::account_balance(buffer.as_ptr() as u64) };
    Balance::from_le_bytes(buffer)
}

pub(crate) fn sys_signer_pk_bytes() -> Vec<u8> {
    log("sys_signer_pk called!");

    unsafe {
        near_sys::signer_account_pk(REGISTER_0);
    }

    register_read(REGISTER_0)
}

pub(crate) fn sys_account_id(which: u8) -> String {
    unsafe {
        match which {
            0 => near_sys::current_account_id(REGISTER_0),
            1 => near_sys::predecessor_account_id(REGISTER_0),
            _ => near_sys::current_account_id(REGISTER_0),
        }
    };
    let current_account_id_bytes = register_read(REGISTER_0);
    alloc::str::from_utf8(&current_account_id_bytes).ok().unwrap_or_else(|| sys::panic()).to_string()
}

pub(crate) fn register_read(id: u64) -> Vec<u8> {
    let len = unsafe { near_sys::register_len(id) };
    if len == u64::MAX {
        // Register was not found
        sys::panic()
    }
    let data = vec![0u8; len as usize];

    //* SAFETY: Length of buffer is set dynamically based on `register_len` so it will always
    //* 		be sufficient length.
    unsafe { near_sys::read_register(id, data.as_ptr() as u64) };
    data
}

pub(crate) fn panic() -> ! {
    //* SAFETY: Assumed valid panic host function implementation
    unsafe { near_sys::panic() }
}