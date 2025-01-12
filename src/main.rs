use std::{fmt::Debug, io::Write};

use clap::{
    builder::{styling::Style, TypedValueParser},
    error::{ContextKind, ContextValue, ErrorKind},
    Parser, ValueEnum,
};
use config::{ConfigError, ConfigLoadable};
// use eventsource::reqwest::Client;
use reqwest::{header, Client, Url};
use serde::{Deserialize, Serialize};

mod config;
const DIMMED: Style = Style::new().dimmed();

#[derive(Debug, clap::Parser)]
#[command(about = "A CLI interface to duckduckgo's chatbots")]
struct Cli {
    #[arg(short = 'm', long = "model", value_parser=ModelIdentArgParser())]
    model: Option<ModelIdentArg>,

    #[arg(short = 's', long = "session")]
    session_name: Option<String>,
    #[arg(short = 'c', long = "continue")]
    continue_session: bool,
    #[arg(short = 'i', long = "interactive")]
    interactive_session: bool,

    #[arg(
        // last = true,
        // multiple = true,
        trailing_var_arg=true,
        required = true
    )]
    query: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum ModelIdentArg {
    GPT4oMini,
    Claude3,
    Llama3,
    Mixtral,
}

impl Default for ModelIdentArg {
    fn default() -> Self {
        ModelIdentArg::GPT4oMini
    }
}

impl ModelIdentArg {
    fn as_str(&self) -> &'static str {
        match self {
            ModelIdentArg::GPT4oMini => "gpt4o-mini",
            ModelIdentArg::Claude3 => "claude3",
            ModelIdentArg::Llama3 => "llama3",
            ModelIdentArg::Mixtral => "mistral",
        }
    }

    fn to_model(&self) -> GPTModelIdent {
        match self {
            ModelIdentArg::GPT4oMini => GPTModelIdent::GPT4oMini,
            ModelIdentArg::Claude3 => GPTModelIdent::Claude3,
            ModelIdentArg::Llama3 => GPTModelIdent::Llama3,
            ModelIdentArg::Mixtral => GPTModelIdent::Mixtral,
        }
    }
}

impl Into<&'static str> for &ModelIdentArg {
    fn into(self) -> &'static str {
        self.as_str()
    }
}

impl ValueEnum for ModelIdentArg {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::GPT4oMini, Self::Claude3, Self::Llama3, Self::Mixtral]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            ModelIdentArg::GPT4oMini => clap::builder::PossibleValue::new(self.as_str())
                .alias("gpt4o")
                .alias("gpt4"),
            ModelIdentArg::Claude3 => {
                clap::builder::PossibleValue::new(self.as_str()).alias("claude")
            }
            ModelIdentArg::Llama3 => {
                clap::builder::PossibleValue::new(self.as_str()).alias("llama")
            }
            ModelIdentArg::Mixtral => {
                clap::builder::PossibleValue::new(self.as_str()).alias("mixtral")
            }
        })
    }
}

fn rank_aliases<T>(v: &str) -> Vec<(f64, T)>
where
    T: ValueEnum + Clone + Debug,
{
    let mut candidates: Vec<_> = T::value_variants().into_iter()
        // GH #4660: using `jaro` because `jaro_winkler` implementation in `strsim-rs` is wrong
        // causing strings with common prefix >=10 to be considered perfectly similar
        .map(|variant: &T| {
            let pmatch = variant.to_possible_value().expect("ValueEnum::value_variants contains only values with a corresponding ValueEnum::to_possible_value");
            (pmatch.get_name_and_aliases().map(| alias | {
                // let res = strsim::jaro(v, alias.as_ref());
                // println!("{alias}, {res}");
                strsim::jaro(v, alias.as_ref())
            }).fold(0.0f64, |acc, v| acc.max(v)), variant.clone())
        })
        .collect();

    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    candidates
}

#[derive(Debug, Clone, Copy)]
struct ModelIdentArgParser();
impl TypedValueParser for ModelIdentArgParser {
    type Value = ModelIdentArg;
    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let ignore_case = arg.map(|a| a.is_ignore_case_set()).unwrap_or(false);
        let possible_vals = || {
            Self::Value::value_variants()
                .iter()
                .filter_map(|v| v.to_possible_value())
                .filter(|v| !v.is_hide_set())
                .map(|v| v.get_name().to_owned())
                .collect::<Vec<_>>()
        };

