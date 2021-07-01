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

use crate::utils::SerializablePeerId;

#[derive(Serialize)]
pub struct NodeIdResponse(pub SerializablePeerId);

#[derive(Serialize)]
pub struct IsConnectedResponse(pub bool);

pub struct ChallengePeerResponse;

impl Serialize for ChallengePeerResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("ok")
    }
}

#[derive(Serialize)]
pub struct AcceptPeerChallengeResponse;

pub enum ServerEvent {
    NodeIdRequest(oneshot::Sender<NodeIdResponse>),
    IsConnectedRequest(oneshot::Sender<IsConnectedResponse>),
    ChallengePeerRequest(libp2p::PeerId, oneshot::Sender<ChallengePeerResponse>),
    AcceptPeerChallengeRequest(libp2p::PeerId, oneshot::Sender<AcceptPeerChallengeResponse>),
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "event_type", content = "data")]
pub enum ServerEventNotification {
    PeerChallenge { peer_id: SerializablePeerId },
    MatchReady { peer_id: SerializablePeerId },
}

pub struct Server {
    event_rx: mpsc::UnboundedReceiver<ServerEvent>,
    local_addr: SocketAddr,

    events_subscribers: Arc<RwLock<Vec<jsonrpsee::ws_server::SubscriptionSink>>>,
}

impl Server {
    pub async fn new(port: u16) -> anyhow::Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let mut server = WsServerBuilder::default()
            .build(format!("127.0.0.1:{}", port))
            .await?;

        let mut module = RpcModule::new(event_tx);

        module.register_async_method("node_id", move |_, event_tx| {
            let (res_tx, res_rx) = oneshot::channel();
            let _ = event_tx.send(ServerEvent::NodeIdRequest(res_tx));

            async move { Ok(res_rx.await.unwrap()) }.boxed()
        })?;

        module.register_async_method("is_connected", move |_, event_tx| {
            let (res_tx, res_rx) = oneshot::channel();
            let _ = event_tx.send(ServerEvent::IsConnectedRequest(res_tx));

            async move { Ok(res_rx.await.unwrap()) }.boxed()
        })?;

        module.register_async_method("challenge_peer", move |params, event_tx| {
            let (res_tx, res_rx) = oneshot::channel();

            let params_str: String = params.one().unwrap();
            let peer_id = libp2p::PeerId::from_str(params_str.as_str()).unwrap();

            let _ = event_tx.send(ServerEvent::ChallengePeerRequest(peer_id, res_tx));

            async move { Ok(res_rx.await.unwrap()) }.boxed()
        })?;

        module.register_async_method("accept_peer_challenge", move |params, event_tx| {
            let (res_tx, res_rx) = oneshot::channel();

            let params_str: String = params.one().unwrap();
            let peer_id = libp2p::PeerId::from_str(params_str.as_str()).unwrap();

            let _ = event_tx.send(ServerEvent::AcceptPeerChallengeRequest(peer_id, res_tx));

            async move { Ok(res_rx.await.unwrap()) }.boxed()
        })?;

        let events_subscribers = Arc::new(RwLock::new(vec![]));
        let events_subscribers_register = events_subscribers.clone();

        module.register_subscription(
            "subscribe_events",
            "unsubscribe_events",
            move |_, sink, _| {
                events_subscribers_register
                    .write()
                    .map_err(|err| jsonrpsee::ws_server::Error::Custom(err.to_string()))?
                    .push(sink);

                Ok(())
            },
        )?;

        server.register_module(module)?;

        let local_addr = server.local_addr()?;
        tokio::spawn(server.start());

        Ok(Server {
            event_rx,
            local_addr,
            events_subscribers,
        })
    }

    pub fn notify_event(&mut self, notification: ServerEventNotification) {
        let mut events_subscribers = self
            .events_subscribers
            .write()
            .expect("failed acquiring subscribers lock");

        for i in (0..events_subscribers.len()).rev() {
            let mut sub = events_subscribers.swap_remove(i);

            if sub.send(&notification).is_ok() {
                events_subscribers.push(sub);
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
