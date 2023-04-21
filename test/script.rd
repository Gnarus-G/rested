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
   body `{
   "neet": "${read("test/text.txt")}",
   "12": "${read("test/text.txt")}"
   }`
}