        let mut value = value
            .to_str()
            .ok_or_else(|| {
                invalid_value(
                    cmd,
                    value.to_string_lossy().into_owned(),
                    &possible_vals(),
                    arg.map(ToString::to_string)
                        .unwrap_or_else(|| "...".to_owned()),
                )
            })?
            .to_string();

        if ignore_case {
            value = value.to_lowercase();
        }

        let err_val = || {
            invalid_value(
                cmd,
                value.to_owned(),
                &possible_vals(),
                arg.map(ToString::to_string)
                    .unwrap_or_else(|| "...".to_owned()),
            )
        };

        let mut candidates = rank_aliases::<Self::Value>(&value);

        // dbg!(&candidates);
        let (best_score, best_match) = candidates
            .pop()
            .filter(|(score, _val)| *score > 0.8)
            .ok_or_else(|| err_val())?;

        let second_best_score = candidates.pop().map(|a| a.0).unwrap_or(0.0);

        if best_score - second_best_score < 0.1 {
            return Err(err_val());
        }

        Ok(best_match)
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = clap::builder::PossibleValue> + '_>> {
        Some(Box::new(
            ModelIdentArg::value_variants()
                .iter()
                .filter_map(|v| v.to_possible_value()),
        ))
    }
}

pub(crate) fn invalid_value(
    cmd: &clap::Command,
    bad_val: String,
    good_vals: &[String],
    arg: String,
) -> clap::error::Error {
    // let suggestion = suggestions::did_you_mean(&bad_val, good_vals.iter()).pop();
    let mut err = clap::Error::new(ErrorKind::InvalidValue).with_cmd(cmd);
    err.insert(ContextKind::InvalidArg, ContextValue::String(arg));
    err.insert(ContextKind::InvalidValue, ContextValue::String(bad_val));
    err.insert(
        ContextKind::ValidValue,
        ContextValue::Strings(good_vals.iter().map(|s| (*s).clone()).collect()),
    );

    // #[cfg(feature = "error-context")]
    // {
    //     err = err.extend_context_unchecked([
    //         (ContextKind::InvalidArg, ContextValue::String(arg)),
    //         (ContextKind::InvalidValue, ContextValue::String(bad_val)),
    //         (
    //             ContextKind::ValidValue,
    //             ContextValue::Strings(good_vals.iter().map(|s| (*s).clone()).collect()),
    //         ),
    //     ]);
    //     if let Some(suggestion) = suggestion {
    //         err = err.insert_context_unchecked(
    //             ContextKind::SuggestedValue,
    //             ContextValue::String(suggestion),
    //         );
    //     }
    // }

    err
}

struct ChunkParser<'a> {
    buf: Vec<u8>,
    delim: &'a [u8],
}

