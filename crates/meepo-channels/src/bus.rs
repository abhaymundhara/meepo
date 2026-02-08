//! Central message bus for routing messages between channels and the agent

use meepo_core::types::{IncomingMessage, OutgoingMessage, ChannelType};
use tokio::sync::mpsc;
use std::collections::HashMap;
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use tracing::{info, error, debug};

/// Trait that all channel adapters implement
#[async_trait]
pub trait MessageChannel: Send + Sync {
    /// Start listening for messages, sending them to the provided sender
    async fn start(&self, tx: mpsc::Sender<IncomingMessage>) -> Result<()>;

    /// Send a message through this channel
    async fn send(&self, msg: OutgoingMessage) -> Result<()>;

    /// Which channel type this adapter handles
    fn channel_type(&self) -> ChannelType;
}

/// Central message bus that routes messages between channels and the agent
pub struct MessageBus {
    channels: HashMap<ChannelType, Box<dyn MessageChannel>>,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: mpsc::Receiver<IncomingMessage>,
}

impl MessageBus {
    /// Create a new message bus with the specified buffer size for incoming messages
    pub fn new(buffer_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer_size);
        info!("Created message bus with buffer size {}", buffer_size);
        Self {
            channels: HashMap::new(),
            incoming_tx: tx,
            incoming_rx: rx,
        }
    }

    /// Register a channel adapter with the bus
    pub fn register(&mut self, channel: Box<dyn MessageChannel>) {
        let channel_type = channel.channel_type();
        info!("Registering channel: {}", channel_type);
        self.channels.insert(channel_type, channel);
    }

    /// Start all registered channel listeners
    /// Each channel runs in its own tokio task
    pub async fn start_all(&self) -> Result<()> {
        info!("Starting all {} registered channels", self.channels.len());

        for (channel_type, channel) in &self.channels {
            let tx = self.incoming_tx.clone();
            let channel_type = channel_type.clone();

            // We need to work around the trait object limitation
            // by having each channel implementation handle its own async execution
            debug!("Starting channel: {}", channel_type);

            // Clone the sender for this channel's task
            let tx_clone = tx.clone();

            // Start the channel (this should spawn its own task internally)
            if let Err(e) = channel.start(tx_clone).await {
                error!("Failed to start channel {}: {}", channel_type, e);
                return Err(anyhow!("Failed to start channel {}: {}", channel_type, e));
            }

            info!("Successfully started channel: {}", channel_type);
        }

        info!("All channels started successfully");
        Ok(())
    }

    /// Receive the next incoming message from any channel
    /// Returns None if all channel senders have been dropped
    pub async fn recv(&mut self) -> Option<IncomingMessage> {
        self.incoming_rx.recv().await
    }

    /// Send an outgoing message to the appropriate channel
    pub async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        let channel_type = &msg.channel;
        debug!("Routing outgoing message to channel: {}", channel_type);

        let channel = self.channels
            .get(channel_type)
            .ok_or_else(|| anyhow!("No channel registered for type: {}", channel_type))?;

        channel.send(msg).await?;
        Ok(())
    }

    /// Get the number of registered channels
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Check if a specific channel type is registered
    pub fn has_channel(&self, channel_type: &ChannelType) -> bool {
        self.channels.contains_key(channel_type)
    }

    /// Split the bus into a receiver and a sender handle.
    /// This allows the receiver to be used in a select! loop while the sender
    /// is cloned into spawned tasks for routing responses.
    pub fn split(self) -> (mpsc::Receiver<IncomingMessage>, BusSender) {
        let sender = BusSender {
            channels: self.channels,
        };
        (self.incoming_rx, sender)
    }
}

/// Send-only handle for the message bus
/// Separated from the receiver to allow concurrent send/receive
pub struct BusSender {
    channels: HashMap<ChannelType, Box<dyn MessageChannel>>,
}

impl BusSender {
    /// Send an outgoing message to the appropriate channel
    pub async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        let channel_type = &msg.channel;
        debug!("Routing outgoing message to channel: {}", channel_type);

        let channel = self.channels
            .get(channel_type)
            .ok_or_else(|| anyhow!("No channel registered for type: {}", channel_type))?;

        channel.send(msg).await?;
        Ok(())
    }

    /// Check if a specific channel type is registered
    pub fn has_channel(&self, channel_type: &ChannelType) -> bool {
        self.channels.contains_key(channel_type)
    }
}
