//! Sparkplug topic parsing and construction.
//!
//! Sparkplug B topics follow the format:
//! - `spBv1.0/{group_id}/{message_type}/{edge_node_id}[/{device_id}]`
//! - `STATE/{scada_host_id}`

use crate::error::{Error, Result};

/// Sparkplug message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    /// Node Birth - published when a node comes online
    NBirth,
    /// Node Death - published when a node goes offline
    NDeath,
    /// Node Data - published when node metrics change
    NData,
    /// Node Command - command sent to a node
    NCmd,
    /// Device Birth - published when a device comes online
    DBirth,
    /// Device Death - published when a device goes offline
    DDeath,
    /// Device Data - published when device metrics change
    DData,
    /// Device Command - command sent to a device
    DCmd,
    /// State - SCADA host application state
    State,
}

impl MessageType {
    /// Returns the string representation used in MQTT topics.
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::NBirth => "NBIRTH",
            MessageType::NDeath => "NDEATH",
            MessageType::NData => "NDATA",
            MessageType::NCmd => "NCMD",
            MessageType::DBirth => "DBIRTH",
            MessageType::DDeath => "DDEATH",
            MessageType::DData => "DDATA",
            MessageType::DCmd => "DCMD",
            MessageType::State => "STATE",
        }
    }

    /// Returns true if this is a node-level message type.
    pub fn is_node_message(&self) -> bool {
        matches!(
            self,
            MessageType::NBirth | MessageType::NDeath | MessageType::NData | MessageType::NCmd
        )
    }

    /// Returns true if this is a device-level message type.
    pub fn is_device_message(&self) -> bool {
        matches!(
            self,
            MessageType::DBirth | MessageType::DDeath | MessageType::DData | MessageType::DCmd
        )
    }

    /// Returns true if this is a birth message (NBIRTH or DBIRTH).
    pub fn is_birth(&self) -> bool {
        matches!(self, MessageType::NBirth | MessageType::DBirth)
    }

    /// Returns true if this is a death message (NDEATH or DDEATH).
    pub fn is_death(&self) -> bool {
        matches!(self, MessageType::NDeath | MessageType::DDeath)
    }

    /// Returns true if this is a data message (NDATA or DDATA).
    pub fn is_data(&self) -> bool {
        matches!(self, MessageType::NData | MessageType::DData)
    }

    /// Returns true if this is a command message (NCMD or DCMD).
    pub fn is_command(&self) -> bool {
        matches!(self, MessageType::NCmd | MessageType::DCmd)
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for MessageType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "NBIRTH" => Ok(MessageType::NBirth),
            "NDEATH" => Ok(MessageType::NDeath),
            "NDATA" => Ok(MessageType::NData),
            "NCMD" => Ok(MessageType::NCmd),
            "DBIRTH" => Ok(MessageType::DBirth),
            "DDEATH" => Ok(MessageType::DDeath),
            "DDATA" => Ok(MessageType::DData),
            "DCMD" => Ok(MessageType::DCmd),
            "STATE" => Ok(MessageType::State),
            _ => Err(Error::InvalidTopic(format!("unknown message type: {}", s))),
        }
    }
}

/// A parsed Sparkplug topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedTopic {
    /// A Sparkplug message topic.
    Sparkplug {
        /// The message type.
        message_type: MessageType,
        /// The group ID.
        group_id: String,
        /// The edge node ID.
        edge_node_id: String,
        /// The device ID (only present for device-level messages).
        device_id: Option<String>,
    },
    /// A STATE topic for SCADA host application state.
    State {
        /// The SCADA host ID.
        host_id: String,
    },
}

impl ParsedTopic {
    /// Parses a Sparkplug topic string.
    ///
    /// # Examples
    ///
    /// ```
    /// use sparkplug_rs::ParsedTopic;
    ///
    /// // Node-level message
    /// let topic = ParsedTopic::parse("spBv1.0/Energy/NDATA/Gateway01")?;
    ///
    /// // Device-level message
    /// let topic = ParsedTopic::parse("spBv1.0/Energy/DDATA/Gateway01/Sensor01")?;
    ///
    /// // State message
    /// let topic = ParsedTopic::parse("STATE/ScadaHost01")?;
    /// # Ok::<(), sparkplug_rs::Error>(())
    /// ```
    pub fn parse(topic: &str) -> Result<Self> {
        let parts: Vec<&str> = topic.split('/').collect();

        // Check for STATE topic
        if parts.len() == 2 && parts[0] == "STATE" {
            return Ok(ParsedTopic::State {
                host_id: parts[1].to_string(),
            });
        }

        // Parse Sparkplug topic: spBv1.0/{group_id}/{message_type}/{edge_node_id}[/{device_id}]
        if parts.len() < 4 {
            return Err(Error::InvalidTopic(format!(
                "topic must have at least 4 parts, got {}",
                parts.len()
            )));
        }

        if parts[0] != "spBv1.0" {
            return Err(Error::InvalidTopic(format!(
                "topic must start with 'spBv1.0', got '{}'",
                parts[0]
            )));
        }

        let group_id = parts[1].to_string();
        let message_type: MessageType = parts[2].parse()?;
        let edge_node_id = parts[3].to_string();
        let device_id = parts.get(4).map(|s| s.to_string());

        // Validate device_id presence based on message type
        if message_type.is_device_message() && device_id.is_none() {
            return Err(Error::InvalidTopic(format!(
                "{} messages require a device_id",
                message_type
            )));
        }

        if message_type.is_node_message() && device_id.is_some() {
            return Err(Error::InvalidTopic(format!(
                "{} messages should not have a device_id",
                message_type
            )));
        }

        Ok(ParsedTopic::Sparkplug {
            message_type,
            group_id,
            edge_node_id,
            device_id,
        })
    }

