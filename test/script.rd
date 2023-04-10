get http://localhost:8080 {
   header "random" env("love")
}

post http://localhost:8080 {
   header "random" "billy bob"
   body `{"neet": 1337}`
}
