set BASE_URL env("b_url")

// @name("a")
// @name("a")
@log
@log("google.html")
get https://google.com

let random = env("love")
//
// @log
// @dbg
// get / {
//    header "random" random
// }
//
//
// let content_type = "application/json"
//
// let billy = "Billy bob"
//
// @log @dbg
// post /echo {
//    header "random" billy
//    header "Content-Type" content_type
//    body read("data.json")
// }
//
// @log @dbg
// post /echo {
//    header "random" "billy bob"
//    header "Content-Type" "application/json"
//    body `{
//    "neet": "${escape_new_lines(read("data.txt"))}",
//    "34": "asdf\nasdf\n"
//    }`
// }
