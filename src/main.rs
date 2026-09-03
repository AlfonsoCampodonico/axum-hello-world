//! Binds the listener and serves the router until the platform asks us to stop.

use std::{
    env, io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
};

use axum_hello_world::app;
use tokio::{net::TcpListener, signal};

/// Used when the platform does not inject a `PORT`.
const DEFAULT_PORT: u16 = 9115;

#[tokio::main]
async fn main() -> io::Result<()> {
    let port = port_from_env();
    let listener = bind(port).await?;

    println!(
        "axum-hello-world listening on http://{} — docs at /docs",
        listener.local_addr()?
    );

    axum::serve(listener, app())
        .with_graceful_shutdown(shutdown_signal())
        .await
}

/// Reads `PORT`, then `ADDR`, then falls back to [`DEFAULT_PORT`]. A value that
/// is set but unparseable is a misconfiguration worth failing loudly on, but a
/// demo app that refuses to boot is worse, so we warn and use the default.
fn port_from_env() -> u16 {
    let Some((name, raw)) = ["PORT", "ADDR"]
        .into_iter()
        .find_map(|name| env::var(name).ok().map(|raw| (name, raw)))
    else {
        return DEFAULT_PORT;
    };

    // `ADDR` is conventionally written as ":9115" or "0.0.0.0:9115"; we only
    // ever bind the wildcard address, so the port is the part that matters.
    let port = raw.rsplit(':').next().unwrap_or_default().trim();

    match port.parse() {
        Ok(port) => port,
        Err(_) => {
            eprintln!("warning: {name}={raw:?} is not a port; falling back to {DEFAULT_PORT}");
            DEFAULT_PORT
        }
    }
}

/// Binds the IPv6 wildcard, which on Linux and macOS also accepts IPv4 through
/// v4-mapped addresses because `IPV6_V6ONLY` defaults to off and neither std
/// nor tokio overrides it. That dual-stack listener is the whole trick for
/// Laravel Cloud: its health probe arrives over IPv6 while the in-pod proxy
/// forwards over IPv4 loopback, so binding `0.0.0.0` alone fails the probe.
/// Hosts built without IPv6 support at all fall back to IPv4.
async fn bind(port: u16) -> io::Result<TcpListener> {
    let dual_stack = SocketAddr::from((Ipv6Addr::UNSPECIFIED, port));

    match TcpListener::bind(dual_stack).await {
        Ok(listener) => Ok(listener),
        Err(error) => {
            eprintln!("warning: could not bind {dual_stack} ({error}); trying IPv4 only");
            TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, port))).await
        }
    }
}

/// Resolves on Ctrl-C or on the SIGTERM the platform sends before replacing a
/// container, letting in-flight requests finish.
async fn shutdown_signal() {
    let interrupt = async {
        signal::ctrl_c().await.expect("ctrl-c handler installs");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler installs")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = interrupt => println!("interrupted; shutting down"),
        () = terminate => println!("SIGTERM received; shutting down"),
    }
}
