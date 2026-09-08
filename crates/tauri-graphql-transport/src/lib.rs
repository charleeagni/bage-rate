pub mod api;
pub mod endpoint;

pub use api::{TransportApi, TransportApiImpl};
pub use endpoint::GraphQlEndpoint;

pub fn export_bindings(path: impl AsRef<std::path::Path>) -> Result<(), String> {
    let path = path.as_ref();
    let api = TransportApiImpl::new();
    taurpc::Exporter::new()
        .export(&api.into_handler(), path)
        .map_err(|error| format!("cannot export TauRPC bindings: {error}"))?;
    let generated = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read TauRPC bindings: {error}"))?;
    std::fs::write(path, format!("{}\n", generated.trim_end()))
        .map_err(|error| format!("cannot normalize TauRPC bindings: {error}"))
}
