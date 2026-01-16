use std::{net::IpAddr, sync::mpsc::Sender};

#[derive(Debug)]
pub(crate) struct RemoveConnection {
    pub address: IpAddr,
    pub tx: Sender<(String, Option<Sender<String>>)>,
}

impl Drop for RemoveConnection {
    fn drop(&mut self) {
        let _ok = self
            .tx
            .send((format!("0 server connection_remove {}", self.address), None));
    }
}
