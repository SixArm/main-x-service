#![warn(clippy::pedantic)]

//! Integration tests for the real `EventService` gRPC server (PRO-H11),
//! following person-service's and worker-service's reference
//! implementations (`tests/grpc_integration_test.rs`) for this repo's
//! gRPC rollout.
//!
//! Unlike [`tests/api_integration_test.rs`], these tests do not go
//! through [`common::create_test_router`]: they bind a real
//! [`tonic::transport::Server`] on an OS-assigned ephemeral port (the
//! same `TcpListener::bind("127.0.0.1:0")` + `serve_with_incoming`
//! pattern `tests/otlp_collector/mod.rs` already uses for its
//! in-process collector) and drive it with a real
//! [`proto::event_service_client::EventServiceClient`] over an actual
//! HTTP/2 connection — a genuine network round trip, not an in-process
//! function call. They share [`common::create_test_app_state`] with the
//! REST integration suite, so the record a gRPC call creates is proven
//! to have actually reached the same database and search index REST
//! reads from, not a second, disconnected code path.
//!
//! Requires a reachable `PostgreSQL` instance (see `compose.test.yaml`).

mod common;

use event_service::api::grpc::{proto, service::EventGrpcService};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::Code;
use tonic::transport::{Endpoint, Server};

/// Bind `EventGrpcService` on an ephemeral port and return a connected
/// client. The server task is detached (dropped with the test process);
/// each test gets its own port, so tests can run concurrently.
async fn start_test_server()
-> proto::event_service_client::EventServiceClient<tonic::transport::Channel> {
    let state = common::create_test_app_state().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        Server::builder()
            .add_service(proto::event_service_server::EventServiceServer::new(
                EventGrpcService::new(state),
            ))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .ok();
    });

    let channel = Endpoint::from_shared(format!("http://{addr}"))
        .expect("valid endpoint")
        .connect()
        .await
        .expect("connect to the test gRPC server");
    proto::event_service_client::EventServiceClient::new(channel)
}

fn new_event_request(name: String) -> proto::CreateEventRequest {
    proto::CreateEventRequest {
        event: Some(proto::Event {
            id: String::new(),
            active: true,
            name,
            start_date: "2026-01-15T09:00:00Z".to_string(),
            end_date: Some("2026-01-15T17:00:00Z".to_string()),
            event_status: Some("scheduled".to_string()),
            created_at: String::new(),
            updated_at: String::new(),
        }),
    }
}

/// Create → Get → List → Delete → Get(NotFound), over a real socket,
/// each RPC calling the same `EventRepository`/`SearchEngine` REST
/// uses. Proves the wire conversion and the shared domain logic both
/// work end to end, not just that the code compiles.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn create_get_list_delete_round_trip_over_real_grpc() {
    let mut client = start_test_server().await;
    let name = common::unique_event_name("Grpc");

    let created = client
        .create_event(new_event_request(name.clone()))
        .await
        .expect("CreateEvent")
        .into_inner();
    assert_eq!(created.name, name);
    assert_eq!(created.start_date, "2026-01-15T09:00:00+00:00");
    assert_eq!(
        created.end_date.as_deref(),
        Some("2026-01-15T17:00:00+00:00")
    );
    assert_eq!(created.event_status.as_deref(), Some("scheduled"));
    assert!(
        !created.id.is_empty(),
        "the server should have assigned a UUID, not echoed the empty input id"
    );
    assert!(
        !created.created_at.is_empty() && !created.updated_at.is_empty(),
        "timestamps should be server-set"
    );

    let got = client
        .get_event(proto::GetEventRequest {
            id: created.id.clone(),
        })
        .await
        .expect("GetEvent")
        .into_inner();
    assert_eq!(got.id, created.id);
    assert_eq!(got.name, name);

    let list = client
        .list_events(proto::ListEventsRequest {
            limit: 500,
            offset: 0,
        })
        .await
        .expect("ListEvents")
        .into_inner();
    assert!(
        list.events.iter().any(|e| e.id == created.id),
        "the just-created event should appear in ListEvents"
    );

    client
        .delete_event(proto::DeleteEventRequest {
            id: created.id.clone(),
        })
        .await
        .expect("DeleteEvent");

    let after_delete = client
        .get_event(proto::GetEventRequest {
            id: created.id.clone(),
        })
        .await
        .expect_err("a soft-deleted event should no longer be gettable");
    assert_eq!(after_delete.code(), Code::NotFound);
}

/// `CreateEvent` runs the exact same `crate::validation::validate_event`
/// REST's `POST /api/events` does — proven here by triggering the same
/// rule (a blank name) and getting `INVALID_ARGUMENT`, not a panic or a
/// silently accepted row.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn create_event_rejects_a_blank_name() {
    let mut client = start_test_server().await;

    let err = client
        .create_event(new_event_request(String::new()))
        .await
        .expect_err("a blank name should fail the shared validator");
    assert_eq!(err.code(), Code::InvalidArgument);
}

/// A malformed `event_status` wire token is a client-input error
/// (`INVALID_ARGUMENT`), never silently coerced to the default.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn create_event_rejects_an_unrecognised_event_status() {
    let mut client = start_test_server().await;
    let mut req = new_event_request(common::unique_event_name("GrpcBadStatus"));
    req.event.as_mut().unwrap().event_status = Some("teleporting".to_string());

    let err = client
        .create_event(req)
        .await
        .expect_err("an unrecognised event_status should be rejected");
    assert_eq!(err.code(), Code::InvalidArgument);
}

/// A malformed id is a client-input error (`INVALID_ARGUMENT`), not a
/// database miss (`NOT_FOUND`) or an unhandled panic (`INTERNAL`).
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn get_event_with_a_malformed_id_is_invalid_argument() {
    let mut client = start_test_server().await;

    let err = client
        .get_event(proto::GetEventRequest {
            id: "not-a-uuid".to_string(),
        })
        .await
        .expect_err("a malformed id should be rejected before any database lookup");
    assert_eq!(err.code(), Code::InvalidArgument);
}
