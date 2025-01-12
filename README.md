# DDGPT (DuckDuckGPT)

A CLI client for duckduckgo's AI-Chatbots.
Only tested under linux.

Whilst this client falls under MIT,
the use of the service falls under duckduckgo's ToS and privacy policy.
Please see [https://duckduckgo.com/aichat/privacy-terms](https://duckduckgo.com/aichat/privacy-terms) for more details.

## Usage:
```
❯ ./target/debug/ddgpt  --help
A CLI interface to duckduckgo's chatbots

Usage: ddgpt [OPTIONS] <QUERY>...

Arguments:
  <QUERY>...  

Options:
  -m, --model <MODEL>           [possible values: gpt4o-mini, claude3, llama3, mistral]
  -s, --session <SESSION_NAME>  
  -c, --continue                
  -i, --interactive             
  -h, --help                    Print help
```

## Example query
```
❯ ./target/debug/ddgpt -cs fake-shell "pretend you're a bash shell on a linux system" 
Using model: gpt4o-mini ("gpt-4o-mini")

Sure! You can type commands, and I'll respond as if I'm a bash shell. What command would you like to run?
[DONE]

❯ ./target/debug/ddgpt -cs fake-shell "ls" 
Using model: gpt4o-mini ("gpt-4o-mini")
~~~
```plaintext
Desktop  Documents  Downloads  Music  Pictures  Videos
```
[DONE]
~~~