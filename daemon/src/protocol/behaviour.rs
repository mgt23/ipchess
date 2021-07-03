use std::{
    collections::{HashMap, HashSet, VecDeque},
    task::Poll,
};

use libp2p::{
    core::connection::ConnectionId,
    multihash::Hasher,
    swarm::{NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, ProtocolsHandler},
    PeerId,
};
use rand::Rng;
use thiserror::Error;

use super::{IpchessHandler, IpchessHandlerEventIn, IpchessHandlerEventOut};

/// Challenge sent to a peer.
struct OutboundChallenge {
    /// Preimage of the commitment sent to the challenged peer.
    preimage: Vec<u8>,
}

/// States a challenge received from a peer is allowed to be in.
enum InboundChallenge {
    /// Challenge was received by this peer and is ready to be accepted or declined.
    Received {
        /// Commitment for the random bytes chosen by the peer.
        commitment: Vec<u8>,
    },

    /// Challenge was accepted by this peer but it has not received the pre image for the challenger's commitment yet.
    PendingPreimage {
        /// Commitment for the random bytes chosen by the peer.
        commitment: Vec<u8>,
        /// Random bytes chosen by the challenged peer.
        random: Vec<u8>,
    },
}

/// A accepted challenge containing all information about the match's negotiation.
#[derive(Debug)]
pub struct AcceptedChallenge {
    /// Random bytes chosen by the challenger peer.
    preimage: Vec<u8>,
    /// Random bytes chosen by the challenged peer.
    random: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum IpchessError {
    #[error("Preimage revealed by peer does not match previously sent commitment")]
    ChallengeCommitmentPreimageMismatch,
    #[error("Peer did not follow the protocol")]
    ChallengePoisoned,
}

#[derive(Debug)]
pub enum IpchessEvent {
    PeerChallenge {
        peer_id: PeerId,
    },

    ChallengeAccepted {
        peer_id: PeerId,
        challenge: AcceptedChallenge,
    },

    ChallengeDeclined {
        peer_id: PeerId,
    },

    ChallengeCanceled {
        peer_id: PeerId,
    },

    Error(IpchessError),
}

pub struct Ipchess {
    actions_queue: VecDeque<NetworkBehaviourAction<IpchessHandlerEventIn, IpchessEvent>>,
    pending_handler_in: HashMap<PeerId, VecDeque<IpchessHandlerEventIn>>,

    outbound_challenges: HashMap<PeerId, OutboundChallenge>,
    inbound_challenges: HashMap<PeerId, InboundChallenge>,

    connected_peers: HashSet<PeerId>,
}

impl Ipchess {
    pub fn new() -> Self {
        Ipchess {
            actions_queue: VecDeque::new(),
            pending_handler_in: HashMap::new(),

            outbound_challenges: HashMap::new(),
            inbound_challenges: HashMap::new(),

            connected_peers: HashSet::new(),
        }
    }

    pub fn challenge_peer(&mut self, peer_id: PeerId) {
        if self.outbound_challenges.contains_key(&peer_id) {
            log::debug!("Duplicate outbound challenge to peer {}, ignoring", peer_id);
            return;
        }

        let mut thread_rng = rand::thread_rng();
        let preimage = thread_rng.gen::<[u8; 32]>().to_vec();

        let commitment = libp2p::multihash::Sha2_256::digest(&preimage)
            .as_ref()
            .to_vec();

        self.outbound_challenges
            .insert(peer_id, OutboundChallenge { preimage });

        self.actions_queue
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id: peer_id,
                handler: NotifyHandler::Any,
                event: IpchessHandlerEventIn::Challenge { commitment },
            });
    }

    pub fn accept_peer_challenge(&mut self, peer_id: PeerId) {
        let challenge_data = match self.inbound_challenges.remove(&peer_id) {
            Some(challenge_data) => challenge_data,
            None => {
                log::warn!(
                    "Ignoring accept_peer_challenge, there are no inbound challenges from peer {}",
                    peer_id
                );
                return;
            }
        };

        let updated_challenge_data = match challenge_data {
            InboundChallenge::Received { commitment } => {
                let mut thread_rng = rand::thread_rng();
                let random = thread_rng.gen::<[u8; 32]>().to_vec();

                self.notify_handler_checked(
                    peer_id,
                    IpchessHandlerEventIn::ChallengeAccept {
                        random: random.clone(),
                    },
                );

                InboundChallenge::PendingPreimage { commitment, random }
            }

            InboundChallenge::PendingPreimage { .. } => {
                log::warn!(
                    "Ignoring accept_peer_challenge for peer {}, challenge was already accepted and is pending the receipt of the preimage",
                    peer_id
                );

                challenge_data
            }
        };

        self.inbound_challenges
            .insert(peer_id, updated_challenge_data);
    }

    pub fn cancel_challenge(&mut self, peer_id: PeerId) {
        if self.outbound_challenges.remove(&peer_id).is_some() {
            self.notify_handler_checked(peer_id, IpchessHandlerEventIn::ChallengeCanceled);
        } else {
            log::debug!(
                "Ignoring cancel_challenge, no challenge for peer {}",
                peer_id
            );
        }
    }

    pub fn decline_peer_challenge(&mut self, peer_id: PeerId) {
        if self.inbound_challenges.remove(&peer_id).is_some() {
            self.notify_handler_checked(peer_id, IpchessHandlerEventIn::ChallengeDeclined);
        } else {
            log::debug!(
                "Ignoring decline_peer_challenge, no challenge from peer {}",
                peer_id
            );
        }
    }

    fn notify_handler_checked(&mut self, peer_id: PeerId, handler_in: IpchessHandlerEventIn) {
        if self.connected_peers.contains(&peer_id) {
            self.actions_queue
                .push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler: NotifyHandler::Any,
                    event: handler_in,
                });
        } else {
            self.actions_queue
                .push_back(NetworkBehaviourAction::DialPeer {
                    peer_id,
                    condition: libp2p::swarm::DialPeerCondition::Disconnected,
                });
            self.pending_handler_in
                .entry(peer_id)
                .or_default()
                .push_back(handler_in);
        }
    }
}

