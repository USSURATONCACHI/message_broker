use std::io::stdout;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::net::SocketAddr;
use std::str::from_utf8_unchecked;
use broker::auth_capnp::auth_service;
use broker::message_capnp;
use broker::message_capnp::message_receiver;
use broker::message_capnp::message_receiver::ReceiveParams;
use broker::message_capnp::message_service;
use broker::topic_capnp::topic;
use broker::topic_capnp::topic_service;
use broker::util_capnp;
use capnp::capability::Promise;
use capnp_rpc::pry;
use chrono::DateTime;
use chrono::Utc;
use tokio::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;

use capnp_rpc::RpcSystem;
use capnp_rpc::rpc_twoparty_capnp;
use tokio::net::TcpStream;

use broker::util::stream_to_rpc_network;
use broker::main_capnp::root_service;
use tokio::task::LocalSet;
use uuid::Uuid;


pub fn print_message(message: &Message, topic_name: &str) {
    // let topic_uuid = &message.topic_uuid;
    let timestamp = &message.timestamp;
    let author = &message.author_name;
    let content = &message.content;

    print!("\r[{topic_name} | {}] {author} |> {content}\n*> ", timestamp.format("%Y.%m.%d %H:%M:%S"))
}

pub struct MessageReceiver {
    pub topic_name: String,
}

impl message_receiver::Server for MessageReceiver {
    fn receive(&mut self, params: ReceiveParams) -> Promise<(), capnp::Error> {
        let reader = pry!(params.get());
        
        let message = pry!(reader.get_message());
        let topic_uuid = read_capnp_uuid(pry!(message.get_topic_uuid()));
        let message = pry!(read_capnp_message(message, topic_uuid));
        
        print_message(&message, &self.topic_name);
        stdout().flush().unwrap();
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

#[derive(Clone, Debug, Default)]
pub struct Message {
    pub uuid: Uuid,
    pub topic_uuid: Uuid,
    pub author_name: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}



pub fn read_capnp_uuid(reader: util_capnp::uuid::Reader<'_>) -> Uuid {
    Uuid::from_u64_pair(
        reader.get_upper(), 
        reader.get_lower()
    )
}

pub fn read_capnp_message(reader: message_capnp::message::Reader<'_>, topic_uuid: Uuid) -> Result<Message, capnp::Error> {

    Ok(Message {
        uuid: read_capnp_uuid(reader.reborrow().get_uuid()?),
        topic_uuid,
        author_name: reader.reborrow().get_author_name()?.to_string()?,
        content: reader.reborrow().get_content()?.to_string()?,
        timestamp: read_capnp_timestamp(reader.reborrow().get_timestamp()?),
    })
}

pub fn read_capnp_timestamp(reader: util_capnp::timestamp::Reader<'_>) -> DateTime<Utc> {
    DateTime::from_timestamp(reader.get_seconds(), reader.get_nanos()).unwrap()
}

pub fn read_capnp_topic(reader: topic::Reader<'_>) -> Result<Topic, capnp::Error> {
    Ok(Topic {
        uuid: read_capnp_uuid(reader.get_uuid()?),
        name: reader.get_name()?.to_string()?,
        creator: reader.get_owner_username()?.to_string()?,
        timestamp: read_capnp_timestamp(reader.get_created_at()?),
    })
}





#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (addr, username) = parse_cli_args()?;

    // Connection
    LocalSet::new().run_until(start_client(addr, username)).await?;
    Ok(())
}

async fn start_client(addr: SocketAddr, username: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to server {addr}");
    let stream = TcpStream::connect(addr).await?;
    let _ = stream.set_nodelay(true);

    // RPC Init
    let network = stream_to_rpc_network(stream);
    let mut rpc_system = RpcSystem::new(Box::new(network), None);

    let root: root_service::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Client);
    let rpc_handle = tokio::task::spawn_local(rpc_system);

    // Auth
    let auth = get_auth_client(&root).await?;
    let mut login_request = auth.login_request();
    login_request.get().set_username(username);
    let _ = login_request.send().promise.await?;

