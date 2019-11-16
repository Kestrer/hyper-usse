// Example of a website built with SSE.
use futures::compat::Future01CompatExt;
use futures::future;
use futures_locks::Mutex;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response};
use hyper_usse::Event;
use std::time::Duration;
use tokio::prelude::*;
use tokio::timer::Interval;

const HTML: &str = r#"
<!DOCTYPE html>
<html>
    <head>
        <title>hyper-usse demo</title>
        <meta charset="utf-8" />
    </head>
    <body>
        <ul id="list"></ul>
        <script>
let server = new EventSource("http://localhost:8000/sse");
server.onmessage = event => {
    let elem = document.createElement("li");
    elem.innerHTML = event.data;

    document.getElementById("list").appendChild(elem);
};
        </script>
    </body>
</html>"#;

async fn process_request(
    sse: Mutex<hyper_usse::Server>,
    request: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    let result = match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => Response::new(Body::from(HTML)),
        (&Method::GET, "/sse") => {
            let (channel, body) = Body::channel();
            sse.lock().compat().await.unwrap().add_client(channel);
            Response::builder()
                .header("Content-Type", "text/event-stream")
                .header("Cache-Control", "no-cache")
                .body(body)
                .unwrap()
        }
        _ => Response::builder()
            .status(404)
            .body(Body::from("Not found."))
            .unwrap(),
    };
    Ok(result)
}

#[tokio::main]
async fn main() {
    let sse = Mutex::new(hyper_usse::Server::new());

    let server = hyper::Server::bind(&([127, 0, 0, 1], 8000).into()).serve(make_service_fn(|_| {
        let sse = Mutex::clone(&sse);

        async move {
            Ok::<_, hyper::Error>(service_fn(move |request: Request<Body>| {
                process_request(Mutex::clone(&sse), request)
            }))
        }
    }));

    let events = Interval::new_interval(Duration::from_secs(3))
        .for_each(|_| {
            async {
                println!("Sending message...");
                sse.lock()
                    .compat()
                    .await
                    .unwrap()
                    .send_to_clients(&Event::new("Some data").to_sse())
                    .await;
            }
        });

    if let Err(err) = future::join(server, events).await.0 {
        eprintln!("Server error: {}", err);
    }
}
