/// common constants and structures for relay communication

// Error responses
pub static CANT_REGISTER_RESPONSE:&str = "Can't register peer";
pub static RELAY_ERROR_RESPONSE:&str = "Can't relay message";
pub static STATE_NOT_INITIALIZED:&str = "Relay sessions state is not initialized";
pub static RELAY_MESSAGE_DELIMITER:&str = ":::";
pub static NOT_YOUR_TURN:&str = "Not this peers turn";

/// eddsa constants
pub static PK_MESSAGE_PREFIX:&str = "PUBLIC_KEY";
pub static COMMITMENT_MESSAGE_PREFIX:&str = "COMMITMENT";
pub static R_KEY_MESSAGE_PREFIX:&str = "R_KEY";
pub static SIGNATURE_MESSAGE_PREFIX:&str = "SIGNATURE";