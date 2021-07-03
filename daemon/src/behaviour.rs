use std::collections::{HashMap, HashSet, VecDeque};
use std::str::FromStr;
use std::task::Poll;

use libp2p::core::either::EitherOutput;
use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent};
use libp2p::kad::handler::KademliaHandlerProto;
use libp2p::kad::{self, KademliaConfig, KademliaEvent};
use libp2p::kad::{store::MemoryStore, Kademlia};
use libp2p::swarm::{
    IntoProtocolsHandler, IntoProtocolsHandlerSelect, NetworkBehaviour, NetworkBehaviourAction,
    NetworkBehaviourEventProcess, ProtocolsHandler, ProtocolsHandlerSelect,
};

use libp2p::{Multiaddr, PeerId};

use crate::protocol::{Ipchess, IpchessEvent, IpchessHandler};

const BOOTSTRAP_PEER_ADDRS: [&str; 5] = [
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
    "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
];

#[derive(Debug)]
pub enum BehaviourEvent {
    Ipchess(IpchessEvent),
}

type IdentifyHandler = <Identify as NetworkBehaviour>::ProtocolsHandler;
type KademliaHandler = KademliaHandlerProto<libp2p::kad::QueryId>;

pub type BehaviourHandler = IntoProtocolsHandlerSelect<
    KademliaHandler,
    ProtocolsHandlerSelect<IdentifyHandler, IpchessHandler>,
>;

struct PeerInfo {
    addrs: VecDeque<Multiaddr>,
    protocols: HashSet<String>,
}

impl Default for PeerInfo {
    fn default() -> Self {
        Self {
            addrs: VecDeque::new(),
            protocols: HashSet::new(),
        }
    }
}

pub struct Behaviour {
    identify: Identify,
    kad: Kademlia<MemoryStore>,
    ipchess: Ipchess,

    peer_infos: HashMap<PeerId, PeerInfo>,

    actions_queue: VecDeque<
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

            peer_infos: HashMap::new(),
            actions_queue: VecDeque::new(),
        }
    }

    pub fn bootstrap(&mut self) {
        self.kad.bootstrap().unwrap();
    }

    pub fn challenge_peer(&mut self, peer_id: PeerId) {
        log::debug!("Challenging peer {}", peer_id);
        self.ipchess.challenge_peer(peer_id);
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

    pub fn is_connected(&mut self) -> bool {
        true
    }
}

macro_rules! delegate_to_behaviours {
    ($self: ident, $fn: ident, $($arg: ident), *) => {
        $self.identify.$fn($($arg),*);
        $self.kad.$fn($($arg),*);
        $self.ipchess.$fn($($arg),*);
    };
}

