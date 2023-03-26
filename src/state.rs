use crate::Args;
use tokio::{runtime::Runtime, sync::Notify};

pub struct ServerState {
    pub(crate) args: Args,
    pub(crate) changed: Notify,
    pub(crate) tokio: Runtime,
    pub(crate) shutdown: Notify,
}
