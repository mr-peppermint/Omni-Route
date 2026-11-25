//! Short, per-link throughput check used to give idle speed meters a useful
//! connection baseline. It runs once when a link comes online and is capped
//! by both time and bytes so reconnects cannot trigger an unbounded download.

use crate::download::http_client::{empty_body, empty_request, host_port_https, send_over_tcp};
use crate::network::connector::LinkConnector;
use http_body_util::BodyExt;
use hyper::header::RANGE;
use hyper::{Method, StatusCode, Uri};
use std::net::IpAddr;
use std::time::{Duration, Instant};

const TEST_URL: &str = "https://ash-speed.hetzner.com/100MB.bin";
const MAX_TEST_BYTES: u64 = 10 * 1024 * 1024;
const MIN_USEFUL_BYTES: u64 = 128 * 1024;
const BODY_WINDOW: Duration = Duration::from_secs(4);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(12);

pub async fn measure_link_speed(if_index: u32, local_ip: IpAddr) -> Option<f64> {
    tokio::time::timeout(TOTAL_TIMEOUT, async move {
        let uri: Uri = TEST_URL.parse().ok()?;
        let (host, port, is_https) = host_port_https(&uri).ok()?;
        let connector = LinkConnector::Bound { if_index, local_ip };
        let tcp = connector.connect(&host, port).await.ok()?;
        let request = empty_request(Method::GET, &uri)
            .ok()?
            .header(RANGE, format!("bytes=0-{}", MAX_TEST_BYTES - 1))
            .body(empty_body())
            .ok()?;
        let response = send_over_tcp(tcp, &host, is_https, request).await.ok()?;
        if !matches!(response.status(), StatusCode::OK | StatusCode::PARTIAL_CONTENT) {
            return None;
        }

        let mut body = response.into_body();
        let started = Instant::now();
        let deadline = tokio::time::Instant::now() + BODY_WINDOW;
        let mut bytes = 0u64;
        while bytes < MAX_TEST_BYTES {
            match tokio::time::timeout_at(deadline, body.frame()).await {
                Ok(Some(Ok(frame))) => {
                    if let Ok(chunk) = frame.into_data() {
                        bytes = bytes.saturating_add(chunk.len() as u64).min(MAX_TEST_BYTES);
                    }
                }
                Ok(Some(Err(_))) => return None,
                Ok(None) | Err(_) => break,
            }
        }

        let elapsed = started.elapsed().as_secs_f64();
        if bytes < MIN_USEFUL_BYTES || elapsed <= 0.0 {
            return None;
        }
        Some(bytes as f64 * 8.0 / elapsed / 1_000_000.0)
    })
    .await
    .ok()
    .flatten()
}
