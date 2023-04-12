set BASE_URL env("b_url")

get http://localhost:8080

get /love/craft {
   header "random" env("love")
}

post http://localhost:8080 {
   header "random" "billy bob"
   body `{"neet": 1337}`
}