impl NetworkBehaviour for Behaviour {
    type ProtocolsHandler = BehaviourHandler;
    type OutEvent = BehaviourEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        IntoProtocolsHandler::select(
            self.kad.new_handler(),
            ProtocolsHandler::select(self.identify.new_handler(), self.ipchess.new_handler()),
        )
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<libp2p::Multiaddr> {
        match self.peer_infos.get(peer_id) {
            Some(info) => info.addrs.iter().cloned().collect(),
            None => self.kad.addresses_of_peer(peer_id),
        }
    }

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        conn_id: &libp2p::core::connection::ConnectionId,
        endpoint: &libp2p::core::ConnectedPoint,
    ) {
        // Move new address to the front of the known addresses list.
        // That way we'll dial it first next time.
        let peer_info = self.peer_infos.entry(*peer_id).or_default();
        let conn_address = match endpoint {
            libp2p::core::ConnectedPoint::Dialer { address } => address,
            libp2p::core::ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        peer_info.addrs.retain(|addr| addr != conn_address);
        peer_info.addrs.push_front(conn_address.clone());

        delegate_to_behaviours!(
            self,
            inject_connection_established,
            peer_id,
            conn_id,
            endpoint
        );
    }

    fn inject_addr_reach_failure(
        &mut self,
        peer_id: Option<&PeerId>,
        addr: &libp2p::Multiaddr,
        error: &dyn std::error::Error,
    ) {
        // Remove unreachable address from known addresses list.
        if let Some(peer_id) = peer_id {
            self.peer_infos
                .entry(*peer_id)
                .and_modify(|e| e.addrs.retain(|known_addr| known_addr != addr));
        }

        delegate_to_behaviours!(self, inject_addr_reach_failure, peer_id, addr, error);
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: libp2p::core::connection::ConnectionId,
        event: <<Self::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::OutEvent,
    ) {
        match event {
            EitherOutput::First(kad_handler_event) => {
                self.kad
                    .inject_event(peer_id, connection, kad_handler_event);
            }
            EitherOutput::Second(e) => match e {
                EitherOutput::First(identify_handler_event) => {
                    self.identify
                        .inject_event(peer_id, connection, identify_handler_event);
                }
                EitherOutput::Second(ipchess_handler_event) => {
                    self.ipchess
                        .inject_event(peer_id, connection, ipchess_handler_event);
                }
            },
        }
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
        params: &mut impl libp2p::swarm::PollParameters,
    ) -> Poll<
        NetworkBehaviourAction<
            <<Self::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::InEvent,
            Self::OutEvent,
        >,
    >{
        if let Poll::Ready(e) = self.identify.poll(cx, params) {
            match e {
                NetworkBehaviourAction::GenerateEvent(event) => {
                    <Self as NetworkBehaviourEventProcess<IdentifyEvent>>::inject_event(
                        self, event,
                    );
                }

                NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                } => {
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler,
                        event: EitherOutput::Second(EitherOutput::First(event)),
                    })
                }

                NetworkBehaviourAction::DialAddress { address } => {
                    return Poll::Ready(NetworkBehaviourAction::DialAddress { address })
                }

                NetworkBehaviourAction::DialPeer { peer_id, condition } => {
                    return Poll::Ready(NetworkBehaviourAction::DialPeer { peer_id, condition })
                }

                NetworkBehaviourAction::ReportObservedAddr { address, score } => {
                    return Poll::Ready(NetworkBehaviourAction::ReportObservedAddr {
                        address,
                        score,
                    })
                }
            }
        }

        if let Poll::Ready(e) = self.kad.poll(cx, params) {
            match e {
                NetworkBehaviourAction::GenerateEvent(event) => {
                    <Self as NetworkBehaviourEventProcess<KademliaEvent>>::inject_event(
                        self, event,
                    );
                }

                NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                } => {
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler,
                        event: EitherOutput::First(event),
                    })
                }

                NetworkBehaviourAction::DialAddress { address } => {
                    return Poll::Ready(NetworkBehaviourAction::DialAddress { address })
                }

                NetworkBehaviourAction::DialPeer { peer_id, condition } => {
                    return Poll::Ready(NetworkBehaviourAction::DialPeer { peer_id, condition })
                }

                NetworkBehaviourAction::ReportObservedAddr { address, score } => {
                    return Poll::Ready(NetworkBehaviourAction::ReportObservedAddr {
                        address,
                        score,
                    })
                }
            }
        }

        if let Poll::Ready(e) = self.ipchess.poll(cx, params) {
            match e {
                NetworkBehaviourAction::GenerateEvent(event) => {
                    <Self as NetworkBehaviourEventProcess<IpchessEvent>>::inject_event(self, event);
                }

                NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                } => {
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler,
                        event: EitherOutput::Second(EitherOutput::Second(event)),
                    })
                }

                NetworkBehaviourAction::DialAddress { address } => {
                    return Poll::Ready(NetworkBehaviourAction::DialAddress { address })
                }

                NetworkBehaviourAction::DialPeer { peer_id, condition } => {
                    return Poll::Ready(NetworkBehaviourAction::DialPeer { peer_id, condition })
                }

                NetworkBehaviourAction::ReportObservedAddr { address, score } => {
                    return Poll::Ready(NetworkBehaviourAction::ReportObservedAddr {
                        address,
                        score,
                    })
                }
            }
        }

        Poll::Pending
    }

    // Empty inject_*
    fn inject_connected(&mut self, peer_id: &PeerId) {
        delegate_to_behaviours!(self, inject_connected, peer_id);
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        delegate_to_behaviours!(self, inject_disconnected, peer_id);
    }

    fn inject_connection_closed(
        &mut self,
        peer_id: &PeerId,
        conn_id: &libp2p::core::connection::ConnectionId,
        endpoint: &libp2p::core::ConnectedPoint,
    ) {
        delegate_to_behaviours!(self, inject_connection_closed, peer_id, conn_id, endpoint);
    }

    fn inject_address_change(
        &mut self,
        peer_id: &PeerId,
        conn_id: &libp2p::core::connection::ConnectionId,
        old: &libp2p::core::ConnectedPoint,
        new: &libp2p::core::ConnectedPoint,
    ) {
        delegate_to_behaviours!(self, inject_address_change, peer_id, conn_id, old, new);
    }

    fn inject_dial_failure(&mut self, peer_id: &PeerId) {
        delegate_to_behaviours!(self, inject_dial_failure, peer_id);
    }

    fn inject_new_listener(&mut self, id: libp2p::core::connection::ListenerId) {
        delegate_to_behaviours!(self, inject_new_listener, id);
    }

    fn inject_new_listen_addr(
        &mut self,
        id: libp2p::core::connection::ListenerId,
        addr: &libp2p::Multiaddr,
    ) {
        delegate_to_behaviours!(self, inject_new_listen_addr, id, addr);
    }

    fn inject_expired_listen_addr(
        &mut self,
        id: libp2p::core::connection::ListenerId,
        addr: &libp2p::Multiaddr,
    ) {
        delegate_to_behaviours!(self, inject_expired_listen_addr, id, addr);
    }

    fn inject_listener_error(
        &mut self,
        id: libp2p::core::connection::ListenerId,
        err: &(dyn std::error::Error + 'static),
    ) {
        delegate_to_behaviours!(self, inject_listener_error, id, err);
    }

    fn inject_listener_closed(
        &mut self,
        id: libp2p::core::connection::ListenerId,
        reason: Result<(), &std::io::Error>,
    ) {
        delegate_to_behaviours!(self, inject_listener_closed, id, reason);
    }

    fn inject_new_external_addr(&mut self, addr: &libp2p::Multiaddr) {
        delegate_to_behaviours!(self, inject_new_external_addr, addr);
    }

    fn inject_expired_external_addr(&mut self, addr: &libp2p::Multiaddr) {
        delegate_to_behaviours!(self, inject_expired_external_addr, addr);
    }
}

