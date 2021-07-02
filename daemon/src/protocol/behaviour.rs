use std::{
    collections::{HashMap, HashSet, VecDeque},
    task::Poll,
    time::{Duration, Instant},
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

/// Challenge sent to a peer.
struct OutboundChallenge {
    /// Preimage of the commitment sent to the challenged peer.
    preimage: Vec<u8>,
    /// Instant the challenge was sent to the peer.
    timestamp: Instant,
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
        /// Instant the random bytes were sent to the challenger.
        timestamp: Instant,
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

#[derive(Debug)]
pub enum ChallengeDirection {
    Inbound,
    Outbound,
}

#[derive(Error, Debug)]
pub enum IpchessError {
    #[error("Preimage revealed by peer does not match previously sent commitment")]
    ChallengeCommitmentPreimageMismatch,
    #[error("Challenge timed out")]
    ChallengeTimeout {
        peer_id: PeerId,
        direction: ChallengeDirection,
    },
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

/// Behaviour configuration.
pub struct IpchessConfig {
    /// Amount of time a peer has until an outbound challenge is considered timed out.
    challenge_accept_timeout: Duration,
    /// Amount of time a peer to sent back the challenge's commitment preimage.
    challenge_preimage_timeout: Duration,
}

impl Default for IpchessConfig {
    fn default() -> Self {
        Self {
            challenge_accept_timeout: Duration::from_secs(5 * 60),
            challenge_preimage_timeout: Duration::from_secs(15),
        }
    }
}

pub struct Ipchess {
    config: IpchessConfig,

    events: VecDeque<NetworkBehaviourAction<IpchessHandlerEventIn, IpchessEvent>>,

    outbound_challenges: HashMap<PeerId, OutboundChallenge>,
    inbound_challenges: HashMap<PeerId, InboundChallenge>,

    peer_addresses: HashMap<PeerId, HashSet<Multiaddr>>,
}

impl Ipchess {
    pub fn new() -> Self {
        Ipchess {
            config: IpchessConfig::default(),

            events: VecDeque::new(),

            outbound_challenges: HashMap::new(),
            inbound_challenges: HashMap::new(),

            peer_addresses: HashMap::new(),
        }
    }

    pub fn add_address(&mut self, peer_id: PeerId, addr: Multiaddr) {
        self.peer_addresses.entry(peer_id).or_default().insert(addr);
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

        self.outbound_challenges.insert(
            peer_id,
            OutboundChallenge {
                preimage,
                // timestamp is set to now but this could be changed to be set to the
                // instant at which the handler sent the challenge through the network.
                timestamp: Instant::now(),
            },
        );

        self.events
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

                self.events
                    .push_back(NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler: NotifyHandler::Any,
                        event: IpchessHandlerEventIn::ChallengeAccept {
                            random: random.clone(),
                        },
                    });

                InboundChallenge::PendingPreimage {
                    commitment,
                    random,
                    timestamp: Instant::now(),
                }
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
            self.events
                .push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler: NotifyHandler::Any,
                    event: IpchessHandlerEventIn::ChallengeCanceled,
                });
        } else {
            log::debug!(
                "Ignoring cancel_challenge, no challenge for peer {}",
                peer_id
            );
        }
    }

    pub fn decline_peer_challenge(&mut self, peer_id: PeerId) {
        if self.inbound_challenges.remove(&peer_id).is_some() {
            self.events
                .push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler: NotifyHandler::Any,
                    event: IpchessHandlerEventIn::ChallengeDeclined,
                });
        } else {
            log::debug!(
                "Ignoring decline_peer_challenge, no challenge from peer {}",
                peer_id
            );
        }
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
            .map_or(vec![], |addrs| addrs.clone().into_iter().collect())
    }

    fn inject_connected(&mut self, _peer_id: &PeerId) {}

    fn inject_disconnected(&mut self, _peer_id: &PeerId) {}

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        _conn_id: ConnectionId,
        event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent,
    ) {
        match event {
            IpchessHandlerEventOut::ChallengeReceived { commitment } => {
                self.inbound_challenges
                    .insert(peer_id, InboundChallenge::Received { commitment });

                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
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
                                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                                    IpchessEvent::ChallengeAccepted {
                                        peer_id,
                                        challenge: AcceptedChallenge { preimage, random },
                                    },
                                ));
                            } else {
                                self.events
                                    .push_back(NetworkBehaviourAction::NotifyHandler {
                                        peer_id,
                                        handler: NotifyHandler::Any,
                                        event: IpchessHandlerEventIn::ChallengePoisoned,
                                    });

                                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                                    IpchessEvent::Error(
                                        IpchessError::ChallengeCommitmentPreimageMismatch,
                                    ),
                                ));
                            }
                        }

                        InboundChallenge::Received { .. } => {
                            self.events
                                .push_back(NetworkBehaviourAction::NotifyHandler {
                                    peer_id,
                                    handler: NotifyHandler::Any,
                                    event: IpchessHandlerEventIn::ChallengePoisoned,
                                });
                        }
                    };
                }
            }

            IpchessHandlerEventOut::ChallengeAccepted { random } => {
                if let Some(sent_challenge) = self.outbound_challenges.remove(&peer_id) {
                    self.events
                        .push_back(NetworkBehaviourAction::NotifyHandler {
                            peer_id,
                            handler: NotifyHandler::Any,
                            event: IpchessHandlerEventIn::ChallengeReveal {
                                preimage: sent_challenge.preimage.clone(),
                            },
                        });

                    self.events.push_back(NetworkBehaviourAction::GenerateEvent(
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
                    self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                        IpchessEvent::ChallengeCanceled { peer_id },
                    ));
                }
            }

            IpchessHandlerEventOut::ChallengeDeclined => {
                if self.outbound_challenges.remove(&peer_id).is_some() {
                    self.events.push_back(NetworkBehaviourAction::GenerateEvent(
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
        // drain pending events
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        // clear timed out outbound challenge
        let now = Instant::now();

        let timedout_outbound_challenge_keys: Vec<_> = self
            .outbound_challenges
            .iter()
            .filter_map(|(peer_id, challenge)| {
                if now.duration_since(challenge.timestamp) > self.config.challenge_accept_timeout {
                    Some(*peer_id)
                } else {
                    None
                }
            })
            .collect();

        for peer_id in timedout_outbound_challenge_keys {
            self.outbound_challenges.remove(&peer_id);
            self.events
                .push_back(NetworkBehaviourAction::GenerateEvent(IpchessEvent::Error(
                    IpchessError::ChallengeTimeout {
                        peer_id,
                        direction: ChallengeDirection::Outbound,
                    },
                )));
        }

        // clear timed out inbound challenges
        let timedout_inbound_challenge_keys: Vec<_> = self
            .inbound_challenges
            .iter()
            .filter_map(|(peer_id, challenge)| match challenge {
                InboundChallenge::Received { .. } => None,
                InboundChallenge::PendingPreimage { timestamp, .. } => {
                    if now.duration_since(*timestamp) > self.config.challenge_preimage_timeout {
                        Some(*peer_id)
                    } else {
                        None
                    }
                }
            })
            .collect();

        for peer_id in timedout_inbound_challenge_keys {
            self.inbound_challenges.remove(&peer_id);
            self.events
                .push_back(NetworkBehaviourAction::GenerateEvent(IpchessEvent::Error(
                    IpchessError::ChallengeTimeout {
                        peer_id,
                        direction: ChallengeDirection::Inbound,
                    },
                )));
        }

        Poll::Pending
    }
}
