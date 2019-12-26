//! A library for an SSE (server sent events) server, for use with Hyper.
//!
//! Start a server with `Server`, and use `EventBuilder` to generate events to send with `Server`.
//! See [examples](https://github.com/koxiaet/hyper-usse/tree/master/examples) for usage examples.
use futures::future;
use hyper::body::{Bytes, Sender};
use std::mem;
use std::fmt::{self, Display, Formatter};

/// A struct used to build server sent events.
///
/// # Examples
/// Build an event with just data:
/// ```
/// # use hyper_usse::EventBuilder;
/// EventBuilder::new("Data").build();
/// ```
/// Build an event with an ID:
/// ```
/// # use hyper_usse::EventBuilder;
/// EventBuilder::new("Data").id("Id").build();
/// ```
/// Build an event with an event type:
/// ```
/// # use hyper_usse::EventBuilder;
/// EventBuilder::new("Data").event_type("update").build();
/// ```
///
/// Because `EventBuilder` implements `Into<Bytes>` you don't have to call `build` to pass it to
/// the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventBuilder<'data, 'id, 'event> {
    pub data: &'data str,
    pub id: Option<&'id str>,
    pub event_type: Option<&'event str>,
}

impl<'data, 'id, 'event> EventBuilder<'data, 'id, 'event> {
    /// Create a new builder with data, no id and no event type.
    pub fn new(data: &'data str) -> Self {
        Self {
            data,
            id: None,
            event_type: None,
        }
    }
    /// Set the data.
    pub fn data(mut self, data: &'data str) -> Self {
        self.data = data;
        self
    }
    /// Set the event id.
    pub fn id(mut self, id: &'id str) -> Self {
        self.id = Some(id);
        self
    }
    /// Set the event type.
    pub fn event_type(mut self, event_type: &'event str) -> Self {
        self.event_type = Some(event_type);
        self
    }
    /// Clear the event id.
    pub fn clear_id(mut self) -> Self {
        self.id = None;
        self
    }
    /// Clear the event type.
    pub fn clear_type(mut self) -> Self {
        self.event_type = None;
        self
    }
    /// Build the event.
    pub fn build(self) -> String {
        let mut event = String::with_capacity(
            self.id.map(|id| 5 + id.len()).unwrap_or(0) +
            self.event_type.map(|event| 8 + event.len()).unwrap_or(0) +
            self.data.lines().count()*6 + self.data.len() +
            1
        );
        if let Some(id) = self.id {
            event.push_str("id: ");
            event.push_str(id);
            event.push('\n');
        }
        if let Some(event_type) = self.event_type {
            event.push_str("event: ");
            event.push_str(event_type);
            event.push('\n');
        }
        for line in self.data.lines() {
            event.push_str("data: ");
            event.push_str(line);
            event.push('\n');
        }
        event.push('\n');
        event
    }
}

impl<'data, 'id, 'event> Display for EventBuilder<'data, 'id, 'event> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.build())
    }
}

impl<'data, 'id, 'event> Into<Bytes> for EventBuilder<'data, 'id, 'event> {
    fn into(self) -> Bytes {
        self.build().into()
    }
}

/// An SSE server.
#[derive(Debug, Default)]
pub struct Server {
    clients: Vec<Sender>,
}

impl Server {
    /// Create a new server with no clients.
    pub fn new() -> Self {
        Server {
            clients: Vec::new(),
        }
    }

    /// Add a client to a server. `Sender` can be obtained by calling `Body::channel()`.
    pub fn add_client(&mut self, client: Sender) {
        self.clients.push(client);
    }

    /// Send some text to the clients. Most often, this text is made using an
    /// [EventBuilder](struct.EventBuilder.html). This will automatically remove all disconnected
    /// clients.
    ///
    /// This function returns the number of currently connected clients.
    pub async fn send_to_clients<B: Into<Bytes>>(&mut self, text: B) -> usize {
        let bytes = text.into();
        let mut sent = future::join_all(self.clients.iter_mut().map(|client| {
            let bytes = Bytes::clone(&bytes);
            async move { client.send_data(bytes).await.is_ok() }
        })).await.into_iter();
        self.clients.retain(|_| sent.next().unwrap());
        self.clients.len()
    }

    /// Send a heartbeat (empty SSE) to all clients. This does not perform any action, but will
    /// prevent your connection being timed out for lasting too long without any data being sent.
    ///
    /// This function returns the number of currently connected clients.
    pub async fn send_heartbeat(&mut self) -> usize {
        self.send_to_clients(":\n\n").await
    }

    /// Disconnect all clients that are currently connected to the server.
    pub fn disconnect_all(&mut self) {
        for client in mem::replace(&mut self.clients, Vec::new()) {
            client.abort();
        }
    }

    /// Count the number of currently held connections.
    ///
    /// Note that this may be an over-estimate of the number of currently connected clients, as
    /// some clients may have disconnected since the last `send_to_clients` or `send_heartbeat`
    /// (both of which prune the list of connections to those which still have a connected client).
    pub fn connections(&self) -> usize {
        self.clients.len()
    }
}
