use tokio::{
    runtime::Runtime,
    sync::{Notify, RwLock},
};

pub struct ServerState {
    pub(crate) changed: Notify,
    pub(crate) typstname: RwLock<String>,
    pub(crate) tokio: Runtime,
    pub(crate) shutdown: Notify,
}
