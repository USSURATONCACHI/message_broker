use std::net::ToSocketAddrs;
use std::net::SocketAddr;
use broker::auth_capnp::auth_service;
use broker::message_capnp::message_service;
use broker::topic_capnp::topic;
use broker::topic_capnp::topic_service;
use broker::util_capnp::timestamp;
use capnp::capability::Promise;
use capnp_rpc::pry;
use chrono::DateTime;
use chrono::Utc;
use tokio::io::BufReader;
use tokio::io::AsyncBufReadExt;

use capnp_rpc::RpcSystem;
use capnp_rpc::rpc_twoparty_capnp;
use tokio::net::TcpStream;

use broker::util::SendFuture;
use broker::util::stream_to_rpc_network;
use broker::echo_capnp::echo;
use broker::echo_capnp::ping_receiver;
use broker::main_capnp::root_service;

struct PingReceiverImpl;

impl ping_receiver::Server for PingReceiverImpl {
    fn ping(&mut self, params: ping_receiver::PingParams, _: ping_receiver::PingResults) -> Promise<(), capnp::Error> {
        let seq = pry!(params.get()).get_seq();
        println!("Server sent a ping event: {seq}");
        Promise::ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Topic {
    pub name: String,
    pub creator: String,
    pub timestamp: DateTime<Utc>,
}

pub fn read_timestamp(reader: timestamp::Reader<'_>) -> Result<DateTime<Utc>, capnp::Error> {
    Ok(DateTime::from_timestamp(reader.get_seconds(), reader.get_nanos()).unwrap())
}

pub fn read_capnp_topic(reader: topic::Reader<'_>) -> Result<Topic, capnp::Error> {
    Ok(Topic {
        name: reader.get_name()?.to_string()?,
        creator: reader.get_owner_username()?.to_string()?,
        timestamp: read_timestamp(reader.get_created_at()?)?,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = parse_cli_addr()?;

    // Connection
    println!("Connecting to server {addr}");
    let stream = TcpStream::connect(addr).await?;
    let _ = stream.set_nodelay(true);

    // RPC Init
    let network = stream_to_rpc_network(stream);
    let mut rpc_system = RpcSystem::new(Box::new(network), None);

    let root: root_service::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Client);
    tokio::spawn(SendFuture::from(rpc_system));

    // Auth
    let auth = get_auth_client(&root).await?;
    let mut login_request = auth.login_request();
    login_request.get().set_username("ussur");
    let _ = login_request.send().promise.await?;

    // Get all topics
    let topic = get_topic_client(&root).await?;
    let all_topics = get_all_topics(&topic).await?;
    println!("All topics: {all_topics:?}");

    // Create topic if it does not exist
    let wanted_topic = "general";
    let topic_exists = all_topics.iter()
        .filter(|topic| topic.name == wanted_topic)
        .count() > 0;
    if !topic_exists {
        let mut request = topic.create_topic_request();
        request.get().set_name(wanted_topic);
        let result = request.send().promise.await?;
        assert!(result.get()?.get_topic()?.has_ok());
    }

    let all_topics = get_all_topics(&topic).await?;
    println!("All topics: {all_topics:?}");


    // Do work
    // do_work(echo).await?;

    Ok(())
}

async fn get_all_topics(topic: &topic_service::Client) -> Result<Vec<Topic>, capnp::Error> {
    topic.get_all_topics_request()
        .send().promise.await?
        .get()?
        .get_topics()?
        .iter()
        .map(read_capnp_topic)
        .collect::<Result<Vec<_>, _>>()
}

async fn get_auth_client(root: &root_service::Client) -> Result<auth_service::Client, Box<dyn std::error::Error>> {
    Ok(
        root.auth_request()
            .send().promise
            .await?
            .get()?
            .get_service()?
    )
}
async fn get_echo_client(root: &root_service::Client) -> Result<echo::Client, Box<dyn std::error::Error>> {
    Ok(
        root.echo_request()
            .send().promise
            .await?
            .get()?
            .get_service()?
    )
}
async fn get_topic_client(root: &root_service::Client) -> Result<topic_service::Client, Box<dyn std::error::Error>> {
    Ok(
        root.topic_request()
            .send().promise
            .await?
            .get()?
            .get_service()?
    )
}
async fn get_message_client(root: &root_service::Client) -> Result<message_service::Client, Box<dyn std::error::Error>> {
    Ok(
        root.message_request()
            .send().promise
            .await?
            .get()?
            .get_service()?
    )
}

fn parse_cli_addr() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let addr = match std::env::args().skip(1).next() {
        Some(x) => x,
        None => {
            eprintln!("Usage: ./client <address>:<port>");
            return Err("No address provided".into());
        }
    };
    let addr = addr.to_socket_addrs()?
        .next()
        .ok_or("Provided address is invalid")?;

    Ok(addr)
}

// async fn do_work(client: echo::Client) -> Result<(), Box<dyn std::error::Error>> {
//     let mut buf = String::new();
//     let mut reader = BufReader::new(tokio::io::stdin());
//     loop {
//         buf.clear();
//         reader.read_line(&mut buf).await?;
//         let trimmed = buf.trim();

//         if trimmed.len() == 0 {
//             println!("Stopping...");
//             break;
//         }

//         let response = echo_request(&client, trimmed).await?;
//         println!("Response: {response}");
//     }

//     Ok(())
// }

// async fn echo_request(client: &echo::Client, message: &str) -> Result<String, Box<dyn std::error::Error>> {
//     let mut request = client.echo_request();
//     let mut builder = request.get().init_request();
//     builder.set_message(message);

//     let reply = request.send().promise.await?;
//     let message = reply.get()?
//         .get_reply()?
//         .get_message()?
//         .to_string()?;

//     Ok(message)
// }

// async fn subscribe_request(client: &echo::Client, ping_receiver: ping_receiver::Client) -> Result<(), Box<dyn std::error::Error>> {
//     let mut request = client.subscribe_to_pings_request();
//     request.get().set_receiver(ping_receiver);
//     request.send().promise.await?;
//     Ok(())
// }