impl NetworkBehaviour for Ipchess {
    type ProtocolsHandler = IpchessHandler;
    type OutEvent = IpchessEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        IpchessHandler::new()
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<libp2p::Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, _peer_id: &PeerId) {}

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        _: &libp2p::core::ConnectedPoint,
    ) {
        self.connected_peers.insert(*peer_id);

        if let Some(pending_handler_in_events) = self.pending_handler_in.remove(peer_id) {
            for pending_handler_in in pending_handler_in_events {
                self.actions_queue
                    .push_back(NetworkBehaviourAction::NotifyHandler {
                        peer_id: *peer_id,
                        handler: NotifyHandler::Any,
                        event: pending_handler_in,
                    });
            }
        }
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        self.connected_peers.remove(peer_id);
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        conn_id: ConnectionId,
        event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent,
    ) {
        match event {
            IpchessHandlerEventOut::ChallengeReceived { commitment } => {
                self.inbound_challenges
                    .insert(peer_id, InboundChallenge::Received { commitment });

                self.actions_queue
                    .push_back(NetworkBehaviourAction::GenerateEvent(
                        IpchessEvent::PeerChallenge { peer_id },
                    ));
            }

            IpchessHandlerEventOut::ChallengeRevealReceived { preimage } => {
                if let Some(inbound_challenge) = self.inbound_challenges.remove(&peer_id) {
                    match inbound_challenge {
                        InboundChallenge::PendingPreimage {
                            commitment, random, ..
                        } => {
                            let preimage_hash = libp2p::multihash::Sha2_256::digest(&preimage);

                            if preimage_hash.as_ref().to_vec() == commitment {
                                self.actions_queue.push_back(
                                    NetworkBehaviourAction::GenerateEvent(
                                        IpchessEvent::ChallengeAccepted {
                                            peer_id,
                                            challenge: AcceptedChallenge { preimage, random },
                                        },
                                    ),
                                );
                            } else {
                                self.actions_queue.push_back(
                                    NetworkBehaviourAction::NotifyHandler {
                                        peer_id,
                                        handler: NotifyHandler::One(conn_id),
                                        event: IpchessHandlerEventIn::ChallengePoisoned,
                                    },
                                );

                                self.actions_queue.push_back(
                                    NetworkBehaviourAction::GenerateEvent(IpchessEvent::Error(
                                        IpchessError::ChallengeCommitmentPreimageMismatch,
                                    )),
                                );
                            }
                        }

                        InboundChallenge::Received { .. } => {
                            self.actions_queue
                                .push_back(NetworkBehaviourAction::NotifyHandler {
                                    peer_id,
                                    handler: NotifyHandler::One(conn_id),
                                    event: IpchessHandlerEventIn::ChallengePoisoned,
                                });

                            self.actions_queue
                                .push_back(NetworkBehaviourAction::GenerateEvent(
                                    IpchessEvent::Error(IpchessError::ChallengePoisoned),
                                ));
                        }
                    };
                }
            }

            IpchessHandlerEventOut::ChallengeAccepted { random } => {
                if let Some(sent_challenge) = self.outbound_challenges.remove(&peer_id) {
                    self.actions_queue
                        .push_back(NetworkBehaviourAction::NotifyHandler {
                            peer_id,
                            handler: NotifyHandler::One(conn_id),
                            event: IpchessHandlerEventIn::ChallengeReveal {
                                preimage: sent_challenge.preimage.clone(),
                            },
                        });

                    self.actions_queue
                        .push_back(NetworkBehaviourAction::GenerateEvent(
                            IpchessEvent::ChallengeAccepted {
                                peer_id,
                                challenge: AcceptedChallenge {
                                    preimage: sent_challenge.preimage,
                                    random,
                                },
                            },
                        ));
                }
            }

            IpchessHandlerEventOut::ChallengeCanceled => {
                if self.inbound_challenges.remove(&peer_id).is_some() {
                    self.actions_queue
                        .push_back(NetworkBehaviourAction::GenerateEvent(
                            IpchessEvent::ChallengeCanceled { peer_id },
                        ));
                }
            }

            IpchessHandlerEventOut::ChallengeDeclined => {
                if self.outbound_challenges.remove(&peer_id).is_some() {
                    self.actions_queue
                        .push_back(NetworkBehaviourAction::GenerateEvent(
                            IpchessEvent::ChallengeDeclined { peer_id },
                        ));
                }
            }
        }
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
        // drain pending network behaviour actions
        if let Some(action) = self.actions_queue.pop_front() {
            return Poll::Ready(action);
        }

        Poll::Pending
    }
}
