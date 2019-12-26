//! A library for an SSE (server sent events) server, for use with Hyper.
//!
//! Start a server with `Server`, and use `Event` to generate events to send with `Server`. See the
//! [README](https://github.com/koxiaet/hyper-usse/blob/master/README.md) for more, and
//! [examples](https://github.com/koxiaet/hyper-usse/tree/master/examples) for usage examples.
use futures::future;
use hyper::body::{Bytes, Sender};
use std::borrow::Cow;

/// A server sent event.
///
/// The fields `id`, `event` and `data` set the `id`, `event` and `data` parameters for the event
/// respectively. `id` and `event` are optional. To convert it to an SSE string, use the `to_sse`
/// function.
///
/// # Examples
/// ```
/// let event = hyper_usse::Event::new("some text\nmore text").set_id("1").set_event("send_text");
/// assert_eq!(event.to_sse(), "id: 1
/// event: send_text
/// data: some text
/// data: more text\n\n");
/// ```
#[derive(Debug, Default)]
pub struct Event {
    pub id: Option<Cow<'static, str>>,
    pub event: Option<Cow<'static, str>>,
    pub data: Cow<'static, str>,
}

impl Event {
    /// Creates a new Event from data.
    pub fn new<T: Into<Cow<'static, str>>>(data: T) -> Self {
        Event {
            id: None,
            event: None,
            data: data.into(),
        }
    }

    /// Sets the id of the event. As it returns `self`, it is useful for creating an event inline.
    pub fn set_id<T: Into<Cow<'static, str>>>(mut self, id: T) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the event type of the event. Like `set_id`, it returns `self` and so can easily be
    /// chained.
    pub fn set_event<T: Into<Cow<'static, str>>>(mut self, event: T) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Converts the Event to an SSE string.
    pub fn to_sse(&self) -> String {
        let mut sse = String::new();
        if let Some(id) = &self.id {
            sse.push_str(&format!("id: {}\n", id));
        }
        if let Some(event) = &self.event {
            sse.push_str(&format!("event: {}\n", event));
        }
        for line in self.data.lines() {
            sse.push_str(&format!("data: {}\n", line));
        }
        sse.push('\n');
        sse
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

    /// Send some text to the clients. Most often, this text is generated by calling `to_sse` on an
    /// [Event](struct.Event.html).
    pub async fn send_to_clients<B: Into<Bytes>>(&mut self, text: B) {
        let bytes = text.into();
        let mut sent = future::join_all(self.clients.iter_mut().map(|client| {
            let bytes = Bytes::clone(&bytes);
            async move { client.send_data(bytes).await.is_ok() }
        }))
        .await
        .into_iter();
        self.clients.retain(|_| sent.next().unwrap());
    }

    /// Send a heartbeat (empty SSE) to all clients. This does not perform any action, but will
    /// prevent your connection being timed out for lasting too long without any data being sent.
    pub async fn send_heartbeat(&mut self) {
        self.send_to_clients(":\n\n").await
    }

    /// Count the number of currently held connections. Note that this may be an
    /// over-estimate of the number of currently connected clients, as some
    /// clients may have disconnected since the last `send_to_clients` or
    /// `send_heartbeat` (both of which prune the list of connections to those
    /// which still have a connected client).
    pub fn connections(&self) -> usize {
        self.clients.len()
    }
}
