use std::{fs::File, io::Read, path::PathBuf};

use insta::assert_debug_snapshot;
use interpreter::{environment::Environment, ureq_runner::UreqRunner, Interpreter};

fn new_env_with_vars(vars: &[(&str, &str)]) -> Environment {
    let mut env = Environment::new(PathBuf::from(".vars.rd.json")).unwrap();

    for (key, value) in vars {
        env.set_variable(key.to_string(), value.to_string())
            .unwrap();
    }

    return env;
}

#[test]
fn requests_work() {
    let mut server = mockito::Server::new();

    let url = server.url();

    let token = "asl236ap9sdhf";

    let env = new_env_with_vars(&[("b_url", &url), ("token", token)]);

    let get_api = server.mock("GET", "/api").with_status(200).create();

    let get_api_v2 = server
        .mock("GET", "/api/v2")
        .with_status(200)
        .with_header("Authorization", &token)
        .create();

    let post_api = server
        .mock("POST", "/api")
        .with_status(200)
        .with_header("Authorization", &token)
        .with_body("data")
        .create();

    let put_api = server
        .mock("PUT", "/api")
        .with_status(200)
        .with_header("Authorization", &token)
        .with_body("data")
        .create();

    let patch_api = server
        .mock("PATCH", "/api")
        .with_status(200)
        .with_header("Authorization", &token)
        .with_body("data")
        .create();

    let delete_api = server
        .mock("DELETE", "/api")
        .with_status(200)
        .with_header("Authorization", &token)
        .create();

    let code = r#"
        set BASE_URL env("b_url")

        get /api
        get /api/v2 {
           header "Authorization" env("token")
        }

        post /api {
           header "Authorization" env("token")
           body "data"
        }

        put /api {
           header "Authorization" env("token")
           body "data"
        }

        patch /api {
           header "Authorization" env("token")
           body "data"
        }

        delete /api {
           header "Authorization" env("token")
        }
    "#;

    let mut program = Interpreter::new(&code, env, UreqRunner);

    program.run(None).unwrap();

    get_api.assert();
    get_api_v2.assert();
    post_api.assert();
    put_api.assert();
    patch_api.assert();
    delete_api.assert();
}

#[test]
fn comments_are_ignored() {
    let mut server = mockito::Server::new();

    let url = server.url();

    let env = Environment::new(PathBuf::from(".vars.rd.json")).unwrap();

    let get_api = server
        .mock("GET", "/api")
        .with_status(200)
        .expect(0)
        .create();

    let get_api_v2 = server.mock("GET", "/api/v2").with_status(200).create();

    let code = format!(
        r#"
        // set BASE_URL env("b_url")
        // get /api
        // @skip
        get {url}/api/v2 {{
            // header Aas "test"
        }}
    "#
    );

    let mut program = Interpreter::new(&code, env, UreqRunner);

    program.run(None).unwrap();

    get_api.assert();
    get_api_v2.assert();
}

#[test]
fn requests_are_skippable() {
    let mut server = mockito::Server::new();
    let url = server.url();
    let mut env = Environment::new(PathBuf::from(".vars.rd.json")).unwrap();

    env.set_variable("b_url".to_string(), url).unwrap();

    let mocks = ["GET", "POST", "PUT", "PATCH", "DELETE"]
        .map(|method| server.mock(method, "/api").with_status(200).create());

    let code = r#"
        set BASE_URL env("b_url")
        get /api 
        @skip
        get /api 

        post /api 
        @skip
        post /api 

        put /api 
        @skip
        put /api 

        patch /api 
        @skip
        patch /api 

        delete /api 
        @skip
        delete /api 
    "#;

    let mut program = Interpreter::new(&code, env, UreqRunner);

    program.run(None).unwrap();

    for mock in mocks {
        mock.assert();
    }
}

#[test]
fn responses_can_be_logged() {
    let mut server = mockito::Server::new();
    let url = server.url();
    let mut env = Environment::new(PathBuf::from(".vars.rd.json")).unwrap();

    env.set_variable("b_url".to_string(), url).unwrap();

    let mocks = ["POST"].map(|method| {
        server
            .mock(method, "/api")
            .with_status(200)
            .with_body_from_file("tests/files/test_data.json")
            .create()
    });

    let code = r#"
        set BASE_URL env("b_url")

        @log("tests/output/test_data_echo.json")
        post /api
    "#;

    let mut program = Interpreter::new(&code, env, UreqRunner);

    program.run(None).unwrap();

    let mut input_file = File::open("tests/files/test_data.json").unwrap();
    let mut output_file = File::open("tests/output/test_data_echo.json").unwrap();

    let mut req_body = String::new();
    input_file.read_to_string(&mut req_body).unwrap();

    let mut res_body = String::new();
    output_file.read_to_string(&mut res_body).unwrap();

    assert_eq!(req_body, res_body);

    for mock in mocks {
        mock.assert();
    }
}