impl<'a> ChunkParser<'a> {
    fn update(&mut self, new_bytes: &[u8]) -> Vec<Vec<u8>> {
        let start = 0isize.max(self.buf.len() as isize - self.delim.len() as isize + 1);
        self.buf.extend_from_slice(new_bytes);

        let mut idx: usize = start as usize;
        let mut result = Vec::new();

        while idx <= (self.buf.len() - self.delim.len()) {
            if &self.buf[idx..idx + self.delim.len()] != self.delim {
                idx += 1;
                continue;
            }

            let new_buf = self.buf.split_off(idx + self.delim.len());
            let mut prev_chunk = std::mem::replace(&mut self.buf, new_buf);

            prev_chunk.truncate(prev_chunk.len() - self.delim.len());
            result.push(prev_chunk);
            idx = 0;

            if self.buf.len() < self.delim.len() {
                break;
            }
        }

        result
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DDGPTConfigDescription {
    default_chatbot: ModelIdentArg,
}

impl ConfigLoadable for DDGPTConfigDescription {
    const FILENAME: &'static str = "config.toml";
    const FILETYPE: config::ConfigFileType = config::ConfigFileType::TOML;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ChatRole {
    Assistant,
    User,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: ChatRole,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatRequest {
    model: GPTModelIdent,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatHistory {
    chat: ChatRequest,
    next_vqid: String,
}

struct PastChats {}
impl PastChats {
    fn load_last() -> Result<Option<ChatHistory>, ConfigError> {
        let data_path = config::user_data_dir();
        let dir_iter = match std::fs::read_dir(&data_path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir_all(&data_path)?;
                return Ok(None);
            }
            res => res,
        }?;

        let mut last_chat = None;
        for entry_result in dir_iter {
            let entry = entry_result?;
            let entry_time = entry.metadata()?.modified()?;
            match last_chat {
                Some((_, last_chat_time)) if last_chat_time > entry_time => continue,
                _ => last_chat = Some((entry, entry_time)),
            }
        }

        Ok(match last_chat {
            Some((val, _)) => Some(serde_json::from_str(&std::fs::read_to_string(val.path())?)?),
            None => None,
        })
    }

    fn load_session_from_name(name: &str) -> Result<Option<ChatHistory>, ConfigError> {
        if name.contains("/") || name.contains(".") {
            return Err(ConfigError::Io(std::io::Error::other(
                "Invalid session name!",
            )));
        }

        let mut data_path = config::user_data_dir();
        data_path.push(name);

        let data = match std::fs::read_to_string(&data_path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            val => val,
        }?;

        Ok(Some(serde_json::from_str(&data)?))
    }

    fn save(name: &str, chat: &ChatHistory) -> Result<(), ConfigError> {
        if name.contains("/") || name.contains(".") {
            return Err(ConfigError::Io(std::io::Error::other(
                "Invalid session name!",
            )));
        }

        let mut data_path = config::user_data_dir();
        // Ensure the directory exists!
        std::fs::create_dir_all(&data_path)?;
        data_path.push(name);

        let chat_serialized = serde_json::to_string(chat)?;
        std::fs::write(data_path, chat_serialized)?;

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ChatBotEvent {
    action: String,
    created: u64,
    message: Option<String>,

    // Doesn't exist for claude3
    id: Option<String>,
    model: Option<String>,

    // Nonexistent for everything but claude3
    role: Option<String>,
}

#[derive(
    Debug,
    serde::Deserialize,
    serde::Serialize, // clap::ValueEnum, Clone, Copy, PartialEq, Eq,
)]
enum GPTModelIdent {
    #[serde(rename = "gpt-4o-mini")]
    GPT4oMini,
    #[serde(rename = "claude-3-haiku-20240307")]
    Claude3,
    #[serde(rename = "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo")]
    Llama3,
    #[serde(rename = "mistralai/Mixtral-8x7B-Instruct-v0.1")]
    Mixtral,
}

fn display_message_fragment(assistant_message: &mut String, message_buffer: &[u8]) {
    let message = match message_buffer.strip_prefix(b"data: ") {
        Some(message) => message,
        None => return,
    };

    if message.starts_with(b"[DONE]") {
        let display_message = String::from_utf8_lossy(message);
        anstream::eprintln!("\n{DIMMED}{display_message}{DIMMED:#}");
        let _ = std::io::stderr().flush();
        return;
    }

    let message_deserialized: ChatBotEvent = serde_json::from_slice(message).unwrap_or_else(|a| {
        let message_printable = String::from_utf8_lossy(message).to_string();
        panic!(
            "Chatbot SSE not valid json! {:?}\n    MESSAGE: {}",
            a, message_printable
        );
    });

    if let Some(chat_message_fragment) = message_deserialized.message {
        assistant_message.push_str(&chat_message_fragment);
        print!("{chat_message_fragment}");
        let _ = std::io::stdout().flush();
    }
}

#[tokio::main]
async fn main() {
    let args_parsed = Cli::parse();
    let ddgpt_config = DDGPTConfigDescription::load()
        .expect("Could not load / access / initialize the general configuration file");
    // dbg!(&args_parsed);

    let model_arg = args_parsed.model.unwrap_or(ddgpt_config.default_chatbot);
    let model = model_arg.to_model();

    anstream::eprintln!(
        "{DIMMED}Using model: {} ({}){DIMMED:#}\n",
        model_arg.as_str(),
        serde_json::to_string(&model).expect("Model doesn't have a valid json representation!")
    );
    let _ = std::io::stderr().flush();

    let query = args_parsed.query.join(" ");
    let mut chat_history = args_parsed
        .continue_session
        .then(|| {
            if let Some(session_name) = args_parsed.session_name.as_deref() {
                PastChats::load_session_from_name(session_name)
            } else {
                PastChats::load_last()
            }
            .expect("Failed to load the previous chat, is the data directoy accessible?")
        })
        .flatten()
        .unwrap_or_else(|| ChatHistory {
            chat: ChatRequest {
                model: model,
                messages: vec![],
            },
            next_vqid: String::new(),
        });

    chat_history.chat.messages.push(ChatMessage {
        role: ChatRole::User,
        content: query,
    });

    let client = Client::builder()
        .user_agent("curl/7.81.0")
        .build()
        .expect("Failed to construct http_client");

    let mut ddg_status_request = client.get("https://duckduckgo.com/duckchat/v1/status");

    // We need to request a new session ID
    if chat_history.next_vqid.is_empty() {
        ddg_status_request = ddg_status_request.header("x-vqd-accept", "1")
    }

    let ddg_status_request = ddg_status_request.build().unwrap();

    // dbg!(&ddg_status_request);
    let ddg_status_response = client
        .execute(ddg_status_request)
        .await
        .expect("Failed to send status request");

    // dbg!(&ddg_status_response);
    // dbg!(&chat_history);
    // dbg!(&serde_json::to_string(&chat_history).unwrap());

    if chat_history.next_vqid.is_empty() {
        chat_history.next_vqid = ddg_status_response
            .headers()
            .get("x-vqd-4")
            .unwrap()
            .to_str()
            .expect("x-vqd-4 ID was not a valid UTF8-String!")
            .to_string();
    }

    // dbg!(&chat_history);
    let ddg_chat_request = client
        .post(Url::parse("https://duckduckgo.com/duckchat/v1/chat").unwrap())
        .header(header::ACCEPT, "text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        // .header(header::COOKIE, value)
        .header("x-vqd-4", &chat_history.next_vqid)
        .body(
            // r#"{"model":"gpt-4o-mini","messages":[{"content": "foobar", "role": "user"}]}"#
            serde_json::to_string(&chat_history.chat)
                .expect("Failed to json-serialize the request"),
        )
        .build()
        .unwrap();

    // println!("{}", serde_json::to_string_pretty(&chat_request).expect("Failed to json-serialize the request"));

    // dbg!(&ddg_chat_request);
    let mut ddg_chat_response = client
        .execute(ddg_chat_request)
        .await
        .expect("Failed to send chat request");

    // DDG still returns 200 even on error ...
    // if ddg_status_response.status() != 200 {
    //     eprintln!("Error {:?}", ddg_chat_response.text().await.expect("Failed to fetch message from webserver!"));
    //     return;
    // }

    let mut assistant_message = String::new();
    loop {
        let mut chunk_parser = ChunkParser {
            buf: vec![],
            delim: b"\n\n",
        };

        let messages = match ddg_chat_response.chunk().await.unwrap() {
            Some(val) => chunk_parser.update(&val),
            None => break,
        };

        for message in messages {
            display_message_fragment(&mut assistant_message, &message);
        }
    }

    if assistant_message != "" {
        chat_history.next_vqid = ddg_chat_response
            .headers()
            .get("x-vqd-4")
            .unwrap()
            .to_str()
            .expect("x-vqd-4 ID was not a valid UTF8-String!")
            .to_string();

        chat_history.chat.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: assistant_message,
        });

        PastChats::save(
            args_parsed.session_name.as_deref().unwrap_or("foobar"),
            &chat_history,
        )
        .expect("Failed to save the chat!");
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use strsim::{jaro, jaro_winkler, normalized_damerau_levenshtein, normalized_levenshtein};

//     #[test]
//     fn test_jaro() {
//         dbg!(jaro("gptclaud", "claude"));
//         dbg!(jaro("claude", "gptclaud"));

//         dbg!(jaro_winkler("gptclaud", "claude"));
//         dbg!(jaro_winkler("claude", "gptclaud"));

//         dbg!(normalized_levenshtein("gptclaud", "claude"));
//         dbg!(normalized_levenshtein("claude", "gptclaud"));
//         dbg!(normalized_levenshtein("gptclaud", "gpt"));
//         dbg!(normalized_levenshtein("gpt", "gptclaud"));
//         dbg!(normalized_levenshtein("gpt", "gtp"));

//         dbg!(normalized_damerau_levenshtein("gptclaud", "claude"));
//         dbg!(normalized_damerau_levenshtein("claude", "gptclaud"));
//         dbg!(normalized_damerau_levenshtein("gptclaud", "gpt"));
//         dbg!(normalized_damerau_levenshtein("gpt", "gptclaud"));
//         dbg!(normalized_damerau_levenshtein("gpt", "gtp"));
//     }
// }
