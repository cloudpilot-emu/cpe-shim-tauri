use std::time::Instant;

pub type NetworkSessionConsent = Option<(bool, Instant)>;

#[derive(Default)]
pub struct State {
    pub network_session_consent: NetworkSessionConsent,
}