#[test]
fn let_bindings_work() {
    let mut server = mockito::Server::new();
    let url = server.url();
    let mut env = Environment::new(PathBuf::from(".vars.rd.json")).unwrap();

    env.set_variable("test".to_string(), "12345".to_string())
        .unwrap();
    env.set_variable("b_url".to_string(), url).unwrap();

    let mocks = ["POST"].map(|method| {
        server
            .mock(method, "/api")
            .with_status(200)
            .with_header("test", env.get_variable_value(&"test".to_string()).unwrap())
            .with_header("test1", "asdf")
            .create()
    });

    let code = r#"
        set BASE_URL env("b_url")

        let variable = env("test")
        let o_variable = "asdf"

        post /api {
            header "test" variable
            header "test1" o_variable
        }
    "#;

    let mut program = Interpreter::new(&code, env, UreqRunner);

    program.run(None).unwrap();

    for mock in mocks {
        mock.assert();
    }
}

#[test]
fn running_specific_requests_by_name() {
    let mut server = mockito::Server::new();
    let url = server.url();
    let env = new_env_with_vars(&[("b_url", &url)]);

    let mocks =
        ["GET", "POST", "PUT"].map(|method| server.mock(method, "/api").with_status(200).create());

    let del = server
        .mock("DELETE", "/api")
        .with_status(200)
        .expect(0)
        .create();

    let code = r#"
        set BASE_URL env("b_url")

        get /api 
        post /api 
        put /api 

        @name("test")
        get /api 

        @name("test")
        post /api 

        @name("test")
        put /api 

        @name("nope")
        delete /api
    "#;

    let mut program = Interpreter::new(&code, env, UreqRunner);

    program.run(Some(vec!["test".to_string()])).unwrap();

    for mock in mocks {
        mock.assert();
    }
    del.assert();
}

#[test]
fn name_attribute_requires_value() {
    let mut env = Environment::new(PathBuf::from(".vars.rd.json")).unwrap();

    env.set_variable("b_url".to_string(), "asdfasdf".to_string())
        .unwrap();

    let code = r#"
        set BASE_URL env("b_url")
        @name
        get /api {}
    "#;

    let mut program = Interpreter::new(&code, env, UreqRunner);

    let name_att_without_arg_err = program.run(Some(vec!["test".to_string()])).unwrap_err();

    assert_debug_snapshot!(name_att_without_arg_err);
}

#[test]
fn prevents_duplicate_attributes() {
    let code = r#"
        set BASE_URL env("b_url")
        @log
        @log
        get /api {}
    "#;

    let env = new_env_with_vars(&[("b_url", "asdfasdf")]);
    let mut program = Interpreter::new(&code, env, UreqRunner);

    let duped_att_err = program.run(Some(vec!["test".to_string()])).unwrap_err();
    assert_debug_snapshot!(duped_att_err);

    let code = r#"
        set BASE_URL env("b_url")
        @name("a")
        @name("b")
        get /api {}
    "#;

    let env = new_env_with_vars(&[("b_url", "asdfasdf")]);
    let mut program = Interpreter::new(&code, env, UreqRunner);

    let duped_att_err = program.run(Some(vec!["test".to_string()])).unwrap_err();
    assert_debug_snapshot!(duped_att_err);
}

#[test]
fn request_with_json_like_data() {
    let code = r#"
set BASE_URL env("b_url")

post /api {
    header "Content-Type" "application/json"
    body {
        neet: 1337,
        nothing: null,
        arr: ["yo", {h: "i"}],
        "hello": {
            w: env("hello"),
            warudo: env(env("hi")),
            "fun": true,
            notFun: false,
            e: {},
            em: []
        },
    }
}
        "#;

    let mut server = mockito::Server::new();
    let url = server.url();
    let env = new_env_with_vars(&[("b_url", &url), ("hello", "world"), ("hi", "hello")]);

    let mock = server
        .mock("POST", "/api")
        .match_header("Content-Type", "application/json")
        .match_body(mockito::Matcher::PartialJsonString(
            r#"{"neet": 1337, "nothing": null, "arr": ["yo", {"h": "i"}], "hello": {"w": "world", "warudo": "world", "fun": true, "notFun": false, "e": {}, "em": []}}"#.to_string(),
        ))
        .with_status(200)
        .create();

    let mut program = Interpreter::new(code, env, UreqRunner);

    program.run(None).unwrap();

    mock.assert();
}

#[test]
fn ignores_expression_items() {
    let code = r#"
env("test") 
read("file")

// obj literal
{
    key: "value",
    oKey: ["1", "2"]
}

// string literal expression
"adsf"
        "#;
    let env = new_env_with_vars(&[]);
    let mut program = Interpreter::new(&code, env, UreqRunner);

    program.run(None).unwrap();
}
