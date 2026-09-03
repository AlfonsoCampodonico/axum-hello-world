# axum-hello-world

A hello-world HTTP API in **Rust**, built to deploy on **Laravel Cloud**.

It is deliberately small — a greeting, a health check, an echo — because the
interesting part is not the app. It is what a Rust web service has to get right
to run on the platform, which this repo documents and enforces in code:

- **Bind the IPv6 wildcard**, not `0.0.0.0`. Cloud's health probe arrives over
  IPv6 while the in-pod proxy forwards over IPv4 loopback, so the listener has
  to serve both. See [The dual-stack gotcha](#the-dual-stack-gotcha).
- **Honour the injected `PORT`**, falling back to `9115` locally.
- **Shut down on `SIGTERM`**, so in-flight requests finish when a container is
  replaced.

Four direct dependencies (`axum`, `tokio`, `serde`, `serde_json`), a
1 MB static binary, an 8 MB image, and the OpenAPI spec compiled into the
executable so there are no files to ship beside it.

## Endpoints

Interactive docs: **`/docs`** · Raw spec: **`/openapi.json`**

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Greet the world |
| GET | `/hello` | Alias for `/` |
| GET | `/hello/{name}` | Greet someone by name (trimmed; 1–64 chars) |
| GET | `/health` | Liveness probe — the endpoint Cloud hits |
| GET | `/version` | Crate and framework versions |
| POST | `/echo` | Echo a JSON body back |
| GET | `/openapi.json` | The OpenAPI 3.1 spec |
| GET | `/docs` | Redoc UI for the spec |

Every failure — a bad name, malformed JSON, an unknown path, a wrong method —
comes back in one envelope:

```json
{ "error": { "status": 400, "message": "name must not be blank" } }
```

## Run locally

```bash
make run                 # cargo run, listening on [::]:9115
make test                # 19 integration tests
make check               # fmt + clippy + test, what CI runs
open http://localhost:9115/docs
```

```bash
curl -s localhost:9115/hello/Ferris
# {"message":"Hello, Ferris!"}

curl -s -X POST localhost:9115/echo \
  -H 'content-type: application/json' \
  -d '{"hello":["world",42]}'
# {"echo":{"hello":["world",42]}}
```

## The dual-stack gotcha

This is the platform lesson the repo exists to record, and it bites every
runtime in this demo suite differently.

Laravel Cloud health-checks your container over **IPv6**, while the proxy in
front of your app forwards requests over **IPv4 loopback**. A listener bound to
`0.0.0.0` is IPv4-only: it serves traffic fine and then fails the health check,
so the deploy never goes green.

In Rust the fix is to bind the IPv6 wildcard and let the kernel accept IPv4
through v4-mapped addresses:

```rust
// src/main.rs
let dual_stack = SocketAddr::from((Ipv6Addr::UNSPECIFIED, port));
TcpListener::bind(dual_stack).await
```

That works because neither `std` nor `tokio` sets the `IPV6_V6ONLY` socket
option, so the socket inherits the OS default — off on Linux and macOS. One
socket, both protocols. You can see it directly:

```console
$ lsof -nP -iTCP:9115 -sTCP:LISTEN
axum-hello-world  ...  IPv6  ...  TCP *:9115 (LISTEN)

$ curl -s -4 http://127.0.0.1:9115/
{"message":"Hello, World!"}
$ curl -s -6 "http://[::1]:9115/health"
{"status":"ok"}
```

Note the asymmetry with the other runtimes: in Rust `[::]` is the right answer,
whereas for `uvicorn` `--host ::` is IPv6-**only** and you need `--host ''`, and
in Node you pass no host at all. `main.rs` also falls back to IPv4 if the IPv6
bind fails outright, for hosts built without IPv6 support.

## Deploy to Laravel Cloud

Laravel Cloud has no native Rust runtime, so the app deploys as a Docker image.

1. Push this repo to GitHub and create a new application in Laravel Cloud
   pointing at your repository and branch.
2. Choose the **Dockerfile** build strategy. No language runtime configuration
   is needed — the included `Dockerfile` produces a static musl binary on
   `distroless/static`, which contains no shell, no package manager, and runs
   as `nonroot`.
3. Cloud injects `PORT` for the web process and the app binds it automatically.
   Confirm the environment's exposed port matches.
4. Deploy, then open `/docs` on your assigned domain.

## Build the container

```bash
make docker              # docker build --platform linux/amd64 -t axum-hello-world:dev .
make docker-run          # PORT=9115, published on the same port
```

The build is two-staged and dependency-cached: the first stage compiles the
dependency tree against a stub `main.rs`, so editing `src/` rebuilds only this
crate. Cloud runs amd64, so `make docker` pins `linux/amd64` — on Apple Silicon
the resulting image runs under emulation locally, which is fine for a smoke
test.

## Layout

```
src/main.rs        bind the listener, serve, shut down gracefully
src/lib.rs         the router — separate from main so tests can drive it
src/routes.rs      handlers
src/error.rs       the single error shape
assets/            openapi.json and the docs page, compiled into the binary
tests/api.rs       integration tests over the router, no socket bound
```

Tests drive the router in-process with `tower::ServiceExt::oneshot`, so the
suite binds no port, never races, and finishes in about 10 ms.

## Sibling repos

One framework per repo, each documenting the platform gotcha it hits:
[`fastapi-hello-world`](https://github.com/AlfonsoCampodonico/fastapi-hello-world) ·
[`flask-hello-world`](https://github.com/AlfonsoCampodonico/flask-hello-world) ·
[`django-hello-world`](https://github.com/AlfonsoCampodonico/django-hello-world) ·
[`nestjs-hello-world`](https://github.com/AlfonsoCampodonico/nestjs-hello-world) ·
[`spring-hello-world`](https://github.com/AlfonsoCampodonico/spring-hello-world) ·
[`drupal-hello-world`](https://github.com/AlfonsoCampodonico/drupal-hello-world) ·
[`go-laravelcloud`](https://github.com/AlfonsoCampodonico/go-laravelcloud)

## License

MIT
