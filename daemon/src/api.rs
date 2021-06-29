use std::{
    net::SocketAddr,
    str::FromStr,
    sync::{Arc, RwLock},
    task::Poll,
};

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
        serializer.serialize_str(self.0.to_string().as_str())
    }
}

pub struct ChallengePeerResponse;

impl Serialize for ChallengePeerResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("ok")
    }
}

pub enum ServerEvent {
    NodeIdRequest(oneshot::Sender<NodeIdResponse>),
    ChallengePeerRequest(libp2p::PeerId, oneshot::Sender<ChallengePeerResponse>),
}

#[derive(Serialize)]
pub enum ServerNotification {
    MatchReady,
}

pub struct Server {
    event_rx: mpsc::UnboundedReceiver<ServerEvent>,
    local_addr: SocketAddr,

    subscribers: Arc<RwLock<Vec<jsonrpsee::ws_server::SubscriptionSink>>>,
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

        module.register_async_method("challenge_peer", move |params, event_tx| {
            let (res_tx, res_rx) = oneshot::channel::<ChallengePeerResponse>();

            let params_str: String = params.parse().unwrap();
            let peer_id = libp2p::PeerId::from_str(params_str.as_str()).unwrap();

            let _ = event_tx.send(ServerEvent::ChallengePeerRequest(peer_id, res_tx));

            async move { Ok(res_rx.await.unwrap()) }.boxed()
        })?;

        let subscribers = Arc::new(RwLock::new(vec![]));
        let subscribers_register = subscribers.clone();

        module.register_subscription("subscribe", "unsubscribe", move |_, sink, _| {
            subscribers_register
                .write()
                .map_err(|err| jsonrpsee::ws_server::Error::Custom(err.to_string()))?
                .push(sink);

            Ok(())
        })?;

        server.register_module(module)?;

        let local_addr = server.local_addr()?;
        tokio::spawn(server.start());

        Ok(Server {
            local_addr,
            event_rx,
            subscribers,
        })
    }

    pub fn notify(&mut self, notification: ServerNotification) {
        let mut subscribers = self
            .subscribers
            .write()
            .expect("failed acquiring subscribers lock");

        for i in (0..subscribers.len()).rev() {
            let mut sub = subscribers.swap_remove(i);

            if let Ok(_) = sub.send(&notification) {
                subscribers.push(sub);
            }
        }
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
