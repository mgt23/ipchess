use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::task::Poll;

use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent, IdentifyInfo};
use libp2p::kad::{self, KademliaConfig};
use libp2p::kad::{store::MemoryStore, Kademlia, KademliaEvent};
use libp2p::swarm::protocols_handler::DummyProtocolsHandler;
use libp2p::swarm::{
    IntoProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess,
    ProtocolsHandler,
};

use libp2p::{NetworkBehaviour, PeerId};

use crate::protocol::{Ipchess, IpchessEvent};

const BOOTSTRAP_PEER_ADDRS: [&str; 5] = [
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
    "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
];

pub enum PeerStoreEvent {}

pub struct PeerStore {
    connected_peers: HashMap<PeerId, Option<IdentifyInfo>>,
}

impl PeerStore {
    fn new() -> Self {
        Self {
            connected_peers: HashMap::new(),
        }
    }

    fn add_identify_info(&mut self, peer_id: PeerId, info: IdentifyInfo) {
        self.connected_peers
            .entry(peer_id)
            .and_modify(|e| *e = Some(info));
    }

    fn peers_for_protocol<'a>(&'a self, protocol: &'a str) -> impl Iterator<Item = PeerId> + 'a {
        self.connected_peers
            .iter()
            .filter_map(move |(peer_id, info)| match info {
                Some(info) => {
                    let found = info.protocols.iter().any(|p| p == protocol);

                    if found {
                        Some(*peer_id)
                    } else {
                        None
                    }
                }

                None => None,
            })
    }
}

impl NetworkBehaviour for PeerStore {
    type ProtocolsHandler = DummyProtocolsHandler;
    type OutEvent = PeerStoreEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        DummyProtocolsHandler::default()
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<libp2p::Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, _peer_id: &PeerId) {}

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _conn_id: &libp2p::core::connection::ConnectionId,
        _endpoint: &libp2p::core::ConnectedPoint,
    ) {
        self.connected_peers.entry(*peer_id).or_default();
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        self.connected_peers.remove(&peer_id);
    }

    fn inject_event(
        &mut self,
        _peer_id: PeerId,
        _connection: libp2p::core::connection::ConnectionId,
        _event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent,
    ) {
    }

    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
        _params: &mut impl libp2p::swarm::PollParameters,
    ) -> Poll<
        NetworkBehaviourAction<
            <Self::ProtocolsHandler as ProtocolsHandler>::InEvent,
            Self::OutEvent,
        >,
    > {
        Poll::Pending
    }
}

#[derive(Debug)]
pub enum BehaviourEvent {
    Ipchess(IpchessEvent),
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourEvent")]
#[behaviour(poll_method = "poll")]
pub struct Behaviour {
    identify: Identify,
    kad: Kademlia<MemoryStore>,
    ipchess: Ipchess,
    peer_store: PeerStore,

    #[behaviour(ignore)]
    challenged_peer_id: Option<PeerId>,
    #[behaviour(ignore)]
    events: VecDeque<
        NetworkBehaviourAction<
            <<<Self as NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::InEvent,
            BehaviourEvent,
        >,
    >,
}

impl Behaviour {
    pub fn new(peer_id: PeerId, public_key: libp2p::identity::PublicKey) -> Self {
        let mut kad_config = KademliaConfig::default();
        kad_config.set_record_ttl(Some(std::time::Duration::from_secs(0)));
        kad_config.set_provider_record_ttl(Some(std::time::Duration::from_secs(0)));
        kad_config.set_kbucket_inserts(kad::KademliaBucketInserts::Manual);

        let mut kad = Kademlia::with_config(peer_id, MemoryStore::new(peer_id), kad_config);

        for addr in BOOTSTRAP_PEER_ADDRS.iter() {
            let mut ma = libp2p::multiaddr::Multiaddr::from_str(addr)
                .expect("invalid bootstrap peer address");
            let p2p = ma.pop().unwrap();

            match p2p {
                libp2p::multiaddr::Protocol::P2p(peer_id) => {
                    kad.add_address(
                        &libp2p::PeerId::from_multihash(peer_id)
                            .expect("missing /p2p in bootstrap peer address"),
                        ma,
                    );
                }

                _ => unreachable!("oh boy"),
            }
        }

        let identify_config = IdentifyConfig::new("ipchess/libp2p".into(), public_key);
        let identify = Identify::new(identify_config);

        let ipchess = Ipchess::new();

        Self {
            identify,
            kad,
            ipchess,
            peer_store: PeerStore::new(),

            challenged_peer_id: None,
            events: VecDeque::new(),
        }
    }

