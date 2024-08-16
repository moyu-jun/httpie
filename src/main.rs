use std::{collections::HashMap, str::FromStr};

use anyhow::{anyhow, Result};
use clap::Parser;
use colored::Colorize;
use mime::Mime;
use reqwest::{header, Client, Response, Url};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

/// A native httpie implementation with Rust.
#[derive(Parser, Debug)]
#[command(version = "1.0.0", author = "Moyu <moyubit@gmail.com>")]
struct Cli {
    #[command(subcommand)]
    command: Option<HttpMethod>,
}

#[derive(Parser, Debug)]
enum HttpMethod {
    Get(Get),
    Post(Post),
    Put(Put),
    Delete(Delete),
}

#[derive(Parser, Debug)]
struct Get {
    #[arg(value_parser = parse_url)]
    url: String,
}

#[derive(Parser, Debug)]
struct Post {
    #[arg(value_parser = parse_url)]
    url: String,
    #[arg(value_parser = parse_key_value)]
    body: Vec<KeyValue>,
}

#[derive(Parser, Debug)]
struct Put {
    #[arg(value_parser = parse_url)]
    url: String,
    #[arg(value_parser = parse_key_value)]
    body: Vec<KeyValue>,
}

#[derive(Parser, Debug)]
struct Delete {
    #[arg(value_parser = parse_url)]
    url: String,
}

#[derive(Debug, Clone, PartialEq)]
struct KeyValue {
    key: String,
    value: String,
}

impl FromStr for KeyValue {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid key-value pair: {}", s));
        }
        Ok(KeyValue {
            key: parts[0].to_string(),
            value: parts[1].to_string(),
        })
    }
}

/// 检查 URL 是否合法
fn parse_url(s: &str) -> Result<String> {
    let _url: Url = s.parse()?;
    Ok(s.into())
}

/// 检查 key-value pairs 是否合法
fn parse_key_value(kv: &str) -> Result<KeyValue> {
    kv.parse()
}

async fn get(client: Client, args: &Get) -> Result<()> {
    let response = client.get(&args.url).send().await?;
    Ok(print_response(response).await?)
}

async fn post(client: Client, args: &Post) -> Result<()> {
    let mut body = HashMap::new();
    for pair in args.body.iter() {
        body.insert(&pair.key, &pair.value);
    }
    let response = client.post(&args.url).form(&body).send().await?;
    Ok(print_response(response).await?)
}

async fn put(client: Client, args: &Put) -> Result<()> {
    let mut body = HashMap::new();
    for pair in args.body.iter() {
        body.insert(&pair.key, &pair.value);
    }
    let response = client.put(&args.url).form(&body).send().await?;
    Ok(print_response(response).await?)
}

async fn delete(client: Client, args: &Delete) -> Result<()> {
    let response = client.delete(&args.url).send().await?;
    Ok(print_response(response).await?)
}

fn print_status(response: &Response) {
    let status = format!("{:?} {}", response.version(), response.status()).blue();
    println!("{}\n", status);
}

fn print_header(response: &Response) {
    for (name, value) in response.headers() {
        println!("{}: {:?}", name.to_string().green(), value);
    }
    println!("\n");
}

fn print_body(m: Option<Mime>, body: &str) {
    match m {
        Some(m) if m == mime::APPLICATION_JSON => print_syntect(body, "json"),
        Some(m) if m == mime::TEXT_PLAIN => print_syntect(body, "html"),
        _ => println!("{}", body),
    }
}

fn get_content_type(response: &Response) -> Option<Mime> {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().parse().unwrap())
}

/// 使用 syntect 对 Body 输出进行语法高亮
fn print_syntect(s: &str, ext: &str) {
    // Load these once at the start of your program
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension(ext).unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    for line in LinesWithEndings::from(s) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
        print!("{}", escaped);
    }
}

/// 打印 Response
async fn print_response(response: Response) -> Result<()> {
    print_status(&response);
    print_header(&response);

    let mime = get_content_type(&response);
    let body = response.text().await?;
    print_body(mime, &body);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut headers = header::HeaderMap::new();
    headers.insert("x-power-by", "Rust".parse()?);
    headers.insert(header::USER_AGENT, "Rust Httpie".parse()?);

    let client = Client::builder().default_headers(headers).build()?;

    let result = match cli.command {
        Some(HttpMethod::Get(ref args)) => get(client, args).await?,
        Some(HttpMethod::Post(ref args)) => post(client, args).await?,
        Some(HttpMethod::Put(ref args)) => put(client, args).await?,
        Some(HttpMethod::Delete(ref args)) => delete(client, args).await?,
        None => println!("No HTTP method specified."),
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_url() {
        assert!(parse_url("abc").is_err());
        assert!(parse_url("http://abc.xyz").is_ok());
        assert!(parse_url("https://httpbin.org/post").is_ok());
    }
    #[test]
    fn test_parse_key_value() {
        assert!(parse_key_value("a").is_err());
        assert_eq!(
            parse_key_value("a=1").unwrap(),
            KeyValue {
                key: "a".into(),
                value: "1".into()
            }
        );
        assert_eq!(
            parse_key_value("b=").unwrap(),
            KeyValue {
                key: "b".into(),
                value: "".into()
            }
        );
    }
}
