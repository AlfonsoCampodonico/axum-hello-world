# Laravel Cloud has no native Rust runtime, so the app ships as an image.
# Stage 1 builds a statically linked musl binary; stage 2 is just that binary.
FROM rust:1-alpine AS build

RUN apk add --no-cache musl-dev

WORKDIR /src

# Dependencies first: this layer is cached until Cargo.toml or Cargo.lock moves.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY src ./src
COPY assets ./assets
# Cargo skips a rebuild when only mtimes changed, so nudge the real entrypoints.
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=build /src/target/release/axum-hello-world /axum-hello-world

# Informational only. The app binds $PORT when the platform injects one.
EXPOSE 9115

USER nonroot:nonroot
ENTRYPOINT ["/axum-hello-world"]
