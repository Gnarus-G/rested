[![crates.io](https://img.shields.io/crates/v/rested.svg)](https://crates.io/crates/rested)
[![npm version](https://img.shields.io/npm/v/rstd.svg)](https://www.npmjs.com/package/rstd)

# rested (Experimental)

Language/Interpreter for easily defining and running requests to an http server.

# Why?

To easily test apis during development, and the Postman experience is slow. As someone who edits text files professionally, it seems natural to have a DSL for this usecase as well. It's a much better workflow to use curl commands in shell scripts than clicking around a GUI.
Many developers have great success with that strategy, and it's powerful because linux (piping files) is powerful. But it could be simpler still to have a DSL for something more powerful than curl.
Hence this experiment.

# Install (the CLI Interpreter)

From `crates.io`

```sh
cargo install rested
```

or from `npmjs.com`

```sh
npm install -g rstd
```

# Usage

```
Language/Interpreter for easily defining and running requests to an http server.

Usage: rstd [OPTIONS] <COMMAND>

Commands:
  run         Run a script written in the language
  fmt         Format a script written in the language
  scratch     Open your default editor to start editing a temporary file
  snap        Generate a static snapshot of the requests with all dynamic values evaluated
  env         Operate on the environment variables available in the runtime. Looking into the `.env.rd.json` in the current directory, or that in the home directory
  completion  Generate a completions file for a specified shell
  lsp         Start the rested language server
  config      Configure, or view current configurations
  help        Print this message or the help of the given subcommand(s)

Options:
  -l, --level <LEVEL>  Set log level, one of trace, debug, info, warn, error [default: info]
  -h, --help           Print help
  -V, --version        Print version
```

Write a script, for example

```rd
// assuming file name requests.rd
@log
get https://jsonplaceholder.typicode.com/todos/1
```

Run it with the CLI.

```sh
rstd run requests.rd
```

# Features

## Global constants

```rd
set BASE_URL "http://localhost:8080/api/v2"
```

setting BASE_URL like so, allows you to be able to make request to just pathnames

```rd
// This will make a request to "http://localhost:8080/api/v2/potatoes"
get /potatoes
```

## Let bindings

```rd
let token = "<token>"

// template string literals
let bearer_token = `Bearer ${token}`
```

## Defining request headers and request body

```rd
post /potatoes {
   header "Authorization" "Bearer token"

   // json expressions
   body json({
       neet: 1337,
       stuff: [1, true, "three"]
   })
}
```

## Reading environment variables

```rd
set BASE_URL env("base-url")

post /tomatoes {
   header "Authorization" env("auth-token")
   body env("data")
}
```

## Setting environment variables (CLI)

```sh
rstd env set <name> <value>
```

It's also possible to namespace the variables.

```sh
rstd env set <name> <value> -n <namespace>
```

```
Operate on the environment variables available in the runtime. Looking into the `.env.rd.json` in the current directory, or that in the home directory

Usage: rstd env [OPTIONS] <COMMAND>

Commands:
  show  View environment variables available in the runtime
  edit  Edit environment variables in your default editor
  set   Set environment variables available in the runtime
  ns    Operate on the variables namespaces available in the runtime
  help  Print this message or the help of the given subcommand(s)

Options:
      --cwd            Set to look at the `.env.rd.json` file in the current working directory. Otherwise this command and its subcommands operate on the `.env.rd.json` file in your home directory
  -l, --level <LEVEL>  Set log level, one of trace, debug, info, warn, error [default: info]
  -h, --help           Print help
```

## Reading files

```rd
post /tomatoes {
   body read("data.json")
}
```

## Attributes

```rd
// prints response body to stdout
@log
get /yams
```

```rd
// prints response body to a file
@log("output/yams.json")
get /yams
```

There are more, but I'm kind of ashamed of these attributes, so let's stop.

# Neovim Plugin

For Syntax Highlighting and Intellisense with the lsp, use [restedlang.nvim](https://github.com/gnarus-g/restedlang.nvim)
