//! Comprehensive unit and integration test suite for Pyth Hermes SSE stream client.

use equity_catalyst_pyth::{
    known_feeds, parse_price_update_event, PythStreamClient, PythStreamConfig, PythStreamError,
    RawSseEvent, SseChunkParser,
};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

/// Representative Pyth Hermes v2 parsed SSE JSON payload fixture.
const HERMES_FIXTURE_JSON: &str = r#"{
  "binary": {
    "encoding": "hex",
    "data": ["0x1234567890abcdef"]
  },
  "parsed": [
    {
      "id": "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175ec697df854ab",
      "price": {
        "price": "22450000000",
        "conf": "15000000",
        "expo": -8,
        "publish_time": 1727164800
      },
      "ema_price": {
        "price": "22445000000",
        "conf": "16000000",
        "expo": -8,
        "publish_time": 1727164800
      }
    }
  ]
}"#;

#[test]
fn test_stream_config_url_generation_dynamic_feeds() {
    let feed_1 = known_feeds::AAPL_USD;
    let feed_2 = format!("0x{}", known_feeds::TSLA_USD);

    let config = PythStreamConfig::new(
        "https://hermes.pyth.network/",
        vec![feed_1.to_string(), feed_2],
    );

    let url = config.build_url().expect("Valid URL should build");
    assert!(url.starts_with("https://hermes.pyth.network/v2/updates/price/stream?parsed=true"));
    // Ensures feed_1 is present
    assert!(url.contains(&format!("&ids[]={}", known_feeds::AAPL_USD)));
    // Ensures feed_2 is normalized (0x stripped, lowercase)
    assert!(url.contains(&format!("&ids[]={}", known_feeds::TSLA_USD)));

    // Rejects empty feed list
    let empty_config = PythStreamConfig::new("https://hermes.pyth.network", vec![]);
    let err = empty_config.build_url().unwrap_err();
    match err {
        PythStreamError::InvalidConfig(msg) => assert!(msg.contains("empty")),
        other => panic!("Expected InvalidConfig, got {:?}", other),
    }
}

#[test]
fn test_stream_config_auth_headers() {
    let config = PythStreamConfig::new("https://hermes.pyth.network", vec!["feed1".into()])
        .with_api_key("pyth_secret_key_123");

    let headers = config.build_headers().expect("Headers should build");
    assert_eq!(headers.get("Accept").unwrap(), "text/event-stream");
    assert_eq!(
        headers.get("Authorization").unwrap(),
        "Bearer pyth_secret_key_123"
    );
}

#[test]
fn test_parse_representative_hermes_fixture() {
    let raw = RawSseEvent {
        event_type: None,
        data: HERMES_FIXTURE_JSON.to_string(),
        id: Some("1".to_string()),
    };

    let event = parse_price_update_event(raw).expect("Must parse Hermes fixture successfully");

    // Verify binary field
    assert!(event.binary.is_some());
    let binary = event.binary.as_ref().unwrap();
    assert_eq!(binary.encoding.as_deref(), Some("hex"));
    assert_eq!(binary.data, vec!["0x1234567890abcdef"]);

    // Verify parsed price feeds
    assert_eq!(event.feeds().len(), 1);
    let feed = &event.feeds()[0];
    assert_eq!(feed.id, known_feeds::AAPL_USD);
    assert_eq!(feed.price.price, "22450000000");
    assert_eq!(feed.price.conf, "15000000");
    assert_eq!(feed.price.expo, -8);
    assert_eq!(feed.price.publish_time, 1727164800);

    // Verify helper lookup
    let found = event.find_feed(known_feeds::AAPL_USD).unwrap();
    assert_eq!(found.price.price, "22450000000");
}

#[test]
fn test_sse_chunk_parser_framing_and_chunk_splitting() {
    let mut parser = SseChunkParser::new();

    // Chunk 1: Partial line
    let events1 = parser.feed_str("data: {\"parsed\": [{\"id\": \"feed1\",");
    assert!(events1.is_empty(), "Partial chunk should yield no events");

    // Chunk 2: Rest of data line, but not ended with double newline
    let events2 = parser.feed_str(" \"price\": {\"price\": \"100\", \"conf\": \"1\", \"expo\": -2, \"publish_time\": 1000}}]}\n");
    assert!(
        events2.is_empty(),
        "Incomplete block should yield no events"
    );

    // Chunk 3: Final newline completing the block
    let events3 = parser.feed_str("\n");
    assert_eq!(events3.len(), 1, "Completed block must yield 1 event");

    let raw = events3.into_iter().next().unwrap().unwrap();
    let event = parse_price_update_event(raw).expect("Must parse successfully");
    assert_eq!(event.feeds().len(), 1);
    assert_eq!(event.feeds()[0].id, "feed1");
}

#[test]
fn test_sse_heartbeat_and_comments_ignored() {
    let mut parser = SseChunkParser::new();

    // SSE comments start with colon
    let sse_stream = ": ping\n: keepalive heartbeat\n\n";
    let events = parser.feed_str(sse_stream);
    assert!(events.is_empty(), "Comments must be silently ignored");

    // Named ping event
    let ping_event = "event: ping\ndata: {}\n\n";
    let ping_parsed = parser.feed_str(ping_event);
    assert_eq!(ping_parsed.len(), 1);
    let raw = ping_parsed.into_iter().next().unwrap().unwrap();
    let event = parse_price_update_event(raw).expect("Ping should parse gracefully");
    assert!(event.feeds().is_empty());
}

