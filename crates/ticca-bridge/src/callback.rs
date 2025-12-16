//! Streaming callback infrastructure for Python-to-Rust updates
//!
//! This module provides the callback mechanism that allows Python code
//! to push streaming updates to Rust without blocking. It uses tokio's
//! mpsc channels for async communication.

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::error::{BridgeError, BridgeResult};
use crate::types::StreamChunk;

/// Channel capacity for streaming chunks
const DEFAULT_CHANNEL_CAPACITY: usize = 256;

/// Sender half of the stream callback
///
/// This is passed to Python and used to send streaming updates.
/// It's cloneable and thread-safe.
#[derive(Clone)]
pub struct StreamSender {
    inner: mpsc::Sender<StreamChunk>,
}

impl StreamSender {
    /// Create a new stream sender from an mpsc sender
    fn new(sender: mpsc::Sender<StreamChunk>) -> Self {
        Self { inner: sender }
    }

    /// Send a streaming chunk
    ///
    /// Returns an error if the receiver has been dropped.
    pub async fn send(&self, chunk: StreamChunk) -> BridgeResult<()> {
        self.inner
            .send(chunk)
            .await
            .map_err(|e| BridgeError::Channel(format!("Failed to send chunk: {e}")))
    }

    /// Try to send a chunk without waiting
    ///
    /// Returns an error if the channel is full or closed.
    pub fn try_send(&self, chunk: StreamChunk) -> BridgeResult<()> {
        self.inner.try_send(chunk).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => {
                BridgeError::Channel("Channel full".to_string())
            }
            mpsc::error::TrySendError::Closed(_) => {
                BridgeError::Channel("Channel closed".to_string())
            }
        })
    }

    /// Check if the receiver is still alive
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}

/// Receiver half of the stream callback
///
/// Used by Rust to receive streaming updates from Python.
pub struct StreamReceiver {
    inner: mpsc::Receiver<StreamChunk>,
}

impl StreamReceiver {
    /// Create a new stream receiver from an mpsc receiver
    fn new(receiver: mpsc::Receiver<StreamChunk>) -> Self {
        Self { inner: receiver }
    }

    /// Receive the next chunk
    ///
    /// Returns `None` if the sender has been dropped.
    pub async fn recv(&mut self) -> Option<StreamChunk> {
        self.inner.recv().await
    }

    /// Try to receive without waiting
    pub fn try_recv(&mut self) -> Option<StreamChunk> {
        self.inner.try_recv().ok()
    }

    /// Close the receiver, causing senders to fail
    pub fn close(&mut self) {
        self.inner.close();
    }
}

/// A complete stream callback pair
///
/// Contains both the sender (for Python) and receiver (for Rust).
pub struct StreamCallback {
    sender: StreamSender,
    receiver: Option<StreamReceiver>,
}

impl StreamCallback {
    /// Create a new stream callback with default capacity
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHANNEL_CAPACITY)
    }

    /// Create a new stream callback with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity);
        Self {
            sender: StreamSender::new(tx),
            receiver: Some(StreamReceiver::new(rx)),
        }
    }

    /// Get a clone of the sender
    ///
    /// The sender can be cloned and passed to multiple sources.
    pub fn sender(&self) -> StreamSender {
        self.sender.clone()
    }

    /// Take the receiver
    ///
    /// This can only be called once; subsequent calls return `None`.
    pub fn take_receiver(&mut self) -> Option<StreamReceiver> {
        self.receiver.take()
    }

    /// Split into sender and receiver
    ///
    /// Consumes self and returns both halves.
    pub fn split(mut self) -> (StreamSender, StreamReceiver) {
        let receiver = self
            .receiver
            .take()
            .expect("Receiver already taken - this is a bug");
        (self.sender, receiver)
    }
}

impl Default for StreamCallback {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper for passing stream sender to Python
///
/// This provides a synchronous interface that Python can use
/// to send chunks without dealing with async directly.
pub struct PythonStreamCallback {
    sender: StreamSender,
    runtime: Arc<tokio::runtime::Handle>,
}

impl PythonStreamCallback {
    /// Create a new Python stream callback
    pub fn new(sender: StreamSender, runtime: Arc<tokio::runtime::Handle>) -> Self {
        Self { sender, runtime }
    }

    /// Send a text delta
    pub fn send_text(&self, content: String) -> BridgeResult<()> {
        self.send_chunk(StreamChunk::text(content))
    }

