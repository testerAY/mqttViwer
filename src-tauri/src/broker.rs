use std::collections::HashMap;
use rumqttd::{Broker, Config};
use tracing::{info, error};

// Configuration Constants
const MAX_CONNECTIONS: usize = 100;
const MAX_OUTGOING_PACKET_COUNT: u64 = 2048;
const MAX_SEGMENT_SIZE: usize = 1048576;
const MAX_SEGMENT_COUNT: usize = 10;
const CONNECTION_TIMEOUT_MS: u16 = 60000;
const MAX_PAYLOAD_SIZE: usize = 2048;
const MAX_INFLIGHT_COUNT: usize = 100;

pub async fn start_broker() {
    tauri::async_runtime::spawn_blocking(move || {
        let mut config = Config::default();
        config.id = 0;
        
        if let Some(ref mut c) = config.console {
            c.listen = "127.0.0.1:0".parse().unwrap();
        }

        let mut router_config = rumqttd::RouterConfig::default();
        router_config.max_connections = MAX_CONNECTIONS;
        router_config.max_outgoing_packet_count = MAX_OUTGOING_PACKET_COUNT;
        router_config.max_segment_size = MAX_SEGMENT_SIZE;
        router_config.max_segment_count = MAX_SEGMENT_COUNT;
        config.router = router_config;
        
        let mut v4 = HashMap::new();
        v4.insert("local".to_string(), rumqttd::ServerSettings {
            name: "local".to_string(),
            listen: "127.0.0.1:9883".parse().unwrap(),
            next_connection_delay_ms: 10,
            connections: rumqttd::ConnectionSettings {
                connection_timeout_ms: CONNECTION_TIMEOUT_MS,
                max_payload_size: MAX_PAYLOAD_SIZE,
                max_inflight_count: MAX_INFLIGHT_COUNT,
                auth: None,
                external_auth: None,
                dynamic_filters: true,
            },
            tls: None,
        });
        
        config.v4 = Some(v4);

        let mut broker = Broker::new(config);
        
        info!("Broker starting on 127.0.0.1:9883...");
        
        if let Err(e) = broker.start() {
             error!("Broker critical failure: {:?}", e);
        }
    });
}
