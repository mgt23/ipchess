#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    #[prost(oneof="message::Payload", tags="1, 2, 3, 4, 5")]
    pub payload: ::core::option::Option<message::Payload>,
}
/// Nested message and enum types in `Message`.
pub mod message {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Challenge {
        #[prost(bytes="vec", tag="1")]
        pub commitment: ::prost::alloc::vec::Vec<u8>,
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ChallengeAccept {
        #[prost(bytes="vec", tag="1")]
        pub random: ::prost::alloc::vec::Vec<u8>,
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ChallengeReveal {
        #[prost(bytes="vec", tag="1")]
        pub preimage: ::prost::alloc::vec::Vec<u8>,
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ChallengeCancel {
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ChallengeDecline {
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Payload {
        #[prost(message, tag="1")]
        Challenge(Challenge),
        #[prost(message, tag="2")]
        ChallengeAccept(ChallengeAccept),
        #[prost(message, tag="3")]
        ChallengeReveal(ChallengeReveal),
        #[prost(message, tag="4")]
        ChallengeCancel(ChallengeCancel),
        #[prost(message, tag="5")]
        ChallengeDecline(ChallengeDecline),
    }
}
