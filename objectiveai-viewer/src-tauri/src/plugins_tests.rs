use axum::body::Body;
use axum::http::{Request, StatusCode};
use objectiveai_sdk::cli::command::plugins::list::{
    ResponseHttpMethod as HttpMethod, ResponseViewerRoute as ViewerRoute,
};
use tokio::sync::mpsc;
use tower::ServiceExt;

use objectiveai_sdk::viewer::Event;
use crate::plugins::register_plugin_route;

#[tokio::test]
async fn register_plugin_route_emits_event_with_type_and_value() {
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let route = ViewerRoute {
        path: "/echo".to_string(),
        method: HttpMethod::Post,
        r#type: "echo_request".to_string(),
    };
    let app = register_plugin_route(axum::Router::new(), tx, "myplugin".to_string(), route);

    let response = app
        .oneshot(
            Request::post("/plugin/myplugin/echo")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"hello":"world"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let event = rx.try_recv().expect("expected an event");
    let Event::Inbound { destination, sub_type, value } = event else {
        panic!("expected Event::Inbound, got {event:?}");
    };
    assert_eq!(destination, "myplugin");
    assert_eq!(sub_type, "echo_request");
    assert_eq!(value["hello"], "world");
}

#[tokio::test]
async fn register_plugin_route_emits_null_value_for_get() {
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let route = ViewerRoute {
        path: "/status".to_string(),
        method: HttpMethod::Get,
        r#type: "status_request".to_string(),
    };
    let app = register_plugin_route(axum::Router::new(), tx, "p".to_string(), route);

    let response = app
        .oneshot(Request::get("/plugin/p/status").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let event = rx.try_recv().expect("expected an event");
    let Event::Inbound { sub_type, value, .. } = event else {
        panic!("expected Event::Inbound, got {event:?}");
    };
    assert_eq!(sub_type, "status_request");
    assert!(value.is_null());
}
