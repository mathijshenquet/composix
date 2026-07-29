use tiny_http::{Response, Server};

fn main() {
    let server = Server::http("0.0.0.0:18082").expect("listen on port 18082");
    for request in server.incoming_requests() {
        request
            .respond(Response::from_string("hello from RUN v0\n"))
            .expect("send response");
    }
}
