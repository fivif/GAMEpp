use game_pp::subscription::{fetcher, parser};

#[tokio::main]
async fn main() {
    let url = "https://pro.dl.214578.xyz/sub?token=93a1f80da0c32c368c16218efe122497";
    
    println!("=== Fetching subscription ===");
    match fetcher::fetch_subscription(url).await {
        Ok(content) => {
            println!("Fetched {} bytes", content.len());
            println!("First 200 chars: {}", &content[..200.min(content.len())]);
            println!("Total lines: {}", content.lines().count());
            
            println!("\n=== Parsing nodes ===");
            let nodes = parser::parse_node_list(&content);
            println!("Parsed {} nodes", nodes.len());
            
            for node in nodes.iter().take(5) {
                println!("  - {} | {}:{} | latency={:?}", node.name, node.address, node.port, node.latency_ms);
            }
        }
        Err(e) => {
            println!("FETCH ERROR: {}", e);
        }
    }
}
