use std::collections::HashMap;
use std::io::{stdout, Write};
use std::net::SocketAddr;

use broker::topic_capnp::topic_service;
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
    pub address: SocketAddr,
    pub username: String,

    #[arg(short, long)]
    pub topics: Vec<String>, 
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();
    let wanted_topics = args.topics;

    LocalSet::new().run_until(run_client(args.address, args.username, &wanted_topics)).await?;
    Ok(())
}

async fn do_work(message_service: &message_service::Client, topics: &[Topic]) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = String::new();
    let mut reader = BufReader::new(tokio::io::stdin());

    let mut current_topic = &topics[0];

    loop {
        print!("\rv | {} |\n", current_topic.name);
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
                    let find_topic = cmd_args.next().and_then(|topic_name| topics.iter().filter(|t| t.name == topic_name).next());
                    if let Some(topic) = find_topic {
                        current_topic = topic;
                    } else {
                        println!("Available topics: {:?}", topics.iter().map(|t| &t.name).collect::<Box<[_]>>())
                    }
                }
                _regular_message => {
                    requests::post_message(message_service, trimmed, current_topic.uuid).await?;
                }
            }
        }
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
    let topics = ensure_topics_exist(&topic_service, wanted_topic_names).await?;
    show_topics(&topics);

    // Get old messages & subscribe to new messages
    let total_history = get_history_for_topics(&message_service, &topics, 100).await?;
    print_messages(total_history.iter(), &topics);

    // Do work
    do_work(&message_service, &topics).await?;
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