#[test]
fn test_malformed_json_returns_structured_error() {
    let raw = RawSseEvent {
        event_type: Some("price_update".to_string()),
        data: "{ this is invalid json }".to_string(),
        id: None,
    };

    let result = parse_price_update_event(raw);
    assert!(result.is_err(), "Malformed JSON must fail");

    match result.unwrap_err() {
        PythStreamError::MalformedJson { raw, error } => {
            assert_eq!(raw, "{ this is invalid json }");
            assert!(!error.is_empty());
        }
        other => panic!("Expected MalformedJson error, got {:?}", other),
    }
}

#[test]
fn test_unknown_event_type_returns_structured_error() {
    let raw = RawSseEvent {
        event_type: Some("unknown_custom_notification".to_string()),
        data: "{\"some\":\"payload\"}".to_string(),
        id: None,
    };

    let result = parse_price_update_event(raw);
    assert!(result.is_err(), "Unknown event type must fail");

    match result.unwrap_err() {
        PythStreamError::UnknownEvent { event, data } => {
            assert_eq!(event, "unknown_custom_notification");
            assert_eq!(data, "{\"some\":\"payload\"}");
        }
        other => panic!("Expected UnknownEvent error, got {:?}", other),
    }
}

#[test]
fn test_malformed_sse_framing_detection() {
    let mut parser = SseChunkParser::new();
    let malformed_chunk = "invalid_field_without_colon\n\n";
    let events = parser.feed_str(malformed_chunk);

    assert_eq!(events.len(), 1);
    match &events[0] {
        Err(PythStreamError::MalformedSse(msg)) => {
            assert!(msg.contains("Unrecognized SSE line format"));
        }
        other => panic!("Expected MalformedSse error, got {:?}", other),
    }
}

/// Acceptance Criteria Test:
/// Given: vec![feed_id_1, feed_id_2]
/// The client can connect and produce parsed Pyth updates.
/// No feed IDs are hardcoded in the stream implementation.
#[tokio::test]
async fn test_acceptance_criteria_end_to_end_sse_stream() {
    // 1. Bind local ephemeral mock SSE server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", local_addr);

    // Dynamic feed IDs supplied at runtime (fulfilling Task 4 & Acceptance Criteria)
    let feed_1 = known_feeds::AAPL_USD.to_string();
    let feed_2 = known_feeds::TSLA_USD.to_string();
    let dynamic_feeds = vec![feed_1.clone(), feed_2.clone()];

    // Spawn mock server task
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = socket.try_read(&mut buf);

        // Send HTTP SSE response header
        let response_headers = "HTTP/1.1 200 OK\r\n\
            Content-Type: text/event-stream\r\n\
            Cache-Control: no-cache\r\n\
            Connection: close\r\n\r\n";
        socket.write_all(response_headers.as_bytes()).await.unwrap();

        // Send Event 1 (AAPL update)
        let sse_event_1 = format!(
            "event: price_update\n\
            data: {{\"parsed\":[{{\"id\":\"{}\",\"price\":{{\"price\":\"15000000000\",\"conf\":\"10000000\",\"expo\":-8,\"publish_time\":1700000000}}}}]}}\n\n",
            known_feeds::AAPL_USD
        );
        socket.write_all(sse_event_1.as_bytes()).await.unwrap();
        socket.flush().await.unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;

        // Send Event 2 (TSLA update)
        let sse_event_2 = format!(
            "data: {{\"parsed\":[{{\"id\":\"{}\",\"price\":{{\"price\":\"25000000000\",\"conf\":\"20000000\",\"expo\":-8,\"publish_time\":1700000005}}}}]}}\n\n",
            known_feeds::TSLA_USD
        );
        socket.write_all(sse_event_2.as_bytes()).await.unwrap();
        socket.flush().await.unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;
        // Clean disconnect / EOF
        let _ = socket.shutdown().await;
    });

    // 2. Client setup
    let config = PythStreamConfig::new(base_url, dynamic_feeds).with_api_key("test_api_key");

    let client = PythStreamClient::new(config);
    let mut stream = client
        .connect()
        .await
        .expect("Stream connection should succeed");

    // 3. Receive Event 1
    let event_1 = stream
        .next_update()
        .await
        .expect("Stream should yield event 1")
        .expect("Event 1 should parse successfully");

    assert_eq!(event_1.feeds().len(), 1);
    assert_eq!(event_1.feeds()[0].id, known_feeds::AAPL_USD);
    assert_eq!(event_1.feeds()[0].price.price, "15000000000");

    // 4. Receive Event 2
    let event_2 = stream
        .next_update()
        .await
        .expect("Stream should yield event 2")
        .expect("Event 2 should parse successfully");

    assert_eq!(event_2.feeds().len(), 1);
    assert_eq!(event_2.feeds()[0].id, known_feeds::TSLA_USD);
    assert_eq!(event_2.feeds()[0].price.price, "25000000000");

    // 5. Stream cleanly terminates on server EOF
    let eof = stream.next_update().await;
    assert!(
        eof.is_none(),
        "Stream must yield None on clean EOF termination"
    );
}

#[tokio::test]
async fn test_http_failure_status_reported_as_structured_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", local_addr);

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = socket.try_read(&mut buf);

        // Send HTTP 401 Unauthorized
        let response = "HTTP/1.1 401 Unauthorized\r\n\
            Content-Type: application/json\r\n\
            Content-Length: 26\r\n\r\n\
            {\"error\":\"Invalid API key\"}";
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let config = PythStreamConfig::new(base_url, vec!["feed1".into()]);
    let client = PythStreamClient::new(config);

    let result = client.connect().await;
    assert!(result.is_err(), "Must fail on HTTP 401");

    match result.unwrap_err() {
        PythStreamError::Http { status, message } => {
            assert_eq!(status, 401);
            assert!(message.contains("Invalid API key"));
        }
        other => panic!("Expected PythStreamError::Http, got {:?}", other),
    }
}
