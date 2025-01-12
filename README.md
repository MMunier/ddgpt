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

## Example Queries
#### GPT4o-Mini:
~~~
❯ ./target/debug/ddgpt -m gpt4 What is the best programming language mascot?
Using model: gpt4o-mini ("gpt-4o-mini")

The "best" programming language mascot can be subjective and often depends on personal preference or the context in which the mascot is used. Here are a few notable programming language mascots:

1. **Tux** - The Linux penguin is one of most recognized mascots in the programming world, representing the Linux operating system.

2. **Duke** - The Java mascot is a friendly character that symbolizes the Java programming language.

3. **Python** - The Python logo features two intertwined snakes, which represent the's name and its emphasis on readability and simplicity.

4. **Ruby** - The Ruby programming language is often represented by a red gemstone, symbolizing elegance and beauty in code.

5. **Go Gopher** - The Gopher is the mascot for the Go programming language, known for its cute and friendly appearance.

Ultimately, the "best" mascot may vary based on individual experiences and the communities surrounding each language.
[DONE]
~~~

#### Claude Haiku
~~~
❯ ./target/debug/ddgpt -m claude3 What is the best programming language mascot?
Using model: claude3 ("claude-3-haiku-20240307")

I do not have a strong opinion on the "best" programming language mascot. Programming language mascots are often used for branding and marketing purposes, but their design and selection is subjective. Some popular examples include the Python snake, the Java Duke, and the C++ elephant, but there is no universally agreed upon "best" mascot. The choice of mascot typically reflects the personality and culture of the programming language community.
[DONE]
~~~
#### Llama3
~~~
❯ ./target/debug/ddgpt -m llama What is the best programming language mascot?
Using model: llama3 ("meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo")

That's a subjective question, as opinions on the best programming language mascot can vary depending on personal taste. However, some popular programming language mascots include:

* Tux the penguin (Linux)
* Slashdot's mascot the "Slashdot Troll" (not specifically a programming language, but often associated with tech and programming communities)
* Glenda the snake (Python)
* Duke the Java mascot (Java)
* Mono's monkey mascot (Mono)

It's worth that some programming languages don't have official mascots, and some mascots may be more well-known or beloved than others. Ultimately, the "best" mascot is a matter of personal preference.
[DONE]
~~~
#### Mixtral
~~~
❯ ./target/debug/ddgpt -m mistral What is the best programming language mascot?
Using model: mistral ("mistralai/Mixtral-8x7B-Instruct-v0.1")

 The choice of the "best" programming language mascot can be subjective and depends on personal preferences. However, I can share some popular and well-liked mascots in the programming community:

1. **Py's Monty:** A friendly-looking cartoon snake, Monty represents Python, a popular and versatile language known for its simplicity and readability.

2. **Dart's Dash:** Dash is a cheerful,-running bird that embodies Dart, a language developed by Google for fast web development.

3. **Ruby's Yukihiro Matsumoto (Matz):** Although not a traditional mascot, Matz, the creator of Ruby, is often associated with the language. Ruby is known for its elegant and expressive syntax.

4. **Java's Duke:** Duke is a friendly, blue-colored character representing Java, a versatile and widelyused language for enterprise-level applications.

5. **C#'s Bee:** The C# mascot is a bee, symbolizing the language's efficiency and productivity. C# is a statically-typed, multi-paradigm language developed by Microsoft.

6. **Go's Gopher:** The Gopher is a cute, furry animal that represents Go, a language developed by Google for simplicity, reliability, and efficiency.

Rem, the best mascot is often a matter of personal preference and the community around the language.
[DONE]
~~~
(Side Note: They are all wrong because we know it's Ferris!)


### Session Mode (continue)
~~~
❯ ./target/debug/ddgpt -cs fake-shell "pretend you're a bash shell on a linux system" 
Using model: gpt4o-mini ("gpt-4o-mini")

Sure! You can type commands, and I'll respond as if I'm a bash shell. What command would you like to run?
[DONE]

❯ ./target/debug/ddgpt -cs fake-shell "ls" 
Using model: gpt4o-mini ("gpt-4o-mini")
```plaintext
Desktop  Documents  Downloads  Music  Pictures  Videos
```
[DONE]
~~~