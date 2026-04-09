use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Clone, Copy)]
#[repr(u8)]
#[derive(Serialize_repr, Deserialize_repr)]
pub enum AppChannel {
    Preview = 0,
    Stable = 1,
}

impl AppChannel {
    pub fn from_number<T: Into<i64>>(value: T) -> Option<AppChannel> {
        match Into::<i64>::into(value) {
            0 => Some(AppChannel::Preview),
            1 => Some(AppChannel::Stable),
            _ => None,
        }
    }
}
