#![warn(clippy::pedantic)]

//! Integration tests for the real `WorkerService` gRPC server (PRO-H11),
//! following person-service's reference implementation
//! (`tests/grpc_integration_test.rs`) for this repo's gRPC rollout.
//!
//! Unlike [`tests/api_integration_test.rs`], these tests do not go
//! through [`common::create_test_router`]: they bind a real
//! [`tonic::transport::Server`] on an OS-assigned ephemeral port (the
//! same `TcpListener::bind("127.0.0.1:0")` + `serve_with_incoming`
//! pattern `tests/otlp_collector/mod.rs` already uses for its
//! in-process collector) and drive it with a real
//! [`proto::worker_service_client::WorkerServiceClient`] over an actual
//! HTTP/2 connection — a genuine network round trip, not an in-process
//! function call. They share [`common::create_test_app_state`] with the
//! REST integration suite, so the record a gRPC call creates is proven
//! to have actually reached the same database and search index REST
//! reads from, not a second, disconnected code path.
//!
//! Requires a reachable `PostgreSQL` instance (see `compose.test.yaml`).

mod common;

use tokio_stream::wrappers::TcpListenerStream;
use tonic::Code;
use tonic::transport::{Endpoint, Server};
use worker_service::api::grpc::{proto, service::WorkerGrpcService};

/// Bind `WorkerGrpcService` on an ephemeral port and return a connected
/// client. The server task is detached (dropped with the test process);
/// each test gets its own port, so tests can run concurrently.
async fn start_test_server()
-> proto::worker_service_client::WorkerServiceClient<tonic::transport::Channel> {
    let state = common::create_test_app_state().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        Server::builder()
            .add_service(proto::worker_service_server::WorkerServiceServer::new(
                WorkerGrpcService::new(state),
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
    proto::worker_service_client::WorkerServiceClient::new(channel)
}

fn new_worker_request(family_name: String) -> proto::CreateWorkerRequest {
    proto::CreateWorkerRequest {
        worker: Some(proto::Worker {
            id: String::new(),
            active: true,
            family_name,
            given_names: vec!["Ada".to_string()],
            gender: proto::Gender::Female as i32,
            worker_type: Some("nurse".to_string()),
            birth_date: Some("1990-01-15".to_string()),
            tax_id: None,
            created_at: String::new(),
            updated_at: String::new(),
        }),
    }
}

/// Create → Get → List → Delete → Get(NotFound), over a real socket,
/// each RPC calling the same `WorkerRepository`/`SearchEngine` REST
/// uses. Proves the wire conversion and the shared domain logic both
/// work end to end, not just that the code compiles.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn create_get_list_delete_round_trip_over_real_grpc() {
    let mut client = start_test_server().await;
    let family_name = common::unique_worker_name("Grpc");

    let created = client
        .create_worker(new_worker_request(family_name.clone()))
        .await
        .expect("CreateWorker")
        .into_inner();
    assert_eq!(created.family_name, family_name);
    assert_eq!(created.given_names, vec!["Ada".to_string()]);
    assert_eq!(created.gender, proto::Gender::Female as i32);
    assert_eq!(created.worker_type.as_deref(), Some("nurse"));
    assert_eq!(created.birth_date.as_deref(), Some("1990-01-15"));
    assert!(
        !created.id.is_empty(),
        "the server should have assigned a UUID, not echoed the empty input id"
    );
    assert!(
        !created.created_at.is_empty() && !created.updated_at.is_empty(),
        "timestamps should be server-set"
    );

    let got = client
        .get_worker(proto::GetWorkerRequest {
            id: created.id.clone(),
        })
        .await
        .expect("GetWorker")
        .into_inner();
    assert_eq!(got.id, created.id);
    assert_eq!(got.family_name, family_name);

    let list = client
        .list_workers(proto::ListWorkersRequest {
            limit: 500,
            offset: 0,
        })
        .await
        .expect("ListWorkers")
        .into_inner();
    assert!(
        list.workers.iter().any(|w| w.id == created.id),
        "the just-created worker should appear in ListWorkers"
    );

    client
        .delete_worker(proto::DeleteWorkerRequest {
            id: created.id.clone(),
        })
        .await
        .expect("DeleteWorker");

    let after_delete = client
        .get_worker(proto::GetWorkerRequest {
            id: created.id.clone(),
        })
        .await
        .expect_err("a soft-deleted worker should no longer be gettable");
    assert_eq!(after_delete.code(), Code::NotFound);
}

/// `CreateWorker` runs the exact same
/// `crate::validation::validate_worker` REST's `POST /api/workers`
/// does — proven here by triggering the same rule (a blank family
/// name) and getting `INVALID_ARGUMENT`, not a panic or a silently
/// accepted row.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn create_worker_rejects_a_blank_family_name() {
    let mut client = start_test_server().await;

    let err = client
        .create_worker(new_worker_request(String::new()))
        .await
        .expect_err("a blank family name should fail the shared validator");
    assert_eq!(err.code(), Code::InvalidArgument);
}

/// A malformed `worker_type` wire token is a client-input error
/// (`INVALID_ARGUMENT`), never silently coerced to a default — the
/// domain enum has no "unknown" variant to fall back to, unlike
/// `Gender`.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn create_worker_rejects_an_unrecognised_worker_type() {
    let mut client = start_test_server().await;
    let mut req = new_worker_request(common::unique_worker_name("GrpcBadType"));
    req.worker.as_mut().unwrap().worker_type = Some("astronaut".to_string());

    let err = client
        .create_worker(req)
        .await
        .expect_err("an unrecognised worker_type should be rejected");
    assert_eq!(err.code(), Code::InvalidArgument);
}

/// A malformed id is a client-input error (`INVALID_ARGUMENT`), not a
/// database miss (`NOT_FOUND`) or an unhandled panic (`INTERNAL`).
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn get_worker_with_a_malformed_id_is_invalid_argument() {
    let mut client = start_test_server().await;

    let err = client
        .get_worker(proto::GetWorkerRequest {
            id: "not-a-uuid".to_string(),
        })
        .await
        .expect_err("a malformed id should be rejected before any database lookup");
    assert_eq!(err.code(), Code::InvalidArgument);
}
