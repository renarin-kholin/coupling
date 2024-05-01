use serde::{Deserialize, Serialize};

use super::user::UserId;

type SDP = String;

#[derive(Serialize, Deserialize)]
pub enum CCMessage {
    SetClientId(UserId),
    CallRequest(UserId),
    CallRequestFailure,
    CallAnswer(bool, Option<SDP>),
    CallReply(SDP),
}