    pub fn bootstrap(&mut self) {
        self.kad.bootstrap().unwrap();
    }

    pub fn challenge_peer(&mut self, peer_id: PeerId) {
        log::debug!("Challenging peer {}", peer_id);
        self.challenged_peer_id = Some(peer_id);

        if self.addresses_of_peer(&peer_id).is_empty() {
            log::debug!(
                "No addresses found for peer {}, starting DHT query",
                peer_id
            );
            self.kad.get_closest_peers(peer_id);
        } else {
            log::debug!(
                "Addresses for peer {} found, starting challenge request",
                peer_id
            );
            self.ipchess.challenge_peer(peer_id);
        }
    }

    pub fn accept_peer_challenge(&mut self, peer_id: PeerId) {
        log::debug!("Accepting challenge from peer {}", peer_id);
        self.ipchess.accept_peer_challenge(peer_id);
    }

    pub fn cancel_challenge(&mut self, peer_id: PeerId) {
        log::debug!("Cancelling challenge to peer {}", peer_id);
        self.ipchess.cancel_challenge(peer_id);
    }

    pub fn decline_peer_challenge(&mut self, peer_id: PeerId) {
        log::debug!("Declining challenge from peer {}", peer_id);
        self.ipchess.decline_peer_challenge(peer_id);
    }

    pub fn is_connected(&self) -> bool {
        self.peer_store
            .peers_for_protocol(
                std::str::from_utf8(kad::protocol::DEFAULT_PROTO_NAME)
                    .expect("Kademlia protocol name is weird"),
            )
            .count()
            > 0
    }

    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
        _params: &mut impl libp2p::swarm::PollParameters
    ) -> Poll<NetworkBehaviourAction<<<<Self as NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::InEvent, <Self as NetworkBehaviour>::OutEvent>>{
        // drain events
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}

impl NetworkBehaviourEventProcess<IdentifyEvent> for Behaviour {
    fn inject_event(&mut self, event: IdentifyEvent) {
        if let IdentifyEvent::Received { peer_id, info } = event {
            for addr in info.listen_addrs.iter() {
                self.kad.add_address(&peer_id, addr.clone());
            }

            self.peer_store.add_identify_info(peer_id, info.clone());

            let challenged_peer_id = if let Some(challenged_peer_id) = &self.challenged_peer_id {
                *challenged_peer_id
            } else {
                return;
            };

            if peer_id != challenged_peer_id {
                return;
            }

            log::debug!(
                "Identified challenged peer {} {:?}, starting challenge request",
                peer_id,
                info,
            );

            self.challenged_peer_id = None;

            for addr in info.listen_addrs {
                self.ipchess.add_address(peer_id, addr);
            }

            self.ipchess.challenge_peer(peer_id);
        }
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for Behaviour {
    fn inject_event(&mut self, _event: KademliaEvent) {}
}

impl NetworkBehaviourEventProcess<IpchessEvent> for Behaviour {
    fn inject_event(&mut self, event: IpchessEvent) {
        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            BehaviourEvent::Ipchess(event),
        ));
    }
}

impl NetworkBehaviourEventProcess<PeerStoreEvent> for Behaviour {
    fn inject_event(&mut self, _: PeerStoreEvent) {}
}
