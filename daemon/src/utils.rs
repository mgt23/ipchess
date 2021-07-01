use serde::Serialize;

pub struct SerializablePeerId(pub libp2p::PeerId);

impl Serialize for SerializablePeerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
}
