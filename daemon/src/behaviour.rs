use std::collections::VecDeque;
use std::str::FromStr;
use std::task::Poll;

use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent};
use libp2p::kad::KademliaConfig;
use libp2p::kad::{store::MemoryStore, Kademlia, KademliaEvent};
use libp2p::swarm::{
    IntoProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess,
    ProtocolsHandler,
};

use libp2p::{NetworkBehaviour, PeerId};

use crate::protocol::{Ipchess, IpchessEvent};

const BOOTSTRAP_PEER_ADDRS: [&'static str; 6] = [
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
    "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
    "/ip4/104.131.131.82/udp/4001/quic/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
];

pub enum BehaviourEvent {
    MatchReady,
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourEvent")]
#[behaviour(poll_method = "poll")]
pub struct Behaviour {
    identify: Identify,
    kad: Kademlia<MemoryStore>,
    ipchess: Ipchess,

    #[behaviour(ignore)]
    challenged_peer_id: Option<PeerId>,
    #[behaviour(ignore)]
    out_events: VecDeque<
        NetworkBehaviourAction<
            <<<Self as NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::InEvent,
            BehaviourEvent,
        >,
    >,
}

impl Behaviour {
    pub fn new(peer_id: PeerId, public_key: libp2p::identity::PublicKey) -> Self {
        let kad_config = KademliaConfig::default();
        let mut kad = Kademlia::with_config(peer_id.clone(), MemoryStore::new(peer_id), kad_config);

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

            challenged_peer_id: None,
            out_events: VecDeque::new(),
        }
    }

    pub fn bootstrap(&mut self) {
        self.kad.bootstrap().unwrap();
    }

    pub fn challenge_peer(&mut self, peer_id: PeerId) {
        log::info!("Challenging peer {}", peer_id);
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

    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
        _params: &mut impl libp2p::swarm::PollParameters
    ) -> Poll<NetworkBehaviourAction<<<<Self as NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::InEvent, <Self as NetworkBehaviour>::OutEvent>>{
        // drain out events
        while let Some(out_event) = self.out_events.pop_front() {
            return Poll::Ready(out_event);
        }

        Poll::Pending
    }
}

impl NetworkBehaviourEventProcess<IdentifyEvent> for Behaviour {
    fn inject_event(&mut self, event: IdentifyEvent) {
        if let IdentifyEvent::Received { peer_id, info } = event {
            let challenged_peer_id = if let Some(challenged_peer_id) = &self.challenged_peer_id {
                *challenged_peer_id
            } else {
                return;
            };

            if peer_id != challenged_peer_id {
                return;
            }

            log::debug!(
                "Identified challenged peer {}, starting challenge request",
                peer_id
            );

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
        match event {
            IpchessEvent::PeerChallenge { peer_id } => {
                log::info!("Accepting peer challenge {}", peer_id);
                self.ipchess.accept_peer_challenge(peer_id);
            }

            IpchessEvent::MatchReady {
                peer_id,
                match_data,
            } => {
                log::info!("Match ready {} {:?}", peer_id, match_data);
                self.challenged_peer_id = None;

                self.out_events
                    .push_back(NetworkBehaviourAction::GenerateEvent(
                        BehaviourEvent::MatchReady,
                    ));
            }

            _ => {}
        }
    }
}
