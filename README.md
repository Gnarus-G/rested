# rested
Language/Interpreter for defining and running requests to an http server.

# Why?
To easily test apis during development, and the Postman experience is slow. As someone who edits text files professionally, it seems natural to have a DSL for this usecase as well. It's a much better workflow to use curl commands in shell scripts than clicking around a GUI.
Many developers have great success with that strategy, and it's powerful because linux (piping files) is powerful. But it could be simpler to still to have a DSL. Hence this experiment.

# Example
```rd
get http://localhost:8080

set BASE_URL env("base-url")

post /api/v2 {
   header "Authorization" env("auth-token")
   body `{"neet": 1337}`
}
```
