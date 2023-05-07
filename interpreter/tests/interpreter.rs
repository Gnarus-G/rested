use std::path::PathBuf;

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
