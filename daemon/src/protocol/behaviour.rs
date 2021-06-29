use std::{
    collections::{HashMap, VecDeque},
    task::Poll,
};

use libp2p::{
    core::connection::ConnectionId,
    multihash::Hasher,
    swarm::{NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, ProtocolsHandler},
    Multiaddr, PeerId,
};
use rand::Rng;
use thiserror::Error;

use super::{IpchessHandler, IpchessHandlerEventIn, IpchessHandlerEventOut};

struct PendingChallenge {
    commitment: Vec<u8>,
    random: Option<Vec<u8>>,
    conn_id: ConnectionId,
}

struct SentChallenge {
    preimage: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MatchData {
    preimage: Vec<u8>,
    random: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum IpchessError {
    #[error("Preimage revealed by peer does not match previously sent commitment")]
    CommitmentPreimageMismatch,
}

#[derive(Debug)]
pub enum IpchessEvent {
    PeerChallenge {
        peer_id: PeerId,
    },

    MatchReady {
        peer_id: PeerId,
        match_data: MatchData,
    },

    Error(IpchessError),
}

pub struct Ipchess {
    handler_in: VecDeque<(PeerId, Option<ConnectionId>, IpchessHandlerEventIn)>,
    handler_out: VecDeque<(PeerId, ConnectionId, IpchessHandlerEventOut)>,

    pending_challenges: HashMap<PeerId, PendingChallenge>,
    sent_challenges: HashMap<PeerId, SentChallenge>,

    peer_addresses: HashMap<PeerId, Vec<Multiaddr>>,
}

impl Ipchess {
    pub fn new() -> Self {
        Ipchess {
            handler_in: VecDeque::new(),
            handler_out: VecDeque::new(),
            pending_challenges: HashMap::new(),
            sent_challenges: HashMap::new(),
            peer_addresses: HashMap::new(),
        }
    }

    pub fn add_address(&mut self, peer_id: PeerId, addr: Multiaddr) {
        let addrs = self.peer_addresses.entry(peer_id).or_default();
        addrs.push(addr);
    }

    pub fn challenge_peer(&mut self, peer_id: PeerId) {
        let mut thread_rng = rand::thread_rng();
        let preimage = thread_rng.gen::<[u8; 32]>().to_vec();

        let commitment = libp2p::multihash::Sha2_256::digest(&preimage)
            .as_ref()
            .to_vec();

        self.sent_challenges
            .insert(peer_id, SentChallenge { preimage });

        self.handler_in.push_back((
            peer_id,
            None,
            IpchessHandlerEventIn::Challenge { commitment },
        ));
    }

    pub fn accept_peer_challenge(&mut self, peer_id: PeerId) {
        let challenge_data = self
            .pending_challenges
            .get_mut(&peer_id)
            .expect("no pending challenge from peer");

        let mut thread_rng = rand::thread_rng();
        let random = thread_rng.gen::<[u8; 32]>().to_vec();

        challenge_data.random = Some(random.clone());

        self.handler_in.push_back((
            peer_id,
            Some(challenge_data.conn_id),
            IpchessHandlerEventIn::ChallengeAccept { random },
        ))
    }
}

impl NetworkBehaviour for Ipchess {
    type ProtocolsHandler = IpchessHandler;
    type OutEvent = IpchessEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        IpchessHandler::new()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<libp2p::Multiaddr> {
        self.peer_addresses
            .get(peer_id)
            .map_or(vec![], |addrs| addrs.clone())
    }

    fn inject_connected(&mut self, _peer_id: &PeerId) {}

    fn inject_disconnected(&mut self, _peer_id: &PeerId) {}

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent,
    ) {
        self.handler_out.push_back((peer_id, connection, event));
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
        // drain handler out events list
        while let Some((peer_id, conn_id, event)) = self.handler_out.pop_front() {
            match event {
                IpchessHandlerEventOut::ChallengeReceived { commitment } => {
                    self.pending_challenges.insert(
                        peer_id.clone(),
                        PendingChallenge {
                            commitment,
                            conn_id,
                            random: None,
                        },
                    );

                    return Poll::Ready(NetworkBehaviourAction::GenerateEvent(
                        IpchessEvent::PeerChallenge { peer_id },
                    ));
                }

                IpchessHandlerEventOut::ChallengeRevealReceived { preimage } => {
                    if let Some(pending_challenge) = self.pending_challenges.remove(&peer_id) {
                        let random = match pending_challenge.random {
                            Some(random) => random,

                            None => {
                                return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                                    peer_id,
                                    handler: NotifyHandler::One(conn_id),
                                    event: IpchessHandlerEventIn::ChallengePoisoned,
                                })
                            }
                        };

                        let preimage_hash = libp2p::multihash::Sha2_256::digest(&preimage);

                        if preimage_hash.as_ref().to_vec() == pending_challenge.commitment {
                            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(
                                IpchessEvent::MatchReady {
                                    peer_id,
                                    match_data: MatchData { preimage, random },
                                },
                            ));
                        } else {
                            self.handler_in.push_back((
                                peer_id.clone(),
                                Some(conn_id),
                                IpchessHandlerEventIn::ChallengePoisoned,
                            ));

                            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(
                                IpchessEvent::Error(IpchessError::CommitmentPreimageMismatch),
                            ));
                        }
                    }
                }

                IpchessHandlerEventOut::ChallengeAccepted { random } => {
                    if let Some(sent_challenge) = self.sent_challenges.remove(&peer_id) {
                        self.handler_in.push_back((
                            peer_id.clone(),
                            Some(conn_id),
                            IpchessHandlerEventIn::ChallengeReveal {
                                preimage: sent_challenge.preimage.clone(),
                            },
                        ));

                        return Poll::Ready(NetworkBehaviourAction::GenerateEvent(
                            IpchessEvent::MatchReady {
                                peer_id,
                                match_data: MatchData {
                                    preimage: sent_challenge.preimage,
                                    random,
                                },
                            },
                        ));
                    }
                }
            }
        }

        // drain handler in events list
        while let Some((peer_id, conn_id, event)) = self.handler_in.pop_front() {
            let handler = match conn_id {
                Some(conn_id) => NotifyHandler::One(conn_id),
                None => NotifyHandler::Any,
            };

            return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                peer_id,
                handler,
                event,
            });
        }

        Poll::Pending
    }
}
