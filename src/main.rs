#[macro_use]
extern crate serde_derive;

use warp::{
    *,
    body::FullBody,
};
use std::{
    env,
    sync::RwLock,
    collections::HashMap,
    time::Instant,
    time::Duration,
    sync::Arc,
    net::SocketAddr,
};
use rdkafka::{
    ClientConfig,
    producer::{
        FutureProducer,
        FutureRecord,
    },
    consumer::{
        StreamConsumer,
        Consumer,
        CommitMode,
    },
    config::RDKafkaLogLevel,
    message::Message,
};
use ::log::*;
use tokio::timer;
use std::thread::JoinHandle;
use exitfailure::ExitFailure;
use failure::ResultExt;
use failure::Error;

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

fn main() -> Result<(), ExitFailure> {
    env_logger::init();
    info!("Starting notary-endpoint!");

    // small macro to get nicer error messages
    macro_rules! get_env {
        ($env_name: literal) => {
            env::var($env_name).context(concat!("getting environment variable ", $env_name))?
        };
    }

    let brokers = get_env!("KAFKA_BROKERS");
    let request_topic = get_env!("REQUEST_TOPIC");
    let response_topic = get_env!("RESPONSE_TOPIC");
    let error_topic = get_env!("ERROR_TOPIC");
    let explorer_url = get_env!("EXPLORER_URL");
    let bind_address = get_env!("BIND_ADDR");

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("produce.offset.report", "true")
        .set("message.timeout.ms", "5000")
        .create()
        .context("creating kafka producer")?;

    let ethereum_service_responses = Arc::new(RwLock::new(HashMap::<String, String>::new()));

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
        .and(body::concat().map(|b: FullBody| hex::encode(b.collect::<Vec<_>>())))
        .and_then(move |encoded_body: String| {
            let record = FutureRecord::to(&request_topic).payload(&encoded_body).key(&encoded_body);
            let timeout = 0;
            producer.send(record, timeout).map_err(reject::custom).and_then(|res| {
                res.map(|_| encoded_body)
                    .map_err(|(kafka_err, _)| reject::custom(kafka_err))
            })
        }).and_then(|encoded_body|
            timer::Delay::new(Instant::now() + Duration::from_secs(1)).map(|_| encoded_body)
                .map_err(reject::custom)
        ).map(get_response_for_encoded_message.clone());

    let check = path!("check" / String).map(get_response_for_encoded_message);

    let consumer_thread_handle = start_kafka_consumer_thread(&brokers, &response_topic, &error_topic, ethereum_service_responses)?;

    serve(check.or(submit)).run(
        bind_address.parse::<SocketAddr>()
            .with_context(|_| format!("parsing {}", bind_address))?
    );

    consumer_thread_handle.join().unwrap();

    Ok(())
}

fn log_error<F>(f: F) where F: FnOnce() -> Result<(), Error> {
    match f() {
        Err(e) => error!("{}", e),
        _ => ()
    }
}

fn start_kafka_consumer_thread(
    brokers: &str,
    response_topic: &str,
    error_topic: &str,
    ethereum_service_responses: Arc<RwLock<HashMap<String, String>>>,
) -> Result<JoinHandle<()>, Error> {
    info!("Starting kafka consumer thread");
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "ubirch-notary-endpoint")
        .set("bootstrap.servers", &brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create()
        .context("creating kafka consumer")?;

    let topics = &[response_topic, error_topic];
    consumer.subscribe(topics)
        .with_context(|_| format!("subscribing to topics: {:?}", topics))?;

    let join_handle = std::thread::spawn(move || {
        let process_responses = consumer.start().for_each(|message| {
            log_error(|| {
                let message = message?;
                consumer.commit_message(&message, CommitMode::Async)?;
                let payload = message.payload().ok_or(failure::err_msg("no payload"))?;
                let payload: EtheriumServiceResponse = serde_json::from_slice(payload)?;

                ethereum_service_responses.write().unwrap().insert(payload.message, payload.txid);
                Ok(())
            });

            Ok(())
        });

        tokio::executor::current_thread::block_on_all(process_responses)
            .expect("Error while processing responses");
    });

    Ok(join_handle)
}
