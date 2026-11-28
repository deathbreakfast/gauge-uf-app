//! Host composition: wire Gauge `actor_can` into uf-product permission gates, and Gauge
//! resource bundles into uf-product's shared Access views.

#[cfg(feature = "ssr")]
use std::sync::Arc;

#[cfg(feature = "ssr")]
use leptos::prelude::*;

/// Gauge-backed [`uf_product::permissions::PermissionBackend`].
#[cfg(feature = "ssr")]
pub struct GaugePermissionBackend;

#[cfg(feature = "ssr")]
#[async_trait::async_trait]
impl uf_product::permissions::PermissionBackend for GaugePermissionBackend {
    async fn has_permission(&self, permission_name: &str) -> Result<bool, ServerFnError> {
        let ctx = higgs::Higgs::from_request().await?;
        let valence = ctx
            .valence()
            .map_err(|e| ServerFnError::new(format!("Failed to build Valence: {e}")))?;
        let _caller = gauge::instrumentation::PermissionCheckCallerGuard::new("permission_backend");
        gauge::service::actor_can(&valence, permission_name)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to check permission: {e}")))
    }
}

/// Call from host shell bootstrap before serving routes (SSR).
#[cfg(feature = "ssr")]
pub fn wire_gauge_permissions() {
    uf_product::permissions::provide_permission_backend(Arc::new(GaugePermissionBackend));
}

#[cfg(feature = "ssr")]
pub use kinds::{register_resource_kind, registered_resource_kind, ResourceKindEntry};
#[cfg(feature = "ssr")]
pub use resource_access::GaugeResourceAccessBackend;

#[cfg(feature = "ssr")]
mod kinds;
#[cfg(feature = "ssr")]
mod resource_access;

#[cfg(all(test, feature = "ssr"))]
mod tests;