    /// Send a thinking delta
    pub fn send_thinking(&self, content: String) -> BridgeResult<()> {
        self.send_chunk(StreamChunk::thinking(content))
    }

    /// Send an error
    pub fn send_error(&self, message: String) -> BridgeResult<()> {
        self.send_chunk(StreamChunk::error(message))
    }

    /// Send a raw chunk (JSON string)
    ///
    /// Parses the JSON and sends the resulting chunk.
    pub fn send_json(&self, json: &str) -> BridgeResult<()> {
        let chunk: StreamChunk =
            serde_json::from_str(json).map_err(|e| BridgeError::Serialization(e.to_string()))?;
        self.send_chunk(chunk)
    }

    /// Send a chunk, blocking if necessary
    fn send_chunk(&self, chunk: StreamChunk) -> BridgeResult<()> {
        // First try non-blocking send
        match self.sender.try_send(chunk.clone()) {
            Ok(()) => {
                debug!("Sent chunk via try_send");
                Ok(())
            }
            Err(BridgeError::Channel(msg)) if msg == "Channel full" => {
                // Channel is full, use blocking send via runtime
                warn!("Channel full, using blocking send");
                self.runtime.block_on(async { self.sender.send(chunk).await })
            }
            Err(e) => Err(e),
        }
    }

    /// Check if the receiver is still listening
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

/// Builder for creating stream callbacks with custom configuration
pub struct StreamCallbackBuilder {
    capacity: usize,
}

impl StreamCallbackBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            capacity: DEFAULT_CHANNEL_CAPACITY,
        }
    }

    /// Set the channel capacity
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// Build the stream callback
    pub fn build(self) -> StreamCallback {
        StreamCallback::with_capacity(self.capacity)
    }
}

impl Default for StreamCallbackBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentResponse, AgentState};

    #[tokio::test]
    async fn test_stream_callback_basic() {
        let mut callback = StreamCallback::new();
        let sender = callback.sender();
        let mut receiver = callback.take_receiver().expect("receiver");

        // Send a chunk
        sender
            .send(StreamChunk::text("Hello"))
            .await
            .expect("send");

        // Receive it
        let chunk = receiver.recv().await.expect("recv");
        match chunk {
            StreamChunk::TextDelta { content } => assert_eq!(content, "Hello"),
            _ => panic!("Wrong chunk type"),
        }
    }

    #[tokio::test]
    async fn test_stream_callback_multiple_chunks() {
        let (sender, mut receiver) = StreamCallback::new().split();

        // Send multiple chunks
        sender.send(StreamChunk::text("1")).await.expect("send 1");
        sender.send(StreamChunk::text("2")).await.expect("send 2");
        sender
            .send(StreamChunk::state_change(
                AgentState::Idle,
                AgentState::Thinking,
            ))
            .await
            .expect("send state");

        // Receive them in order
        let c1 = receiver.recv().await.expect("recv 1");
        let c2 = receiver.recv().await.expect("recv 2");
        let c3 = receiver.recv().await.expect("recv 3");

        assert!(matches!(c1, StreamChunk::TextDelta { content } if content == "1"));
        assert!(matches!(c2, StreamChunk::TextDelta { content } if content == "2"));
        assert!(matches!(c3, StreamChunk::StateChange { from: AgentState::Idle, to: AgentState::Thinking }));
    }

    #[tokio::test]
    async fn test_sender_closed_detection() {
        let mut callback = StreamCallback::new();
        let sender = callback.sender();
        let mut receiver = callback.take_receiver().expect("receiver");

        assert!(!sender.is_closed());

        // Close receiver
        receiver.close();
        drop(receiver);

        // Sender should detect closure
        assert!(sender.is_closed());
    }

    #[tokio::test]
    async fn test_done_chunk() {
        let (sender, mut receiver) = StreamCallback::new().split();

        let response = AgentResponse::text("msg-1", "Done!");
        sender
            .send(StreamChunk::done(response.clone()))
            .await
            .expect("send");

        let chunk = receiver.recv().await.expect("recv");
        match chunk {
            StreamChunk::Done { response: resp } => {
                assert_eq!(resp.message_id, "msg-1");
                assert_eq!(resp.content, "Done!");
            }
            _ => panic!("Wrong chunk type"),
        }
    }
}
