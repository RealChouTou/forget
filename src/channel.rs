use std::{future::Future, pin::Pin};

use uuid::Uuid;

use crate::models::SessionMessage;

pub trait Channel: Send + Sync {
    fn publish(
        &self,
        session_id: Uuid,
        msg: SessionMessage,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}
