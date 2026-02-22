use chrono::{DateTime, Utc};
use influxdb::{Client, InfluxDbWriteable};
use std::env;

#[derive(InfluxDbWriteable)]
struct StockMeasure {
    time: DateTime<Utc>,
    ask: f64,
    bid: f64,
    #[influxdb(tag)]
    ticker: String,
}

const INFLUXDB: &str = "stonks";
const STOCKS: &str = "stocks";

#[tokio::main]
async fn main() {
    println!("Starting influxdb test");
    let token = env::var("INFLUXDB3_AUTH_TOKEN").unwrap();
    let client = Client::new("http://localhost:8181", INFLUXDB).with_token(token);

    for _ in 0..100 {
        let client = client.clone();
        let measure = StockMeasure {
            time: Utc::now(),
            ask: 14.0,
            bid: 10.59,
            ticker: "RIVN".to_string(),
        };
        let m2 = StockMeasure {
            time: Utc::now(),
            ask: 14.0,
            bid: 10.59,
            ticker: "AVX".to_string(),
        };

        let queries = vec![
            measure.try_into_query(STOCKS).unwrap(),
            m2.try_into_query(STOCKS).unwrap(),
        ];

        match client.query(queries).await {
            Ok(_s) => {}
            Err(e) => println!("Err: {e}"),
        }
    }
}
