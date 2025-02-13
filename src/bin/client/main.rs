use std::collections::HashMap;
use std::io::{stdout, Write};
use std::net::SocketAddr;
use std::str::FromStr;

use broker::topic_capnp::topic_service;
use chrono::Duration;
use futures::future::join_all;
use network::connect_to_server;
use tokio::{io::BufReader, task::LocalSet};

use broker::message_capnp::message_service;

mod datatypes;
mod message_receiver_impl;
mod readers;
mod cli;
mod requests;
mod network;

use cli::read_line;
use datatypes::{Message, Topic};


use clap::arg;
use clap::Parser;
#[derive(Parser, Debug, Clone)]
pub struct CliArgs {
    pub username: String,

    #[arg(short, long, default_value_t = SocketAddr::from_str("127.0.0.1:8080").unwrap())]
    pub address: SocketAddr,

    #[arg(short, long)]
    pub topics: Vec<String>, 
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();
    let mut wanted_topics = args.topics;
    if wanted_topics.is_empty() {
        eprintln!("No topics specified with `--topics`. Choosing 'general' automatically.");
        wanted_topics.push("general".to_string());
    }

    LocalSet::new().run_until(run_client(args.address, args.username, &mut wanted_topics)).await?;
    Ok(())
}

async fn do_work(message_service: &message_service::Client, topic_service: &topic_service::Client, topics: &mut [Topic]) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = String::new();
    let mut reader = BufReader::new(tokio::io::stdin());

    let mut current_topic_id = 0;
    let mut key: Option<String> = None;

    loop {
        print!("\rv | {} |\n", topics[current_topic_id].name);
        stdout().flush()?;
        let line = read_line(&mut reader, &mut buf).await?;
        
        let trimmed = line.trim();

        if line.len() == 0 { // EOF
            break;
        }
        if trimmed.len() == 0 { // User entered spaces or just a newline
            continue;
        }
        {
            let mut cmd_args = trimmed.split(' ');

            match cmd_args.next().unwrap() {
                // Check for commands
                "/q" | "/stop" => break,
                "/topic" => {
                    let find_topic_id = cmd_args.next()
                        .and_then(
                            |topic_name| 
                                topics.iter()
                                    .enumerate()
                                    .filter(|(_, t)| t.name == topic_name)
                                    .map(|(i, _)| i)
                                    .next()
                        );
                    if let Some(topic_id) = find_topic_id {
                        current_topic_id = topic_id;
                    } else {
                        println!("Available topics: {:?}", topics.iter().map(|t| &t.name).collect::<Box<[_]>>())
                    }
                }
                "/key" => {
                    key = cmd_args.next().map(|x| x.to_string());
                    println!("New key set: {key:?}");
                }
                "/retention" => {
                    command_retention(topic_service, &mut topics[current_topic_id], cmd_args).await?;
                }

                _regular_message => {
                    let key_ref = key.as_ref().map(|x| x.as_str());
                    requests::post_message(message_service, trimmed, topics[current_topic_id].uuid, key_ref).await?;
                }
            }
        }
    }

    Ok(())
}

async fn command_retention(topic_service: &topic_service::Client, current_topic: &mut Topic, mut cmd_args: impl Iterator<Item = &str>) -> Result<(), capnp::Error> {
    if let Some(new_retention) = cmd_args.next() {
        // Parse retention
        let new_retention = if new_retention == "-" {
            Ok(None)
        } else {
            new_retention.parse::<f64>().map(|x| Some(x))
        };

        let new_retention = match new_retention {
            Err(e) => {
                eprintln!("Parsing error: {e:?}");
                return Ok(());
            }
            Ok(retention) => retention,
        };
        if new_retention.is_some() && new_retention.unwrap() <= 0.0 {
            eprintln!("Retention must be a positive number");
            return Ok(());
        }

        // Apply it
        current_topic.retention = new_retention.map(|x| Duration::seconds((x * 60.0) as i64));

        let result = requests::update_topic(topic_service, &*current_topic).await?;
        *current_topic = result;
    } else {
        println!("Current retention on topic '{}' is {:?}.", current_topic.name, current_topic.retention);
        println!("Set retention via `/retention -` of `/retention 12.3` in minutes.");
    }
    Ok(())
}

async fn run_client(addr: SocketAddr, username: String, wanted_topic_names: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Connect and get services
    println!("Connecting to server on {addr}");
    let (rpc_system, root_service) = connect_to_server(addr).await?;

    let topic_service = requests::get_topic_client(&root_service).await?;
    let message_service = requests::get_message_client(&root_service).await?;

    // Authorize
    requests::autorize(&root_service, &username).await?;

    // Get or create topics
    let mut topics = ensure_topics_exist(&topic_service, wanted_topic_names).await?;
    show_topics(&topics);

    // Get old messages & subscribe to new messages
    let total_history = get_history_for_topics(&message_service, &topics, 100).await?;
    print_messages(total_history.iter(), &topics);

    // Do work
    do_work(&message_service, &topic_service, &mut topics).await?;
    println!("All work done");

    rpc_system.abort();
    Ok(())
}

async fn ensure_topics_exist(topic_service: &topic_service::Client, topics: &[String]) -> Result<Vec<Topic>, capnp::Error> {
    let all_topics = requests::get_all_topics(&topic_service).await?;
    let mut results = vec![];

    let topic_by_name = |name: &str| 
        all_topics.iter()
            .filter(|topic| topic.name == name)
            .next()
            .cloned();

    for wanted_topic_name in topics {
        if let Some(topic) = topic_by_name(wanted_topic_name) {
            results.push(topic);
        } else {
            let new_topic = requests::create_topic(topic_service, &wanted_topic_name).await?;
            results.push(new_topic);
        }
    }

    Ok(results)
}

async fn get_history_for_topics(message_service: &message_service::Client, topics: &[Topic], max_messages: u32) -> Result<Vec<Message>, capnp::Error> {
    // Subscribe to all topics in parallel
    let handles = topics.iter()
        .map(|topic| {
            let topic_name = topic.name.clone();
            requests::subscribe_and_get_messages(
                &message_service, 
                topic, 
                move |message| print_message(&message, &topic_name), 
                max_messages
            )
        })
        .collect::<Vec<_>>();

    // Combine results of all requests
    let topics_histories = join_all(handles).await.into_iter().collect::<Result<Vec<_>, _>>()?;
    let mut total_history = topics_histories.into_iter().flatten().collect::<Vec<_>>();

    total_history.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    Ok(total_history)
}

// ---- Printing utilities ----

pub fn print_message(message: &Message, topic_name: &str) {
    let timestamp = &message.timestamp;
    let author = &message.author_name;
    let content = &message.content;

    println!("\r[{topic_name} | {}] {author} |> {content}", timestamp.with_timezone(&chrono::Local).format("%Y.%m.%d %H:%M:%S"))
}

fn print_messages<'a>(messages: impl Iterator<Item = &'a Message>, all_topics: &[Topic]) {
    let mut topic_names = HashMap::new();
    for topic in all_topics {
        topic_names.insert(topic.uuid, topic.name.clone());
    }

    for m in messages {
        print_message(m, &topic_names[&m.topic_uuid]);
    }
}

fn show_topics(topics: &[Topic]) {
    for topic in topics {
        println!("{}", topic);
    }    
}