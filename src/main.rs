use std::{fmt::Debug, io::Write};

use clap::{
    builder::{styling::Style, TypedValueParser},
    error::{ContextKind, ContextValue, ErrorKind},
    Parser, ValueEnum,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
// use eventsource::reqwest::Client;
use reqwest::{header, Client, Url};
use serde_json::json;

const DIMMED: Style = Style::new().dimmed();

#[derive(Debug, clap::Parser)]
#[command(about = "A CLI interface to duckduckgo's chatbots")]
struct Cli {
    #[arg(short = 'm', long = "model", default_value = "gpt4o-mini", value_parser=ModelIdentArgParser())]
    model: ModelIdentArg,
    #[arg(
        // last = true,
        // multiple = true,
        trailing_var_arg=true,
        required = true
    )]
    query: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum ModelIdentArg {
    GPT4oMini,
    Claude3,
    Llama3,
    Mixtral,
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

    fn from_str(input: &str, ignore_case: bool) -> Result<Self, String> {
        let mut input_normalized = input.to_string();
        if !ignore_case {
            input_normalized = input.to_lowercase();
        }

        let matcher = SkimMatcherV2::default();
        for variant in Self::value_variants() {
            let pattern: &'static str = variant.into();
            let score = matcher.fuzzy_match(input_normalized.as_str(), pattern);

            dbg!(score);
        }

        Err(format!("Invalid variant: {input}"))
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

fn display_message_fragment(message_buffer: &[u8]) {
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
        print!("{chat_message_fragment}");
        let _ = std::io::stdout().flush();
    }
}

#[tokio::main]
async fn main() {
    let args_parsed = Cli::parse();
    // dbg!(&args_parsed);

    let model = args_parsed.model.to_model();
    let query = args_parsed.query.join(" ");
    anstream::eprintln!(
        "{DIMMED}Using model: {} ({}){DIMMED:#}\n",
        args_parsed.model.as_str(),
        serde_json::to_string(&model).expect("Model doesn't have a valid json representation!")
    );
    let _ = std::io::stderr().flush();

    let client = Client::builder()
        .user_agent("curl/7.81.0")
        .build()
        .expect("Failed to construct http_client");

    let ddg_status_request = client
        .get("https://duckduckgo.com/duckchat/v1/status")
        .header("x-vqd-accept", "1")
        .build()
        .unwrap();

    // dbg!(&ddg_status_request);
    let ddg_status_response = client
        .execute(ddg_status_request)
        .await
        .expect("Failed to send status request");

    // dbg!(&ddg_status_response);

    let header_x_vqd_4_id = ddg_status_response.headers().get("x-vqd-4").unwrap();
    let ddg_chat_request = client
        .post(Url::parse("https://duckduckgo.com/duckchat/v1/chat").unwrap())
        .header(header::ACCEPT, "text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        // .header(header::COOKIE, value)
        .header("x-vqd-4", header_x_vqd_4_id)
        .body(
            json!(
            {
                "model": model,
                "messages":[{"content": query, "role": "user"}]
            })
            .to_string(),
            // r#"{"model":"gpt-4o-mini","messages":[{"content": "foobar", "role": "user"}]}"#
        )
        .build()
        .unwrap();

    // dbg!(&ddg_chat_request);
    let mut ddg_chat_response = client
        .execute(ddg_chat_request)
        .await
        .expect("Failed to send chat request");

    loop {
        let mut chunk_parser = ChunkParser {
            buf: vec![],
            delim: b"\n\n",
        };

        let messages = match ddg_chat_response.chunk().await.unwrap() {
            Some(val) => {
                // dbg!(&val);
                chunk_parser.update(&val)
            }
            None => break,
        };

        for message in messages {
            display_message_fragment(&message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use strsim::{jaro, jaro_winkler, normalized_damerau_levenshtein, normalized_levenshtein};

    // #[test]
    // fn test_jaro() {
    //     dbg!(jaro("gptclaud", "claude"));
    //     dbg!(jaro("claude", "gptclaud"));

    //     dbg!(jaro_winkler("gptclaud", "claude"));
    //     dbg!(jaro_winkler("claude", "gptclaud"));

    //     dbg!(normalized_levenshtein("gptclaud", "claude"));
    //     dbg!(normalized_levenshtein("claude", "gptclaud"));
    //     dbg!(normalized_levenshtein("gptclaud", "gpt"));
    //     dbg!(normalized_levenshtein("gpt", "gptclaud"));
    //     dbg!(normalized_levenshtein("gpt", "gtp"));

    //     dbg!(normalized_damerau_levenshtein("gptclaud", "claude"));
    //     dbg!(normalized_damerau_levenshtein("claude", "gptclaud"));
    //     dbg!(normalized_damerau_levenshtein("gptclaud", "gpt"));
    //     dbg!(normalized_damerau_levenshtein("gpt", "gptclaud"));
    //     dbg!(normalized_damerau_levenshtein("gpt", "gtp"));
    // }
}
