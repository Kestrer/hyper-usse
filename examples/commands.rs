// Example of a website built with SSE.
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response};
use hyper_usse::EventBuilder;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{self, BufReader, AsyncBufReadExt};
use std::io::Write;

const HTML: &str = r#"
<!DOCTYPE html>
<html>
    <head>
        <title>Hyper-usse Demo</title>
        <meta charset="utf-8" />
    </head>
    <body>
        <h1>Hyper-usse Demo</h1>
        <p>Incoming events:</p>
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

async fn command_loop(
    sse: Arc<Mutex<hyper_usse::Server>>,
) {
    let mut stdin = BufReader::new(io::stdin());

    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();
        let mut line = String::new();
        stdin.read_line(&mut line).await.unwrap();

        let trimmed_line = line.trim();
        let (command, args) = match trimmed_line.find(' ') {
            Some(i) => {
                let (command, args) = trimmed_line.split_at(i);
                (command, &args[1..])
            },
            None => (trimmed_line, ""),
        };

        match command {
            "help" => {
                println!("help: show this menu");
                println!("send [data]: send data as a string to all connected clients");
                println!("count: show number of connected clients");
                println!("stop: stop the server");
            },
            "send" => {
                sse.lock().await.send_to_clients(EventBuilder::new(args)).await;
                println!("Sent '{}' to clients.", args);
            },
            "count" => {
                let connections = sse.lock().await.connections();
                match connections {
                    1 => println!("There is 1 connected client."),
                    _ => println!("There are {} connected clients.", connections),
                }
            },
            "stop" => {
                sse.lock().await.disconnect_all();
                break;
            }
            _ => println!("Unknown command {}.", command),
        }
    } 
}

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    let sse = Arc::new(Mutex::new(hyper_usse::Server::new()));

    let server = hyper::Server::bind(&([127, 0, 0, 1], 8000).into()).serve(make_service_fn(|_| {
        let sse = Arc::clone(&sse);

        async move {
            Ok::<_, hyper::Error>(service_fn(move |request: Request<Body>| {
                process_request(Arc::clone(&sse), request)
            }))
        }
    }));

    let server = server.with_graceful_shutdown(command_loop(Arc::clone(&sse)));

    println!("Go to http://localhost:8000/.");

    server.await?;

    Ok(())
}