    /// Returns the message type, if this is a Sparkplug message.
    pub fn message_type(&self) -> Option<MessageType> {
        match self {
            ParsedTopic::Sparkplug { message_type, .. } => Some(*message_type),
            ParsedTopic::State { .. } => None,
        }
    }

    /// Returns the group ID, if this is a Sparkplug message.
    pub fn group_id(&self) -> Option<&str> {
        match self {
            ParsedTopic::Sparkplug { group_id, .. } => Some(group_id),
            ParsedTopic::State { .. } => None,
        }
    }

    /// Returns the edge node ID, if this is a Sparkplug message.
    pub fn edge_node_id(&self) -> Option<&str> {
        match self {
            ParsedTopic::Sparkplug { edge_node_id, .. } => Some(edge_node_id),
            ParsedTopic::State { .. } => None,
        }
    }

    /// Returns the device ID, if this is a device-level Sparkplug message.
    pub fn device_id(&self) -> Option<&str> {
        match self {
            ParsedTopic::Sparkplug { device_id, .. } => device_id.as_deref(),
            ParsedTopic::State { .. } => None,
        }
    }

    /// Returns the host ID, if this is a STATE message.
    pub fn host_id(&self) -> Option<&str> {
        match self {
            ParsedTopic::State { host_id } => Some(host_id),
            ParsedTopic::Sparkplug { .. } => None,
        }
    }

    /// Converts the parsed topic back to a topic string.
    pub fn to_topic_string(&self) -> String {
        match self {
            ParsedTopic::Sparkplug {
                message_type,
                group_id,
                edge_node_id,
                device_id,
            } => {
                if let Some(device_id) = device_id {
                    format!(
                        "spBv1.0/{}/{}/{}/{}",
                        group_id,
                        message_type.as_str(),
                        edge_node_id,
                        device_id
                    )
                } else {
                    format!(
                        "spBv1.0/{}/{}/{}",
                        group_id,
                        message_type.as_str(),
                        edge_node_id
                    )
                }
            }
            ParsedTopic::State { host_id } => format!("STATE/{}", host_id),
        }
    }
}

impl std::fmt::Display for ParsedTopic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_topic_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nbirth() {
        let topic = ParsedTopic::parse("spBv1.0/Energy/NBIRTH/Gateway01").unwrap();
        assert_eq!(topic.message_type(), Some(MessageType::NBirth));
        assert_eq!(topic.group_id(), Some("Energy"));
        assert_eq!(topic.edge_node_id(), Some("Gateway01"));
        assert_eq!(topic.device_id(), None);
    }

    #[test]
    fn test_parse_ddata() {
        let topic = ParsedTopic::parse("spBv1.0/Manufacturing/DDATA/Node1/Sensor01").unwrap();
        assert_eq!(topic.message_type(), Some(MessageType::DData));
        assert_eq!(topic.group_id(), Some("Manufacturing"));
        assert_eq!(topic.edge_node_id(), Some("Node1"));
        assert_eq!(topic.device_id(), Some("Sensor01"));
    }

    #[test]
    fn test_parse_state() {
        let topic = ParsedTopic::parse("STATE/ScadaHost01").unwrap();
        assert_eq!(topic.message_type(), None);
        assert_eq!(topic.host_id(), Some("ScadaHost01"));
    }

    #[test]
    fn test_invalid_prefix() {
        let result = ParsedTopic::parse("invalid/Energy/NDATA/Node1");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_device_id_for_device_message() {
        let result = ParsedTopic::parse("spBv1.0/Energy/DDATA/Node1");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_topic_string() {
        let topic = ParsedTopic::Sparkplug {
            message_type: MessageType::NData,
            group_id: "Energy".to_string(),
            edge_node_id: "Gateway01".to_string(),
            device_id: None,
        };
        assert_eq!(topic.to_topic_string(), "spBv1.0/Energy/NDATA/Gateway01");
    }
}
