use std::net::ToSocketAddrs;
use std::net::SocketAddr;
use broker::auth_capnp::auth_service;
use broker::message_capnp::message_service;
use broker::topic_capnp::topic;
use broker::topic_capnp::topic_service;
use broker::util_capnp;
use broker::util_capnp::timestamp;
use capnp::capability::Promise;
use capnp_rpc::pry;
use chrono::DateTime;
use chrono::Utc;
use futures::FutureExt;
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
use uuid::Uuid;


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
    pub uuid: Uuid,
    pub name: String,
    pub creator: String,
    pub timestamp: DateTime<Utc>,
}

pub fn read_uuid(reader: util_capnp::uuid::Reader<'_>) -> Uuid {
    Uuid::from_u64_pair(
        reader.get_upper(), 
        reader.get_lower()
    )
}

pub fn read_timestamp(reader: util_capnp::timestamp::Reader<'_>) -> DateTime<Utc> {
    DateTime::from_timestamp(reader.get_seconds(), reader.get_nanos()).unwrap()
}

pub fn read_capnp_topic(reader: topic::Reader<'_>) -> Result<Topic, capnp::Error> {
    Ok(Topic {
        uuid: read_uuid(reader.get_uuid()?),
        name: reader.get_name()?.to_string()?,
        creator: reader.get_owner_username()?.to_string()?,
        timestamp: read_timestamp(reader.get_created_at()?),
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
    let message = get_message_client(&root).await?;
    let all_topics = get_all_topics(&topic).await?;
    // println!("All topics: {all_topics:?}");

    // Create topic if it does not exist
    let wanted_topic_name = "general";
    let wanted_topic_exists = all_topics.iter()
        .filter(|topic| topic.name == wanted_topic_name)
        .count() > 0;
    if !wanted_topic_exists {
        let mut request = topic.create_topic_request();
        request.get().set_name(wanted_topic_name);
        let result = request.send().promise.await?;
        assert!(result.get()?.get_topic()?.has_ok());
    }

    let all_topics = get_all_topics(&topic).await?;
    println!("All topics: {all_topics:?}");

    let wanted_topic = all_topics.iter()
        .filter(|topic| topic.name == wanted_topic_name)
        .next()
        .unwrap()
        .clone();

    println!("Requesting all messages.");
    let mut handles = vec![];
    for topic in all_topics {
        handles.push(request_existing_messages(&message, topic.uuid, topic.name.clone()));
    }
    for handle in handles {
        handle.await?;
    }

    // Do work
    do_work(message, wanted_topic.uuid).await?;

    Ok(())
}

pub fn string_error(err: String) -> capnp::Error {
    capnp::Error { kind: capnp::ErrorKind::Failed, extra: err }
}

async fn request_existing_messages(message: &message_service::Client, topic_uuid: Uuid, topic_name: String) -> Result<(), capnp::Error> {
    let mut request = message.get_messages_sync_request();
    let mut builder = request.get().init_topic_id();

    let (upper, lower) = topic_uuid.as_u64_pair();
    builder.set_upper(upper);
    builder.set_lower(lower);

    let capnp_messages = request.send()
        .promise
        .await?;

    match capnp_messages.get()?.get_messages()?.which()? {
        util_capnp::result::Which::Err(err) => {
            let err= err?;
            match err.which()? {
                message_service::error::Which::TopicDoesntExist(()) => 
                    return Err(string_error("Topic does not exist".to_string())),
                message_service::error::Which::InvalidContent(()) => 
                    return Err(string_error("Invalid content".to_string())),
            }
        },
        util_capnp::result::Which::Ok(ok) => {
            let ok = ok?;
            for message in ok.iter() {
                let author = message.get_author_name()?.to_string()?;
                let content = message.get_content()?.to_string()?;
                let timestamp = read_timestamp(message.get_timestamp()?);

                println!("[{topic_name} | {}] {author} |> {content}", timestamp.format("%Y.%m.%d %H:%M"))
            }
        },
    }

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

async fn do_work(client: message_service::Client, topic_uuid: Uuid) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = String::new();
    let mut reader = BufReader::new(tokio::io::stdin());
    loop {
        buf.clear();
        reader.read_line(&mut buf).await?;
        let trimmed = buf.trim();

        if trimmed.len() == 0 {
            continue;
        }
        if trimmed == "/q" {
            println!("Stopping...");
            break;
        }

        let mut request = client.post_message_request();
        let mut builder = request.get();
        builder.set_content(trimmed);
        let mut capnp_uuid = builder.init_topic_id();
        let (upper, lower) = topic_uuid.as_u64_pair();
        capnp_uuid.set_upper(upper);
        capnp_uuid.set_lower(lower);

        let response = request.send().promise.await?;
        let response = response.get()?.get_message()?;
        match response.which()? {
            util_capnp::result::Which::Err(err) => {
                let err = err?;
                match err.which()? {
                    message_service::error::Which::TopicDoesntExist(()) => return Err(Box::new(string_error("Topic does not exist".to_string()))),
                    message_service::error::Which::InvalidContent(()) => return Err(Box::new(string_error("Invalid content".to_string()))),
                }
            },
            util_capnp::result::Which::Ok(_) => {},
        }
        println!("Sent: {trimmed} to Topic {topic_uuid}");
    }

    Ok(())
}

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


