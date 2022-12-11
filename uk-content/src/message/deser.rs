use std::marker::PhantomData;

use super::*;

impl Serialize for MessagePack {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (file, msyt) in self.0.iter() {
            map.serialize_entry(
                file,
                &serde_json::to_string(&msyt).map_err(serde::ser::Error::custom)?,
            )?;
        }
        map.end()
    }
}

struct MessagePackVisitor {
    marker: PhantomData<fn() -> MessagePack>,
}

impl MessagePackVisitor {
    fn new() -> Self {
        MessagePackVisitor {
            marker: PhantomData,
        }
    }
}

impl<'de> serde::de::Visitor<'de> for MessagePackVisitor {
    type Value = MessagePack;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("MessagePack")
    }

    fn visit_map<M>(self, mut access: M) -> std::result::Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut map: BTreeMap<String, Msyt> = BTreeMap::new();

        while let Some((key, value)) = access.next_entry()? {
            map.insert(
                key,
                serde_json::from_str(value).map_err(serde::de::Error::custom)?,
            );
        }

        Ok(MessagePack(map))
    }
}

impl<'de> Deserialize<'de> for MessagePack {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(MessagePackVisitor::new())
    }
}
