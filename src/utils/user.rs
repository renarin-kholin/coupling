use std::{collections::HashMap, sync::Arc};

use rand::{distributions::Alphanumeric, Rng};
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;

pub type UserId = String;
pub type Users = Arc<RwLock<HashMap<UserId, mpsc::UnboundedSender<Message>>>>;
pub type UserConnections = Arc<RwLock<HashMap<UserId, UserId>>>; //Initiator is the key

//Make this ID longer and unique in the future and make it a behind the curtains implementation
pub fn generate_userid() -> UserId {
    let user_id = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(4)
        .map(char::from)
        .collect();
    user_id
}
