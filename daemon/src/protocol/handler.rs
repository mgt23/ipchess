use core::iter;
use std::{ops::Add, task::Poll, time};

use futures::{
    future::{self, BoxFuture},
    AsyncReadExt, AsyncWriteExt, FutureExt,
};
use libp2p::swarm::{
    protocols_handler::{InboundUpgradeSend, OutboundUpgradeSend, UpgradeInfoSend},
    KeepAlive, NegotiatedSubstream, ProtocolsHandler, ProtocolsHandlerEvent, SubstreamProtocol,
};
use prost::Message;
use thiserror::Error;

use super::ipchessproto;

#[derive(Debug)]
pub enum IpchessHandlerEventIn {
    Challenge { commitment: Vec<u8> },
    ChallengeAccept { random: Vec<u8> },
    ChallengeReveal { preimage: Vec<u8> },
    ChallengePoisoned,
}

#[derive(Debug)]
pub enum IpchessHandlerEventOut {
    ChallengeReceived { commitment: Vec<u8> },
    ChallengeRevealReceived { preimage: Vec<u8> },
    ChallengeAccepted { random: Vec<u8> },
}

#[derive(Error, Debug)]
pub enum IpchessHandlerError {
    #[error("failed encoding protobuf message, reason: `{0}`")]
    ProtobufEncode(prost::EncodeError),
    #[error("failed decoding protobuf message, reason: `{0}`")]
    ProtobufDecode(prost::DecodeError),

    #[error("failed writing `{0}`, reason: `{1}`")]
    SubstreamWrite(&'static str, std::io::Error),
    #[error("failed reading `{0}, reason: `{1}`")]
    SubstreamRead(&'static str, std::io::Error),
    #[error("failed flusing substream, reason: `{0}`")]
    SubstreamFlush(std::io::Error),

    #[error("poisoned")]
    Poisoned,
}

pub struct IpchessProtocol {}

impl UpgradeInfoSend for IpchessProtocol {
    type Info = &'static str;
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once("/ipchess/1.0.0")
    }
}

impl InboundUpgradeSend for IpchessProtocol {
    type Output = NegotiatedSubstream;
    type Error = IpchessHandlerError;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, socket: NegotiatedSubstream, _info: Self::Info) -> Self::Future {
        future::ok(socket)
    }
}

impl OutboundUpgradeSend for IpchessProtocol {
    type Output = NegotiatedSubstream;
    type Error = IpchessHandlerError;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, socket: NegotiatedSubstream, _info: Self::Info) -> Self::Future {
        future::ok(socket)
    }
}

enum SubstreamState {
    PendingOpen(ipchessproto::Message),
    PendingSend(BoxFuture<'static, Result<(), IpchessHandlerError>>),
    WaitingMessage(BoxFuture<'static, Result<ipchessproto::Message, IpchessHandlerError>>),
}

pub struct IpchessHandler {
    substream_states: Vec<SubstreamState>,
    handler_error_received: bool,
    keep_alive: KeepAlive,
}

impl IpchessHandler {
    pub fn new() -> Self {
        IpchessHandler {
            substream_states: vec![],
            handler_error_received: false,
            keep_alive: KeepAlive::Yes,
        }
    }
}

impl ProtocolsHandler for IpchessHandler {
    type InEvent = IpchessHandlerEventIn;
    type OutEvent = IpchessHandlerEventOut;

    type Error = IpchessHandlerError;

    type InboundProtocol = IpchessProtocol;
    type OutboundProtocol = IpchessProtocol;

    type InboundOpenInfo = ();
    type OutboundOpenInfo = ipchessproto::Message;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(IpchessProtocol {}, ())
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: <Self::InboundProtocol as InboundUpgradeSend>::Output,
        _info: Self::InboundOpenInfo,
    ) {
        log::debug!("Ipchess inbound negotiated");

        self.substream_states.push(SubstreamState::WaitingMessage(
            read_message(protocol).boxed(),
        ));
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        protocol: <Self::OutboundProtocol as OutboundUpgradeSend>::Output,
        msg: Self::OutboundOpenInfo,
    ) {
        log::debug!("Ipchess outbound negotiated");

        self.substream_states.push(SubstreamState::PendingSend(
            send_message(protocol, msg).boxed(),
        ));
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        match event {
            IpchessHandlerEventIn::Challenge { commitment } => {
                log::debug!("Initiating peer challenge");

                self.substream_states
                    .push(SubstreamState::PendingOpen(ipchessproto::Message {
                        payload: Some(ipchessproto::message::Payload::Challenge(
                            ipchessproto::message::Challenge { commitment },
                        )),
                    }));
            }

            IpchessHandlerEventIn::ChallengeAccept { random } => {
                log::debug!("Accepting peer challenge");

                self.substream_states
                    .push(SubstreamState::PendingOpen(ipchessproto::Message {
                        payload: Some(ipchessproto::message::Payload::ChallengeAccept(
                            ipchessproto::message::ChallengeAccept { random },
                        )),
                    }));
            }

            IpchessHandlerEventIn::ChallengeReveal { preimage } => {
                log::debug!("Revealing challenge commitment preimage");

                self.substream_states
                    .push(SubstreamState::PendingOpen(ipchessproto::Message {
                        payload: Some(ipchessproto::message::Payload::ChallengeReveal(
                            ipchessproto::message::ChallengeReveal { preimage },
                        )),
                    }));
            }

            IpchessHandlerEventIn::ChallengePoisoned => {
                self.handler_error_received = true;
            }
        }
    }

    fn inject_dial_upgrade_error(
        &mut self,
        _info: Self::OutboundOpenInfo,
        error: libp2p::swarm::ProtocolsHandlerUpgrErr<
            <Self::OutboundProtocol as OutboundUpgradeSend>::Error,
        >,
    ) {
        log::debug!("dial upgrade error: {:?}", error);
        self.keep_alive = KeepAlive::No;
    }

    fn connection_keep_alive(&self) -> libp2p::swarm::KeepAlive {
        self.keep_alive
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        ProtocolsHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
            Self::Error,
        >,
    > {
        if self.handler_error_received {
            return Poll::Ready(ProtocolsHandlerEvent::Close(IpchessHandlerError::Poisoned));
        }

        if self.substream_states.is_empty() {
            return Poll::Pending;
        }

        for n in (0..self.substream_states.len()).rev() {
            let state = self.substream_states.swap_remove(n);

            let next_state = match state {
                SubstreamState::PendingOpen(msg_to_send) => {
                    return Poll::Ready(ProtocolsHandlerEvent::OutboundSubstreamRequest {
                        protocol: SubstreamProtocol::new(IpchessProtocol {}, msg_to_send),
                    });
                }

                SubstreamState::PendingSend(mut fut) => match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(_)) => None,

                    Poll::Ready(Err(err)) => return Poll::Ready(ProtocolsHandlerEvent::Close(err)),

                    Poll::Pending => Some(SubstreamState::PendingSend(fut)),
                },

                SubstreamState::WaitingMessage(mut fut) => match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(msg)) => match msg.payload {
                        Some(payload) => match payload {
                            ipchessproto::message::Payload::Challenge(
                                ipchessproto::message::Challenge { commitment },
                            ) => {
                                return Poll::Ready(ProtocolsHandlerEvent::Custom(
                                    IpchessHandlerEventOut::ChallengeReceived { commitment },
                                ))
                            }

                            ipchessproto::message::Payload::ChallengeAccept(
                                ipchessproto::message::ChallengeAccept { random },
                            ) => {
                                return Poll::Ready(ProtocolsHandlerEvent::Custom(
                                    IpchessHandlerEventOut::ChallengeAccepted { random },
                                ))
                            }

                            ipchessproto::message::Payload::ChallengeReveal(
                                ipchessproto::message::ChallengeReveal { preimage },
                            ) => {
                                return Poll::Ready(ProtocolsHandlerEvent::Custom(
                                    IpchessHandlerEventOut::ChallengeRevealReceived { preimage },
                                ));
                            }
                        },

                        None => {
                            log::debug!("Ignoring message without payload");
                            None
                        }
                    },

                    Poll::Ready(Err(err)) => return Poll::Ready(ProtocolsHandlerEvent::Close(err)),

                    Poll::Pending => Some(SubstreamState::WaitingMessage(fut)),
                },
            };

