use tiny_http::{Response, Server};

fn main() {
    let server = Server::http("0.0.0.0:18083").expect("listen on port 18083");
    for request in server.incoming_requests() {
        request
            .respond(Response::from_string("hello from the chef chain\n"))
            .expect("send response");
    }
}
