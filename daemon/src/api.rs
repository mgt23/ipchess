use std::{net::SocketAddr, task::Poll};

use futures::FutureExt;
use jsonrpsee::ws_server::{RpcModule, WsServerBuilder};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

pub struct NodeIdResponse(pub libp2p::PeerId);

impl Serialize for NodeIdResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self.0.to_string().as_str())
    }
}

pub enum ServerEvent {
    NodeIdRequest(oneshot::Sender<NodeIdResponse>),
}

pub struct Server {
    event_rx: mpsc::UnboundedReceiver<ServerEvent>,
    local_addr: SocketAddr,
}

impl Server {
    pub async fn new(port: u16) -> anyhow::Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let mut server = WsServerBuilder::default()
            .build(format!("127.0.0.1:{}", port))
            .await?;

        let mut module = RpcModule::new(event_tx);

        module.register_async_method("node_id", move |_, event_tx| {
            let (res_tx, res_rx) = oneshot::channel::<NodeIdResponse>();
            let _ = event_tx.send(ServerEvent::NodeIdRequest(res_tx));

            async move { Ok(res_rx.await.unwrap()) }.boxed()
        })?;

        server.register_module(module)?;

        let local_addr = server.local_addr()?;
        tokio::spawn(server.start());

        Ok(Server {
            local_addr,
            event_rx,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl futures::Stream for Server {
    type Item = ServerEvent;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.event_rx.poll_recv(cx)
    }
}