impl NetworkBehaviourEventProcess<IdentifyEvent> for Behaviour {
    fn inject_event(&mut self, event: IdentifyEvent) {
        if let IdentifyEvent::Received { peer_id, info } = event {
            let peer_info = self.peer_infos.entry(peer_id).or_default();
            peer_info.protocols = info.protocols.into_iter().collect();

            if peer_info.protocols.contains(
                std::str::from_utf8(libp2p::kad::protocol::DEFAULT_PROTO_NAME).expect("oh no"),
            ) {
                for addr in info.listen_addrs.iter() {
                    self.kad.add_address(&peer_id, addr.clone());
                }
            }

            if peer_info.protocols.contains(crate::protocol::PROTOCOL_NAME) {
                for addr in info.listen_addrs.iter() {
                    if !peer_info.addrs.contains(addr) {
                        peer_info.addrs.push_back(addr.clone());
                    }
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for Behaviour {
    fn inject_event(&mut self, _event: KademliaEvent) {}
}

impl NetworkBehaviourEventProcess<IpchessEvent> for Behaviour {
    fn inject_event(&mut self, event: IpchessEvent) {
        self.actions_queue
            .push_back(NetworkBehaviourAction::GenerateEvent(
                BehaviourEvent::Ipchess(event),
            ));
    }
}