    // Get all topics
    let topic = get_topic_client(&root).await?;
    let message = get_message_client(&root).await?;
    let all_topics = get_all_topics(&topic).await?;

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

    // Getting wanted topic
    let wanted_topic = all_topics.iter()
        .filter(|topic| topic.name == wanted_topic_name)
        .next()
        .unwrap()
        .clone();
    
    // Requesting all messages
    {
        let mut handles = vec![];
        for topic in all_topics {
            handles.push(request_existing_messages(&message, topic.uuid, topic.name.clone()));
        }
        for handle in handles {
            handle.await?;
        }
    }

    // Subscribing to live messages
    let receiver = MessageReceiver { topic_name: wanted_topic_name.to_string() };
    let receiver_client: message_receiver::Client = capnp_rpc::new_client(receiver);
    {
        let mut subscribe_request = message.subscribe_request();
        let mut builder = subscribe_request.get();
        builder.set_receiver(receiver_client);
        let mut uuid_builder = builder.init_topic_id();
        let (upper, lower) = wanted_topic.uuid.as_u64_pair();
        uuid_builder.set_upper(upper);
        uuid_builder.set_lower(lower);
        subscribe_request.send().promise.await?;
    }

    // Do work
    do_work(message, wanted_topic.uuid).await?;
    println!("All work done");

    rpc_handle.abort();
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
                message_service::error::Which::EntityDoesNotExist(()) => 
                    return Err(string_error("Topic does not exist".to_string())),
                message_service::error::Which::InvalidContent(()) => 
                    return Err(string_error("Invalid content".to_string())),
            }
        },
        util_capnp::result::Which::Ok(ok) => {
            let ok = ok?;
            for message in ok.iter() {
                print_message(&read_capnp_message(message, topic_uuid)?, &topic_name);
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

fn print_usage() {
    eprintln!("Usage: ./client <address>:<port> <username>");
}

fn parse_cli_args() -> Result<(SocketAddr, String), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let addr = match args.next() {
        Some(x) => x,
        None => {
            print_usage();
            return Err("No address provided".into());
        }
    };
    let addr = addr.to_socket_addrs()?
        .next()
        .ok_or("Provided address is invalid")?;

    let username = match args.next() {
        Some(x) => x,
        None => {
            print_usage();
            return Err("No username provided".into());
        },
    };

    Ok((addr, username))
}

async fn read_line(reader: &mut (impl AsyncRead + Unpin), buffer: &mut String) -> io::Result<String> {
    let mut bytes: [u8; 1024] = [0; 1024];

    while !buffer.contains('\n') {
        let was_read = reader.read(&mut bytes).await?;
        buffer.push_str(unsafe { from_utf8_unchecked(&bytes[0..was_read]) });

        if was_read == 0 {
            let result = buffer.clone();
            buffer.clear();
            return Ok(result);
        }
    }

    match buffer.find('\n') {
        Some(newline_pos) => {
            let result = buffer[0..=newline_pos].to_owned();
            *buffer = buffer[newline_pos + 1 ..].to_owned();
            Ok(result)
        }
        None => {
            let result = buffer.clone();
            buffer.clear();
            Ok(result)
        }
    }
}

async fn do_work(client: message_service::Client, topic_uuid: Uuid) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = String::new();
    let mut reader = BufReader::new(tokio::io::stdin());
    loop {
        print!("\r|> ");
        stdout().flush()?;
        let line = read_line(&mut reader, &mut buf).await?;
        
        let trimmed = line.trim();

        if line.len() == 0 {
            break;
        }
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
                    message_service::error::Which::EntityDoesNotExist(()) => return Err(Box::new(string_error("Topic does not exist".to_string()))),
                    message_service::error::Which::InvalidContent(()) => return Err(Box::new(string_error("Invalid content".to_string()))),
                }
            },
            util_capnp::result::Which::Ok(_) => {},
        }
        // println!("Sent: {trimmed} to Topic {topic_uuid}");
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


