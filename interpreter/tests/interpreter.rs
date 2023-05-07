use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use interpreter::{runtime::Environment, Interpreter};

#[test]
fn requests_work() {
    let mut server = mockito::Server::new();

    let url = server.url();

    let mut env = Environment::new(PathBuf::from(".vars.rd.json")).unwrap();

    env.set_variable("b_url".to_string(), url).unwrap();
    env.set_variable("token".to_string(), "asl236ap9sdhf".to_string())
        .unwrap();

    let token = env.get_variable_value("token".to_string()).unwrap();

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

    let mut program = Interpreter::new(&code, env);

    program.run().unwrap();

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

    let mut program = Interpreter::new(&code, env);

    program.run().unwrap();

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

    let mut program = Interpreter::new(&code, env);

    program.run().unwrap();

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

    let mut program = Interpreter::new(&code, env);

    program.run().unwrap();

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
