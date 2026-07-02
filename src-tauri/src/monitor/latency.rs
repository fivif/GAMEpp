use crate::subscription::parser::ProxyNode;
use std::net::ToSocketAddrs;
use std::time::Instant;
use tokio::net::TcpStream;

/// Test TCP latency to a list of nodes concurrently
pub async fn test_nodes_latency(nodes: &[ProxyNode]) -> Vec<(String, Option<u64>)> {
    let mut tasks = Vec::new();

    for node in nodes {
        let address = node.address.clone();
        let port = node.port;
        let name = node.name.clone();

        tasks.push(tokio::spawn(async move {
            let latency = tcp_ping(&address, port).await;
            (name, latency)
        }));
    }

    let mut results = Vec::new();
    for task in tasks {
        if let Ok(result) = task.await {
            results.push(result);
        }
    }

    // Sort by latency
    results.sort_by_key(|(_, latency)| latency.unwrap_or(u64::MAX));
    results
}

/// Simple TCP ping - measure time to establish a TCP connection
async fn tcp_ping(address: &str, port: u16) -> Option<u64> {
    let addr_str = format!("{}:{}", address, port);

    // Resolve DNS
    let addrs = match tokio::task::spawn_blocking({
        let addr_str = addr_str.clone();
        move || addr_str.to_socket_addrs().ok()
    })
    .await
    .ok()?
    {
        Some(a) => a,
        None => return None,
    };

    let addr = addrs.into_iter().next()?;

    // Measure TCP connect time
    let start = Instant::now();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        TcpStream::connect(addr),
    )
    .await;

    match result {
        Ok(Ok(_stream)) => {
            let elapsed = start.elapsed();
            Some(elapsed.as_millis() as u64)
        }
        _ => None,
    }
}
