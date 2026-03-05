use polyref_drift::http_client::HttpClient;
use polyref_drift::registry::RegistryClient;
use polyref_drift::registry::crates_io::CratesIoClient;
use polyref_drift::registry::pypi::PypiClient;
use polyref_drift::registry::npm::NpmClient;

// =====================================================================
// crates.io tests
// =====================================================================

#[test]
fn test_crates_io_get_latest_version() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/crates/serde")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"crate": {"max_version": "1.0.219"}}"#)
        .create();

    let client = HttpClient::new(None);
    let crates_client = CratesIoClient::with_base_url(client, server.url());
    let result = crates_client.get_latest_version("serde").unwrap();

    assert_eq!(result.name, "serde");
    assert_eq!(result.version, "1.0.219");
    assert_eq!(result.registry, "crates.io");
    mock.assert();
}

#[test]
fn test_crates_io_registry_name() {
    let client = HttpClient::new(None);
    let crates_client = CratesIoClient::new(client);
    assert_eq!(crates_client.registry_name(), "crates.io");
}

#[test]
fn test_crates_io_not_found() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/crates/nonexistent-crate-xyz")
        .with_status(404)
        .with_body(r#"{"errors":[{"detail":"Not Found"}]}"#)
        .create();

    let client = HttpClient::new(None);
    let crates_client = CratesIoClient::with_base_url(client, server.url());
    let result = crates_client.get_latest_version("nonexistent-crate-xyz");

    assert!(result.is_err());
    mock.assert();
}

#[test]
fn test_crates_io_malformed_json() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/crates/bad")
        .with_status(200)
        .with_body("not json at all")
        .create();

    let client = HttpClient::new(None);
    let crates_client = CratesIoClient::with_base_url(client, server.url());
    let result = crates_client.get_latest_version("bad");

    assert!(result.is_err());
    mock.assert();
}

// =====================================================================
// PyPI tests
// =====================================================================

#[test]
fn test_pypi_get_latest_version() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/requests/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"info": {"version": "2.31.0"}}"#)
        .create();

    let client = HttpClient::new(None);
    let pypi_client = PypiClient::with_base_url(client, server.url());
    let result = pypi_client.get_latest_version("requests").unwrap();

    assert_eq!(result.name, "requests");
    assert_eq!(result.version, "2.31.0");
    assert_eq!(result.registry, "pypi");
    mock.assert();
}

#[test]
fn test_pypi_registry_name() {
    let client = HttpClient::new(None);
    let pypi_client = PypiClient::new(client);
    assert_eq!(pypi_client.registry_name(), "pypi");
}

#[test]
fn test_pypi_not_found() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/nonexistent-package/json")
        .with_status(404)
        .with_body("Not Found")
        .create();

    let client = HttpClient::new(None);
    let pypi_client = PypiClient::with_base_url(client, server.url());
    let result = pypi_client.get_latest_version("nonexistent-package");

    assert!(result.is_err());
    mock.assert();
}

// =====================================================================
// npm tests
// =====================================================================

#[test]
fn test_npm_get_latest_version() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/express")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"dist-tags": {"latest": "4.18.2"}}"#)
        .create();

    let client = HttpClient::new(None);
    let npm_client = NpmClient::with_base_url(client, server.url());
    let result = npm_client.get_latest_version("express").unwrap();

    assert_eq!(result.name, "express");
    assert_eq!(result.version, "4.18.2");
    assert_eq!(result.registry, "npm");
    mock.assert();
}

#[test]
fn test_npm_registry_name() {
    let client = HttpClient::new(None);
    let npm_client = NpmClient::new(client);
    assert_eq!(npm_client.registry_name(), "npm");
}

#[test]
fn test_npm_not_found() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/nonexistent-package-xyz")
        .with_status(404)
        .with_body("Not Found")
        .create();

    let client = HttpClient::new(None);
    let npm_client = NpmClient::with_base_url(client, server.url());
    let result = npm_client.get_latest_version("nonexistent-package-xyz");

    assert!(result.is_err());
    mock.assert();
}

#[test]
fn test_npm_malformed_json() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/bad-package")
        .with_status(200)
        .with_body("{invalid json}")
        .create();

    let client = HttpClient::new(None);
    let npm_client = NpmClient::with_base_url(client, server.url());
    let result = npm_client.get_latest_version("bad-package");

    assert!(result.is_err());
    mock.assert();
}

// =====================================================================
// HttpClient tests
// =====================================================================

#[test]
fn test_http_client_get_json_success() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/test")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"key": "value"}"#)
        .create();

    let client = HttpClient::new(None);
    let result: serde_json::Value = client.get_json(&format!("{}/test", server.url())).unwrap();

    assert_eq!(result["key"], "value");
    mock.assert();
}

#[test]
fn test_http_client_get_success() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/hello")
        .with_status(200)
        .with_body("hello world")
        .create();

    let client = HttpClient::new(None);
    let result = client.get(&format!("{}/hello", server.url())).unwrap();

    assert_eq!(result, "hello world");
    mock.assert();
}
