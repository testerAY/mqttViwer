use std::thread;
use std::collections::HashMap;
use rumqttd::{Broker, Config};

pub fn start_broker() {
    thread::spawn(|| {
        let mut config = Config::default();
        config.id = 0;
        config.console.listen = "127.0.0.1:0".parse().unwrap();

        let mut router_config = rumqttd::RouterConfig::default();
        router_config.max_connections = 100; // 100接続まで許可
        router_config.max_outgoing_packet_count = 1024;
        router_config.max_segment_size = 1048576;
        router_config.max_segment_count = 10;
        config.router = router_config;
        
        let mut v4 = HashMap::new();
        v4.insert("local".to_string(), rumqttd::ServerSettings {
            name: "local".to_string(),
            listen: "127.0.0.1:9883".parse().unwrap(),
            next_connection_delay_ms: 10,
            connections: rumqttd::ConnectionSettings {
                connection_timeout_ms: 60000,
                max_payload_size: 2048,
                max_inflight_count: 100,
                auth: None,
                dynamic_filters: true,
            },
            tls: None,
        });
        
        config.v4 = v4;

        let mut broker = Broker::new(config);
        
        println!("Broker starting on 127.0.0.1:9883...");
        
        // start() がエラーを返した場合、ここで原因が表示されます
        if let Err(e) = broker.start() {
             eprintln!("Broker critical failure: {:?}", e); // println! -> eprintln! に変更
        }
    });
}
