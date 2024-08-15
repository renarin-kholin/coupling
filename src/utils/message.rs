use serde::{Deserialize, Serialize};

use super::user::UserId;
pub type SDP = String;


#[derive(Serialize, Deserialize)]
pub enum WSCommand {
    SetClientId(UserId),
    CallRequest(UserId),
    CallRequestFailure,
    CallAnswer(bool, Option<SDP>),
    CallReply(SDP),
}
