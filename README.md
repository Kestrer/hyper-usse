**DO NOT USE THIS LIBRARY**. Use instead a combination of
[`Body::wrap_stream`](https://docs.rs/hyper/0.13.2/hyper/body/struct.Body.html#method.wrap_stream)
and [`tokio::sync::broadcast`](https://docs.rs/tokio/0.2.11/tokio/sync/broadcast/index.html), and
either build the events manually or use [`uhttp_sse`](https://crates.io/crates/uhttp_sse).

# hyper-usse

An SSE (server sent events) server for use with Hyper, simpler than [hyper-sse](https://crates.io/crates/hyper-sse).

See [examples](https://github.com/koxiaet/hyper-usse/tree/master/examples) for some examples of how to use this library.

## License

Dual-licensed under MIT License and Apache 2.0.
