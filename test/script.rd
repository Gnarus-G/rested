set BASE_URL env("b_url")

get http://localhost:8080

@log("output/random.json")
get / {
   header "random" env("love")
}

@dbg
@log("output/billy.json")
post /echo {
   header "random" "billy bob"
   header "Content-Type" "application/json"
   body `{"neet": "${read("test/text.txt")}"}`
}
