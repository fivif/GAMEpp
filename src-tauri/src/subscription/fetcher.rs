use anyhow::Result;

/// Fetch a subscription from a URL and decode it
/// Subscription URLs return base64-encoded text with one proxy URL per line
pub async fn fetch_subscription(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let response = client
        .get(url)
        .header("User-Agent", "GAME++/0.1")
        .send()
        .await?;

    let body = response.text().await?;

    // Try base64 decode
    if let Ok(decoded) = base64_decode(&body) {
        if decoded.contains("://") {
            return Ok(decoded);
        }
    }

    // If not base64 or no valid URLs, return raw body
    Ok(body)
}

fn base64_decode(input: &str) -> Result<String> {
    use base64::Engine;
    let input = input.trim().replace('-', "+").replace('_', "/");
    let padding = (4 - (input.len() % 4)) % 4;
    let padded = input + &"=".repeat(padding);
    let bytes = base64::engine::general_purpose::STANDARD.decode(padded)?;
    Ok(String::from_utf8(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_real_subscription() {
        let url = "https://pro.dl.214578.xyz/sub?token=93a1f80da0c32c368c16218efe122497";
        let result = fetch_subscription(url).await;
        match &result {
            Ok(content) => {
                println!("Fetched {} bytes", content.len());
                println!("First line: {}", content.lines().next().unwrap_or("EMPTY"));
                println!("Total lines: {}", content.lines().count());
                // Verify it contains VLESS URLs
                assert!(content.contains("vless://"), "Content should contain vless URLs");
                assert!(content.lines().count() > 10, "Should have many nodes");
            }
            Err(e) => {
                println!("FETCH ERROR: {:?}", e);
                panic!("Fetch failed: {}", e);
            }
        }
    }
}
