[![crates.io](https://img.shields.io/crates/v/rested.svg)](https://crates.io/crates/rested)
[![npm version](https://img.shields.io/npm/v/rstd.svg)](https://www.npmjs.com/package/rstd)

# rested (Experimental)

Language/Interpreter for easily defining and running requests to an http server.

# Why?

To easily test apis during development, and the Postman experience is slow. As someone who edits text files professionally, it seems natural to have a DSL for this usecase as well. It's a much better workflow to use curl commands in shell scripts than clicking around a GUI.
Many developers have great success with that strategy, and it's powerful because linux (piping files) is powerful. But it could be simpler to still to have a DSL. Hence this experiment.

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
The CLI runtime for Rested, the language/interpreter for easily defining and running requests to an http server

Usage: rstd [COMMAND]

Commands:
  run         Run a script written in the language
  env         Operate on the environment variables available in the runtime
  completion  Generate a completions file for a specified shell
  lsp         Start the rested language server
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
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

setting BASE_URL like so, allows you to be able to request to pathnames

```rd
get /potatoes
```

## Let bindings

```rd
let variable = "Bearer <token>"
```

## Defining request headers and request body

```rd
post /potatoes {
   header "Authorization" "Bearer token"

   // template string literals
   body `{"neet": 1337}`

   // or json-like expressions
   body { neet: 1337 }
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

## Reading files

```rd
post /tomatoes {
   body read("data.json")
}
```

## Attributes

```rd
@log("output/yams.json")
get /yams
```
