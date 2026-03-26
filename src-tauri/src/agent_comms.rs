use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::error::ReasonanceError;

/// Channel type for inter-agent communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ChannelType {
    /// Node-to-node direct messaging.
    Direct { target_id: String },
    /// Broadcast to all nodes in a workflow.
    Broadcast { workflow_id: String },
    /// Named topic channel.
    Topic { name: String },
}

/// Message envelope for agent communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from: String,
    pub channel: ChannelType,
    pub payload: serde_json::Value,
    pub timestamp: String,
    pub reply_to: Option<String>,
    pub ttl_secs: Option<u64>,
}

/// Channel-based communication bus for HIVE workflow agents.
///
/// Supports direct, broadcast, and topic channels with per-channel
/// backpressure (oldest messages dropped when limit reached) and
/// TTL-based expiry.
pub struct AgentCommsBus {
    /// Per-channel message queues: channel_key → VecDeque<AgentMessage>
    channels: Mutex<HashMap<String, VecDeque<AgentMessage>>>,
    /// Max messages per channel before dropping oldest.
    max_per_channel: usize,
}

impl AgentCommsBus {
    pub fn new(max_per_channel: usize) -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
            max_per_channel,
        }
    }

    /// Publish a message to a channel.
    pub fn publish(&self, message: AgentMessage) -> Result<(), ReasonanceError> {
        let key = Self::channel_key(&message.channel);
        debug!(
            "AgentComms: publish from={} to channel={}",
            message.from, key
        );

        let mut channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        let queue = channels.entry(key).or_insert_with(VecDeque::new);

        // Backpressure: drop oldest when at capacity
        if queue.len() >= self.max_per_channel {
            let dropped = queue.pop_front();
            if let Some(d) = dropped {
                warn!(
                    "AgentComms: backpressure drop on channel, oldest msg id={}",
                    d.id
                );
            }
        }

        queue.push_back(message);
        Ok(())
    }

    /// Get messages for a node (checks its direct channel).
    /// If `since_id` is provided, only messages after that ID are returned.
    pub fn get_messages(&self, node_id: &str, since_id: Option<&str>) -> Vec<AgentMessage> {
        let key = format!("direct:{}", node_id);
        let channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());

        Self::filter_messages(channels.get(&key), since_id)
    }

    /// Get all messages on a topic.
    /// If `since_id` is provided, only messages after that ID are returned.
    pub fn get_topic_messages(&self, topic: &str, since_id: Option<&str>) -> Vec<AgentMessage> {
        let key = format!("topic:{}", topic);
        let channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());

        Self::filter_messages(channels.get(&key), since_id)
    }

    /// Get all messages on a broadcast channel.
    pub fn get_broadcast_messages(
        &self,
        workflow_id: &str,
        since_id: Option<&str>,
    ) -> Vec<AgentMessage> {
        let key = format!("broadcast:{}", workflow_id);
        let channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());

        Self::filter_messages(channels.get(&key), since_id)
    }

    /// Clean expired messages (TTL). Returns count of removed messages.
    pub fn sweep_expired(&self) -> usize {
        let now = chrono::Utc::now();
        let mut total_removed = 0;

        let mut channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        for queue in channels.values_mut() {
            let before = queue.len();
            queue.retain(|msg| {
                if let Some(ttl) = msg.ttl_secs {
                    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&msg.timestamp) {
                        let expiry = ts + chrono::Duration::seconds(ttl as i64);
                        return now < expiry;
                    }
                }
                true // no TTL or unparseable timestamp → keep
            });
            total_removed += before - queue.len();
        }

        if total_removed > 0 {
            info!("AgentComms: swept {} expired messages", total_removed);
        }
        total_removed
    }

    /// Clear all channels for a workflow (broadcast + any channel containing the workflow_id).
    pub fn clear_workflow(&self, workflow_id: &str) {
        let broadcast_key = format!("broadcast:{}", workflow_id);
        let mut channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        channels.remove(&broadcast_key);
        // Also remove any channels that reference this workflow
        channels.retain(|_k, _v| true); // broadcast is the main one; direct/topic are not workflow-scoped
        info!("AgentComms: cleared channels for workflow={}", workflow_id);
    }

    /// Get channel key from ChannelType.
    fn channel_key(channel: &ChannelType) -> String {
        match channel {
            ChannelType::Direct { target_id } => format!("direct:{}", target_id),
            ChannelType::Broadcast { workflow_id } => format!("broadcast:{}", workflow_id),
            ChannelType::Topic { name } => format!("topic:{}", name),
        }
    }

    /// Filter messages from a queue, optionally skipping until after `since_id`.
    fn filter_messages(
        queue: Option<&VecDeque<AgentMessage>>,
        since_id: Option<&str>,
    ) -> Vec<AgentMessage> {
        let queue = match queue {
            Some(q) => q,
            None => return Vec::new(),
        };

        match since_id {
            None => {
                // Also filter expired on read
                let now = chrono::Utc::now();
                queue
                    .iter()
                    .filter(|msg| !Self::is_expired(msg, &now))
                    .cloned()
                    .collect()
            }
            Some(id) => {
                let now = chrono::Utc::now();
                let mut found = false;
                queue
                    .iter()
                    .filter(move |msg| {
                        if found {
                            !Self::is_expired(msg, &now)
                        } else {
                            if msg.id == id {
                                found = true;
                            }
                            false
                        }
                    })
                    .cloned()
                    .collect()
            }
        }
    }

    /// Check if a message has expired based on its TTL.
    fn is_expired(msg: &AgentMessage, now: &chrono::DateTime<chrono::Utc>) -> bool {
        if let Some(ttl) = msg.ttl_secs {
            if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&msg.timestamp) {
                let expiry = ts + chrono::Duration::seconds(ttl as i64);
                return *now >= expiry;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(from: &str, channel: ChannelType) -> AgentMessage {
        AgentMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.to_string(),
            channel,
            payload: serde_json::json!({"text": "hello"}),
            timestamp: chrono::Utc::now().to_rfc3339(),
            reply_to: None,
            ttl_secs: None,
        }
    }

    fn make_msg_with_id(from: &str, channel: ChannelType, id: &str) -> AgentMessage {
        AgentMessage {
            id: id.to_string(),
            from: from.to_string(),
            channel,
            payload: serde_json::json!({"text": "hello"}),
            timestamp: chrono::Utc::now().to_rfc3339(),
            reply_to: None,
            ttl_secs: None,
        }
    }

    #[test]
    fn test_direct_message_delivered_to_target() {
        let bus = AgentCommsBus::new(100);
        let msg = make_msg(
            "node-a",
            ChannelType::Direct {
                target_id: "node-b".to_string(),
            },
        );
        bus.publish(msg).unwrap();

        let msgs = bus.get_messages("node-b", None);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].from, "node-a");

        // node-a should have no messages
        let msgs_a = bus.get_messages("node-a", None);
        assert!(msgs_a.is_empty());
    }

    #[test]
    fn test_broadcast_reaches_all() {
        let bus = AgentCommsBus::new(100);
        let msg = make_msg(
            "node-a",
            ChannelType::Broadcast {
                workflow_id: "wf-1".to_string(),
            },
        );
        bus.publish(msg).unwrap();

        let msgs = bus.get_broadcast_messages("wf-1", None);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].from, "node-a");
    }

    #[test]
    fn test_topic_subscription_and_retrieval() {
        let bus = AgentCommsBus::new(100);

        // Publish 3 messages to same topic
        for i in 0..3 {
            let msg = AgentMessage {
                id: uuid::Uuid::new_v4().to_string(),
                from: format!("node-{}", i),
                channel: ChannelType::Topic {
                    name: "results".to_string(),
                },
                payload: serde_json::json!({"i": i}),
                timestamp: chrono::Utc::now().to_rfc3339(),
                reply_to: None,
                ttl_secs: None,
            };
            bus.publish(msg).unwrap();
        }

        let msgs = bus.get_topic_messages("results", None);
        assert_eq!(msgs.len(), 3);
    }

    #[test]
    fn test_ttl_expiry_and_sweep() {
        let bus = AgentCommsBus::new(100);

        // Message with 0-second TTL (already expired)
        let msg = AgentMessage {
            id: "expired-1".to_string(),
            from: "node-a".to_string(),
            channel: ChannelType::Direct {
                target_id: "node-b".to_string(),
            },
            payload: serde_json::json!(null),
            // Set timestamp 10 seconds in the past
            timestamp: (chrono::Utc::now() - chrono::Duration::seconds(10)).to_rfc3339(),
            reply_to: None,
            ttl_secs: Some(1), // 1 second TTL, but timestamp is 10s ago
        };
        bus.publish(msg).unwrap();

        // Message with no TTL (should survive)
        let msg2 = make_msg(
            "node-c",
            ChannelType::Direct {
                target_id: "node-b".to_string(),
            },
        );
        bus.publish(msg2).unwrap();

        // Sweep should remove the expired one
        let removed = bus.sweep_expired();
        assert_eq!(removed, 1);

        // Only the non-TTL message should remain
        let msgs = bus.get_messages("node-b", None);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].from, "node-c");
    }

    #[test]
    fn test_backpressure_drops_oldest() {
        let bus = AgentCommsBus::new(3);

        // Publish 4 messages, oldest should be dropped
        for i in 0..4 {
            let msg = AgentMessage {
                id: format!("msg-{}", i),
                from: "sender".to_string(),
                channel: ChannelType::Direct {
                    target_id: "receiver".to_string(),
                },
                payload: serde_json::json!({"seq": i}),
                timestamp: chrono::Utc::now().to_rfc3339(),
                reply_to: None,
                ttl_secs: None,
            };
            bus.publish(msg).unwrap();
        }

        let msgs = bus.get_messages("receiver", None);
        assert_eq!(msgs.len(), 3);
        // First message (msg-0) should have been dropped
        assert_eq!(msgs[0].id, "msg-1");
        assert_eq!(msgs[1].id, "msg-2");
        assert_eq!(msgs[2].id, "msg-3");
    }

    #[test]
    fn test_get_messages_with_since_id() {
        let bus = AgentCommsBus::new(100);

        let channel = ChannelType::Direct {
            target_id: "node-b".to_string(),
        };
        let msg1 = make_msg_with_id("node-a", channel.clone(), "id-1");
        let msg2 = make_msg_with_id("node-a", channel.clone(), "id-2");
        let msg3 = make_msg_with_id("node-a", channel.clone(), "id-3");

        bus.publish(msg1).unwrap();
        bus.publish(msg2).unwrap();
        bus.publish(msg3).unwrap();

        // Get messages after id-1 → should return id-2, id-3
        let msgs = bus.get_messages("node-b", Some("id-1"));
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].id, "id-2");
        assert_eq!(msgs[1].id, "id-3");

        // Get messages after id-2 → should return id-3
        let msgs = bus.get_messages("node-b", Some("id-2"));
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].id, "id-3");

        // Get messages after id-3 → should return empty
        let msgs = bus.get_messages("node-b", Some("id-3"));
        assert!(msgs.is_empty());

        // Unknown since_id → returns empty (ID not found, so "found" never becomes true)
        let msgs = bus.get_messages("node-b", Some("nonexistent"));
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_clear_workflow_removes_broadcast() {
        let bus = AgentCommsBus::new(100);

        let msg = make_msg(
            "node-a",
            ChannelType::Broadcast {
                workflow_id: "wf-1".to_string(),
            },
        );
        bus.publish(msg).unwrap();

        let msg2 = make_msg(
            "node-b",
            ChannelType::Broadcast {
                workflow_id: "wf-2".to_string(),
            },
        );
        bus.publish(msg2).unwrap();

        bus.clear_workflow("wf-1");

        // wf-1 should be empty
        let msgs = bus.get_broadcast_messages("wf-1", None);
        assert!(msgs.is_empty());

        // wf-2 should be unaffected
        let msgs = bus.get_broadcast_messages("wf-2", None);
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn test_reply_to_threading_preserved() {
        let bus = AgentCommsBus::new(100);

        let original_id = "original-msg-id".to_string();
        let original = AgentMessage {
            id: original_id.clone(),
            from: "node-a".to_string(),
            channel: ChannelType::Direct {
                target_id: "node-b".to_string(),
            },
            payload: serde_json::json!({"question": "what is 2+2?"}),
            timestamp: chrono::Utc::now().to_rfc3339(),
            reply_to: None,
            ttl_secs: None,
        };
        bus.publish(original).unwrap();

        let reply = AgentMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: "node-b".to_string(),
            channel: ChannelType::Direct {
                target_id: "node-a".to_string(),
            },
            payload: serde_json::json!({"answer": 4}),
            timestamp: chrono::Utc::now().to_rfc3339(),
            reply_to: Some(original_id.clone()),
            ttl_secs: None,
        };
        bus.publish(reply).unwrap();

        let msgs = bus.get_messages("node-a", None);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].reply_to, Some(original_id));
    }

    #[test]
    fn test_ttl_filtered_on_read() {
        let bus = AgentCommsBus::new(100);

        // Expired message (not yet swept, but should be filtered on read)
        let expired = AgentMessage {
            id: "old".to_string(),
            from: "node-a".to_string(),
            channel: ChannelType::Topic {
                name: "events".to_string(),
            },
            payload: serde_json::json!(null),
            timestamp: (chrono::Utc::now() - chrono::Duration::seconds(100)).to_rfc3339(),
            reply_to: None,
            ttl_secs: Some(5),
        };
        bus.publish(expired).unwrap();

        // Fresh message
        let fresh = AgentMessage {
            id: "new".to_string(),
            from: "node-b".to_string(),
            channel: ChannelType::Topic {
                name: "events".to_string(),
            },
            payload: serde_json::json!(null),
            timestamp: chrono::Utc::now().to_rfc3339(),
            reply_to: None,
            ttl_secs: Some(3600),
        };
        bus.publish(fresh).unwrap();

        // Read should filter out expired even without sweep
        let msgs = bus.get_topic_messages("events", None);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].id, "new");
    }

    #[test]
    fn test_channel_key_format() {
        assert_eq!(
            AgentCommsBus::channel_key(&ChannelType::Direct {
                target_id: "n1".to_string()
            }),
            "direct:n1"
        );
        assert_eq!(
            AgentCommsBus::channel_key(&ChannelType::Broadcast {
                workflow_id: "wf".to_string()
            }),
            "broadcast:wf"
        );
        assert_eq!(
            AgentCommsBus::channel_key(&ChannelType::Topic {
                name: "foo".to_string()
            }),
            "topic:foo"
        );
    }
}
