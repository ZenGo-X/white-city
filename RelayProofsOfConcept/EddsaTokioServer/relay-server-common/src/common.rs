/// common constants and structures for relay communication
use super::MessagePayload;
// Error responses
pub static CANT_REGISTER_RESPONSE: &str = "Can't register peer";
pub static RELAY_ERROR_RESPONSE: &str = "Can't relay message";
pub static STATE_NOT_INITIALIZED: &str = "Relay sessions state is not initialized";
pub static RELAY_MESSAGE_DELIMITER: &str = ":::";
pub static NOT_YOUR_TURN: &str = "Not this peers turn";

/// eddsa constants
pub static PK_MESSAGE_PREFIX: &str = "PUBLIC_KEY";
pub static COMMITMENT_MESSAGE_PREFIX: &str = "COMMITMENT";
pub static R_KEY_MESSAGE_PREFIX: &str = "R_KEY";
pub static R_KEY_MESSAGE_DELIMITER: &str = "@";
pub static SIGNATURE_MESSAGE_PREFIX: &str = "SIGNATURE";

pub static EMPTY_MESSAGE_PAYLOAD: &str = "";

pub fn generate_pk_message_payload(pk: &String) -> MessagePayload {
    return format!(
        "{}{}{}",
        PK_MESSAGE_PREFIX,
        RELAY_MESSAGE_DELIMITER,
        pk.clone()
    );
}

pub fn generate_commitment_message_payload(cmtnmt: &String) -> MessagePayload {
    return format!(
        "{prefix}{delimiter}{message}",
        prefix = COMMITMENT_MESSAGE_PREFIX,
        delimiter = RELAY_MESSAGE_DELIMITER,
        message = cmtnmt.clone()
    );
}

pub fn generate_R_message_payload(r: &String) -> MessagePayload {
    return format!(
        "{prefix}{delimiter}{message}",
        prefix = R_KEY_MESSAGE_PREFIX,
        delimiter = RELAY_MESSAGE_DELIMITER,
        message = r.clone()
    );
}

pub fn generate_signature_message_payload(sig: &String) -> MessagePayload {
    return format!(
        "{prefix}{delimiter}{message}",
        prefix = SIGNATURE_MESSAGE_PREFIX,
        delimiter = RELAY_MESSAGE_DELIMITER,
        message = sig.clone()
    );
}