            if let Some(next_state) = next_state {
                self.substream_states.push(next_state);
            }
        }

        // We have processed all substreams
        if self.substream_states.is_empty() {
            self.keep_alive =
                KeepAlive::Until(time::Instant::now().add(time::Duration::from_secs(30)));
        } else {
            self.keep_alive = KeepAlive::Yes;
        }

        Poll::Pending
    }
}

async fn read_message(
    mut stream: NegotiatedSubstream,
) -> Result<ipchessproto::Message, IpchessHandlerError> {
    let mut msg_len_buf = [0u8, 2];

    stream
        .read_exact(&mut msg_len_buf)
        .await
        .map_err(|err| IpchessHandlerError::SubstreamRead("message length", err))?;

    let msg_len = u16::from_be_bytes(msg_len_buf);
    let mut msg_buf = vec![0; msg_len as usize];

    stream
        .read_exact(&mut msg_buf)
        .await
        .map_err(|err| IpchessHandlerError::SubstreamRead("message content", err))?;

    let msg = ipchessproto::Message::decode(std::io::Cursor::new(msg_buf))
        .map_err(IpchessHandlerError::ProtobufDecode)?;

    match msg.payload {
        Some(ipchessproto::message::Payload::Challenge(_)) => {
            log::debug!("Read Challenge message")
        }
        Some(ipchessproto::message::Payload::ChallengeAccept(_)) => {
            log::debug!("Read ChallengeAccept message")
        }
        Some(ipchessproto::message::Payload::ChallengeReveal(_)) => {
            log::debug!("Read ChallengeReveal message")
        }

        None => log::debug!("Read empty message"),
    }

    Ok(msg)
}

async fn send_message(
    mut stream: NegotiatedSubstream,
    msg: ipchessproto::Message,
) -> Result<(), IpchessHandlerError> {
    match msg.payload {
        Some(ipchessproto::message::Payload::Challenge(_)) => {
            log::debug!("Sending Challenge message")
        }
        Some(ipchessproto::message::Payload::ChallengeAccept(_)) => {
            log::debug!("Sending ChallengeAccept message")
        }
        Some(ipchessproto::message::Payload::ChallengeReveal(_)) => {
            log::debug!("Sending ChallengeReveal message")
        }

        None => log::warn!("Sending empty message"),
    }

    let msg_len = msg.encoded_len();
    let mut buf = Vec::with_capacity(msg_len);
    msg.encode(&mut buf)
        .map_err(IpchessHandlerError::ProtobufEncode)?;

    stream
        .write_all(&(msg_len as u16).to_be_bytes())
        .await
        .map_err(|err| IpchessHandlerError::SubstreamWrite("message length", err))?;

    stream
        .write_all(&buf)
        .await
        .map_err(|err| IpchessHandlerError::SubstreamWrite("message content", err))?;

    stream
        .flush()
        .await
        .map_err(IpchessHandlerError::SubstreamFlush)?;

    Ok(())
}
