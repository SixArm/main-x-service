#![warn(clippy::pedantic)]

//! Integration tests for the real `PersonService` gRPC server (PRO-H11).
//!
//! Unlike [`tests/api_integration_test.rs`], these tests do not go
//! through [`common::create_test_router`]: they bind a real
//! [`tonic::transport::Server`] on an OS-assigned ephemeral port (the
//! same `TcpListener::bind("127.0.0.1:0")` +
//! `serve_with_incoming` pattern `tests/otlp_collector/mod.rs` already
//! uses for its in-process collector) and drive it with a real
//! [`proto::person_service_client::PersonServiceClient`] over an actual
//! HTTP/2 connection — a genuine network round trip, not an in-process
//! function call. They share [`common::create_test_app_state`] with the
//! REST integration suite, so the record a gRPC call creates is proven
//! to have actually reached the same database and search index REST
//! reads from, not a second, disconnected code path.
//!
//! Requires a reachable `PostgreSQL` instance (see `compose.test.yaml`).

mod common;

use person_service::api::grpc::{proto, service::PersonGrpcService};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::Code;
use tonic::transport::{Endpoint, Server};

/// Bind `PersonGrpcService` on an ephemeral port and return a connected
/// client. The server task is detached (dropped with the test process);
/// each test gets its own port, so tests can run concurrently.
async fn start_test_server()
-> proto::person_service_client::PersonServiceClient<tonic::transport::Channel> {
    let state = common::create_test_app_state().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        Server::builder()
            .add_service(proto::person_service_server::PersonServiceServer::new(
                PersonGrpcService::new(state),
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
    proto::person_service_client::PersonServiceClient::new(channel)
}

fn new_person_request(family_name: String) -> proto::CreatePersonRequest {
    proto::CreatePersonRequest {
        person: Some(proto::Person {
            id: String::new(),
            active: true,
            family_name,
            given_names: vec!["Ada".to_string()],
            gender: proto::Gender::Female as i32,
            birth_date: Some("1990-01-15".to_string()),
            tax_id: None,
            created_at: String::new(),
            updated_at: String::new(),
        }),
    }
}

/// Create → Get → List → Delete → Get(NotFound), over a real socket,
/// each RPC calling the same `PersonRepository`/`SearchEngine`
/// REST uses. Proves the wire conversion and the shared domain logic
/// both work end to end, not just that the code compiles.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn create_get_list_delete_round_trip_over_real_grpc() {
    let mut client = start_test_server().await;
    let family_name = common::unique_person_name("Grpc");

    let created = client
        .create_person(new_person_request(family_name.clone()))
        .await
        .expect("CreatePerson")
        .into_inner();
    assert_eq!(created.family_name, family_name);
    assert_eq!(created.given_names, vec!["Ada".to_string()]);
    assert_eq!(created.gender, proto::Gender::Female as i32);
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
        .get_person(proto::GetPersonRequest {
            id: created.id.clone(),
        })
        .await
        .expect("GetPerson")
        .into_inner();
    assert_eq!(got.id, created.id);
    assert_eq!(got.family_name, family_name);

    let list = client
        .list_persons(proto::ListPersonsRequest {
            limit: 500,
            offset: 0,
        })
        .await
        .expect("ListPersons")
        .into_inner();
    assert!(
        list.persons.iter().any(|p| p.id == created.id),
        "the just-created person should appear in ListPersons"
    );

    client
        .delete_person(proto::DeletePersonRequest {
            id: created.id.clone(),
        })
        .await
        .expect("DeletePerson");

    let after_delete = client
        .get_person(proto::GetPersonRequest {
            id: created.id.clone(),
        })
        .await
        .expect_err("a soft-deleted person should no longer be gettable");
    assert_eq!(after_delete.code(), Code::NotFound);
}

/// T-36: `GetPerson` over gRPC writes the same HIPAA §164.528
/// disclosure-accounting audit row REST's `GET /api/persons/{id}`
/// does (`crate::api::rest::handlers::get_person`), driven by the same
/// `x-purpose-of-use` / `x-disclosure-recipient` declarations — carried
/// as gRPC metadata instead of HTTP headers. Mirrors
/// `tests/api_integration_test.rs`'s
/// `test_disclosure_accounting_states_whether_it_is_complete` shape:
/// the assertion is conditional on `PERSON_AUDIT_READS`, since that
/// gate is a process-wide `OnceLock` no test can flip after the first
/// read in this binary.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn get_person_over_grpc_writes_a_disclosure_accounting_row() {
    let state = common::create_test_app_state().await;
    let audit_log = state.audit_log.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        Server::builder()
            .add_service(proto::person_service_server::PersonServiceServer::new(
                PersonGrpcService::new(state),
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
    let mut client = proto::person_service_client::PersonServiceClient::new(channel);

    // A UUID rather than the shared timestamped helper: that helper
    // produces names sharing a long prefix across runs, which the
    // matcher scores as a duplicate and rejects (`AlreadyExists`) — the
    // same reason `tests/api_integration_test.rs`'s disclosure test
    // avoids it.
    let family_name = format!("GrpcDisclosure{}", uuid::Uuid::new_v4().simple());
    let created = client
        .create_person(new_person_request(family_name))
        .await
        .expect("CreatePerson")
        .into_inner();
    let id = uuid::Uuid::parse_str(&created.id).expect("a valid uuid from the server");

    // A disclosing read: naming a recipient is what makes this a
    // §164.528 *disclosure* rather than an internal access — without it
    // the accounting would be trivially empty and this test would pin
    // nothing.
    let mut request = tonic::Request::new(proto::GetPersonRequest {
        id: created.id.clone(),
    });
    request
        .metadata_mut()
        .insert("x-purpose-of-use", "treatment".parse().unwrap());
    request.metadata_mut().insert(
        "x-disclosure-recipient",
        "referring-clinic".parse().unwrap(),
    );
    let got = client
        .get_person(request)
        .await
        .expect("GetPerson")
        .into_inner();
    assert_eq!(got.id, created.id);

    let disclosures = audit_log
        .disclosures_for_entity(id, 10)
        .await
        .expect("query the disclosure trail");

    if person_service::compliance::audit_reads() {
        let row = disclosures
            .first()
            .expect("the gRPC read should have written a disclosure row");
        assert!(row.disclosure);
        let context = row.context.as_ref().expect("context json");
        assert_eq!(context["recipient"].as_str().unwrap(), "referring-clinic");
        assert_eq!(context["purpose_of_use"].as_str().unwrap(), "treatment");
    } else {
        assert!(
            disclosures.is_empty(),
            "PERSON_AUDIT_READS is off in this process, so no disclosure rows should have been \
             written"
        );
    }
}

/// `CreatePerson` runs the exact same
/// `crate::validation::validate_person` REST's `POST /api/persons`
/// does — proven here by triggering the same rule (a blank family
/// name) and getting `INVALID_ARGUMENT`, not a panic or a silently
/// accepted row.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn create_person_rejects_a_blank_family_name() {
    let mut client = start_test_server().await;

    let err = client
        .create_person(new_person_request(String::new()))
        .await
        .expect_err("a blank family name should fail the shared validator");
    assert_eq!(err.code(), Code::InvalidArgument);
}

/// A malformed id is a client-input error (`INVALID_ARGUMENT`), not a
/// database miss (`NOT_FOUND`) or an unhandled panic (`INTERNAL`).
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_integration_test -- --ignored`"]
async fn get_person_with_a_malformed_id_is_invalid_argument() {
    let mut client = start_test_server().await;

    let err = client
        .get_person(proto::GetPersonRequest {
            id: "not-a-uuid".to_string(),
        })
        .await
        .expect_err("a malformed id should be rejected before any database lookup");
    assert_eq!(err.code(), Code::InvalidArgument);
}
