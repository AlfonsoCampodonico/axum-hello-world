# axum-hello-world

A hello-world HTTP API in **Rust**, built to deploy on **Laravel Cloud** using
the native Rust runtime.

It is deliberately small — a greeting, a health check, an echo — because the
interesting part is not the app. It is what a Rust web service has to get right
to run on the platform, which this repo documents and enforces in code:

- **Bind the IPv6 wildcard**, not `0.0.0.0`. Cloud's health probe arrives over
  IPv6 while the in-pod proxy forwards over IPv4 loopback, so the listener has
  to serve both. See [The dual-stack gotcha](#the-dual-stack-gotcha).
- **Honour the injected `PORT`**, falling back to `9115` locally.
- **Shut down on `SIGTERM`**, so in-flight requests finish when an instance is
  replaced.

Four direct dependencies (`axum`, `tokio`, `serde`, `serde_json`), a 1 MB
release binary that builds cold in about 15 seconds, and the OpenAPI spec
compiled into the executable so there are no files to ship beside it.

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

To reproduce what the platform does, rather than what a dev loop does:

```bash
make build               # cargo build --release --locked
make serve               # PORT=9115 target/release/axum-hello-world
```

## The dual-stack gotcha

This is the platform lesson the repo exists to record, and it bites every
runtime in this demo suite differently.

Laravel Cloud health-checks your app over **IPv6**, while the proxy in front of
it forwards requests over **IPv4 loopback**. A listener bound to `0.0.0.0` is
IPv4-only: it serves traffic fine and then fails the health check, so the
deploy never goes green.

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

CI asserts this: the `release` job boots the release binary and health-checks
it over IPv4 loopback, which only answers because the socket is dual-stack.

## Deploy to Laravel Cloud

The app builds and runs directly on Cloud's Rust runtime — there is no
Dockerfile, and nothing in the repo describes the platform. `Cargo.toml` at the
repository root is what marks this as a Rust application; the commands below
are what the environment runs.

| Setting | Value |
|---------|-------|
| Build command | `cargo build --release --locked` |
| Start command | `./target/release/axum-hello-world` |
| Deploy command | *(leave empty)* |

Cloud keeps these three separate, and the distinction matters. The **build
command** compiles. The **start command** is the long-running process that
serves HTTP — the one Rust needs set explicitly, exactly as Go, Python and
JavaScript applications do. The **deploy command** is a one-shot hook that runs
just before a release goes live, for work like database migrations; its
filesystem changes are not persisted, so nothing about building belongs there.
This app has no migrations and no release-time work, so it stays empty.

1. Create an application pointing at this repository and branch.
2. Set the build and start commands above.
3. Leave `PORT` alone — Cloud injects it and the app binds it automatically.
   Nothing else needs configuring; the app reads no other environment variable.
4. Deploy, then open `/docs` on the assigned domain and `/health` to confirm
   the probe target.

`--locked` makes the platform build the exact dependency versions in
`Cargo.lock` and fail loudly rather than silently resolving something newer.
`rust-toolchain.toml` declares the toolchain so CI and the platform build with
the same compiler rather than each picking a default.

The release profile in `Cargo.toml` trades build time for size and speed —
fat LTO, one codegen unit, symbols stripped. That is worth it here because a
cold build of this dependency tree still finishes in about 15 seconds.

## Layout

```
src/main.rs           bind the listener, serve, shut down gracefully
src/lib.rs            the router — separate from main so tests can drive it
src/routes.rs         handlers
src/error.rs          the single error shape
assets/               openapi.json and the docs page, compiled into the binary
tests/api.rs          integration tests over the router, no socket bound
rust-toolchain.toml   the compiler version CI and Cloud both build with
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
