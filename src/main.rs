#[macro_use]
extern crate serde_derive;

use warp::*;
use std::io::Read;
use std::error::Error;
use std::env;
use rdkafka::{
    ClientConfig,
    producer::FutureProducer,
    producer::FutureRecord,
};
use warp::body::FullBody;
use rdkafka::consumer::StreamConsumer;
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::consumer::Consumer;
use ::log::*;
use rdkafka::message::Message;
use std::sync::RwLock;
use std::collections::HashMap;
use std::time::Instant;
use std::time::Duration;
use tokio::timer;
use std::sync::Arc;
use std::net::SocketAddr;
use rdkafka::consumer::CommitMode;

#[derive(Serialize, Deserialize)]
enum NotaryResponse {
    Ok { explorer_url: String },
    CheckAgain { url: String },
    Error { message: String },
}

#[derive(Serialize, Deserialize)]
struct EtheriumServiceResponse {
    txid: String,
    message: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting notary-endpoint!");

    let brokers = env::var("KAFKA_BROKERS")?;
    let request_topic = env::var("REQUEST_TOPIC")?;
    let response_topic = env::var("RESPONSE_TOPIC")?;
    let error_topic = env::var("ERROR_TOPIC")?;
    let explorer_url = env::var("EXPLORER_URL")?;
    let bind_address = env::var("BIND_ADDR")?;

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("produce.offset.report", "true")
        .set("message.timeout.ms", "5000")
        .create()?;

    let ethereum_service_responses = Arc::new(RwLock::new(HashMap::<String, String>::new()));

    let hex_encoded_body = body::concat().and_then(|fb: FullBody| -> Result<_, Rejection> {
        let mut body_str = String::new();
        fb.reader().read_to_string(&mut body_str).map_err(reject::custom)?;

        Ok(hex::encode(body_str))
    });

    let get_response_for_encoded_message = {
        let ethereum_service_responses = Arc::clone(&ethereum_service_responses);

        move |message| {
            if let Some(txid) = ethereum_service_responses.read().unwrap().get(&message) {
                reply::json(&NotaryResponse::Ok { explorer_url: format!("{}/{}", explorer_url, txid) })
            } else {
                reply::json(&NotaryResponse::CheckAgain { url: format!("/check/{}", message) })
            }
        }
    };

    let submit = path::end()
        .and(post2())
        .and(hex_encoded_body)
        .and_then(move |encoded_body: String| {
            let record = FutureRecord::to(&request_topic).payload(&encoded_body).key(&encoded_body);
            let timeout = 0;
            producer.send(record, timeout)
                .map_err(reject::custom)
                .and_then(|res| {
                    res.map(|_| encoded_body)
                        .map_err(|(kafka_err, _)| reject::custom(kafka_err))
                })
        }).and_then(|encoded_body| {
            // wait a sec and check if the response's ready
            timer::Delay::new(Instant::now() + Duration::from_secs(1))
                .map(|_| encoded_body)
                .map_err(reject::custom)
        }).map(get_response_for_encoded_message.clone());

    let check = path!("check" / String).map(get_response_for_encoded_message);

    let consumer_thread_handle = std::thread::spawn(move || {
        info!("Starting kafka consumer thread");
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "ubirch-notary-endpoint")
            .set("bootstrap.servers", &brokers)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set_log_level(RDKafkaLogLevel::Debug)
            .create()
            .expect("Could not create kafka consumer");

        consumer.subscribe(&[&response_topic, &error_topic])
            .expect("Could not subscribe to given topics");

        let process_responses = consumer.start().for_each(|message| {
            log_error(|| {
                let message = message?;
                consumer.commit_message(&message, CommitMode::Async)?;
                let payload = message.payload().ok_or("no payload")?;
                let payload: EtheriumServiceResponse = serde_json::from_slice(payload)?;

                ethereum_service_responses.write().unwrap().insert(payload.message, payload.txid);
                Ok(())
            });

            Ok(())
        });

        tokio::executor::current_thread::block_on_all(process_responses)
            .expect("Error while processing responses");
    });

    serve(check.or(submit)).run(bind_address.parse::<SocketAddr>()?);
    consumer_thread_handle.join().unwrap();

    Ok(())
}

fn log_error<F>(f: F) where F: FnOnce() -> Result<(), Box<dyn Error>> {
    match f() {
        Err(e) => error!("{}", e),
        _ => ()
    }
}
