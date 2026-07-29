const http = require("node:http");

const server = http.createServer((_request, response) => {
  response.writeHead(200, { "content-type": "text/plain" });
  response.end("node JIT is enabled\n");
});

server.listen(8081, "127.0.0.1");
