set BASE_URL env("b_url")

get http://localhost:8080

@log("output/random.json")
get / {
   header "random" env("love")
}

@log("output/billy.json")
post http://localhost:8080 {
   header "random" "billy bob"
   body `{"neet": 1337}`
}
