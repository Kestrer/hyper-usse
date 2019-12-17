// Example of a website built with SSE.
//
// Go to http://localhost:8000/.
use futures::future;
use futures::stream::StreamExt;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response};
use hyper_usse::Event;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;

const HTML: &str = r#"
<!DOCTYPE html>
<html>
    <head>
        <title>hyper-usse demo</title>
        <meta charset="utf-8" />
    </head>
    <body>
        <h1>hyper-usse demo</h1>
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
    sse: Arc<Mutex<hyper_usse::Server>>,
    request: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    let result = match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => Response::new(Body::from(HTML)),
        (&Method::GET, "/sse") => {
            let (channel, body) = Body::channel();
            sse.lock().await.add_client(channel);
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
    let sse = Arc::new(Mutex::new(hyper_usse::Server::new()));

    let server = hyper::Server::bind(&([127, 0, 0, 1], 8000).into()).serve(make_service_fn(|_| {
        let sse = Arc::clone(&sse);

        async move {
            Ok::<_, hyper::Error>(service_fn(move |request: Request<Body>| {
                process_request(Arc::clone(&sse), request)
            }))
        }
    }));

    let events = time::interval(Duration::from_secs(3)).for_each(|_| {
        async {
            println!("Sending message...");
            sse.lock()
                .await
                .send_to_clients(Event::new("Some data").to_sse())
                .await;
        }
    });

    if let Err(err) = future::join(server, events).await.0 {
        eprintln!("Server error: {}", err);
    }
}
