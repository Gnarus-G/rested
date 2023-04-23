set BASE_URL env("b_url")

@skip
get http://localhost:8080

@skip
@log
@dbg
get / {
   header "random" env("love")
}

@log @dbg
post /echo {
   header "random" "billy bob"
   header "Content-Type" "application/json"
   body read("data.json")

   // body `{
   // "neet": "${read("test/text.txt")}",
   // "12": "${read("test/text.txt")}" }`
}

@log @dbg
post /echo {
   header "random" "billy bob"
   header "Content-Type" "application/json"
   // body `{
   // "neet": "${escape_new_lines(read("data.txt"))}",
   // "34": "asdf\nasdf\n",
   // "12": "${escape_new_lines("yo\nbull\n")}" 
   // }`
body read(env(read("data.json")))
}
