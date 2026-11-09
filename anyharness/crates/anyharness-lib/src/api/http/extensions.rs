use anyharness_contract::v1::extensions::{
    ExtensionColorTheme, ExtensionInstallationState, ExtensionPackageErrorCode,
    ExtensionSyntaxRule, ExtensionSyntaxTheme, InstallRegistryExtensionRequest,
    InstalledExtensionSummary, ListExtensionColorThemesResponse, ListExtensionSyntaxThemesResponse,
    ListInstalledExtensionsResponse,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::adapters::extensions::package::{PackageError, MAX_PACKAGE_BYTES};
use crate::adapters::extensions::registry::{
    fetch_registry_package, RegistryFetchError, RegistryPackageIdentity,
};
use crate::app::AppState;
use crate::domains::extensions::manager::ManagementError;
use crate::domains::extensions::manifest::ExtensionPackageIdentity;
use crate::domains::extensions::model::{ExtensionState, RegisteredExtension};

#[utoipa::path(
    get, path = "/v1/extensions",
    responses(
        (status = 200, description = "Registered extension packages; installation does not imply activation", body = ListInstalledExtensionsResponse),
        (status = 401, description = "Runtime authentication required", body = anyharness_contract::v1::ProblemDetails),
        (status = 503, description = "Extension management unavailable", body = anyharness_contract::v1::ProblemDetails)
    ), tag = "extensions"
)]
pub async fn list_extensions(
    State(state): State<AppState>,
) -> Result<Json<ListInstalledExtensionsResponse>, ApiError> {
    let manager = state.extension_manager.clone();
    let rows = run_blocking("extension_list", move || manager.list())
        .await
        .map_err(|_| unavailable())?
        .map_err(map_error)?;
    Ok(Json(ListInstalledExtensionsResponse {
        extensions: rows.into_iter().map(to_summary).collect(),
    }))
}

#[utoipa::path(
    get, path = "/v1/extensions/{namespace}/{name}/{version}/color-themes",
    params(("namespace" = String, Path), ("name" = String, Path), ("version" = String, Path)),
    responses(
        (status = 200, description = "Declared core app color data from an installed package", body = ListExtensionColorThemesResponse),
        (status = 400, description = "Invalid extension identity", body = anyharness_contract::v1::ProblemDetails),
        (status = 401, description = "Runtime authentication required", body = anyharness_contract::v1::ProblemDetails),
        (status = 404, description = "Extension version is not installed", body = anyharness_contract::v1::ProblemDetails),
        (status = 409, description = "Package validation failed", body = anyharness_contract::v1::ProblemDetails),
        (status = 503, description = "Extension management unavailable", body = anyharness_contract::v1::ProblemDetails)
    ), tag = "extensions"
)]
pub async fn list_color_themes(
    State(state): State<AppState>,
    Path((namespace, name, version)): Path<(String, String, String)>,
) -> Result<Json<ListExtensionColorThemesResponse>, ApiError> {
    let manager = state.extension_manager.clone();
    let (themes, truncated) = run_blocking("extension_color_themes", move || {
        manager.color_themes(&ExtensionPackageIdentity {
            namespace,
            name,
            version,
        })
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(map_error)?;
    Ok(Json(ListExtensionColorThemesResponse {
        themes: themes
            .into_iter()
            .map(|theme| ExtensionColorTheme {
                label: theme.label,
                color_mode: theme.color_mode.map(str::to_owned),
                colors: theme.colors,
                available: theme.available,
            })
            .collect(),
        truncated,
    }))
}

#[utoipa::path(
    get, operation_id = "list_extension_syntax_themes", path = "/v1/extensions/{namespace}/{name}/{version}/syntax-themes",
    params(("namespace" = String, Path), ("name" = String, Path), ("version" = String, Path)),
    responses((status = 200, body = ListExtensionSyntaxThemesResponse)), tag = "extensions"
)]
pub async fn list_syntax_themes(
    State(state): State<AppState>,
    Path((namespace, name, version)): Path<(String, String, String)>,
) -> Result<Json<ListExtensionSyntaxThemesResponse>, ApiError> {
    let manager = state.extension_manager.clone();
    let (themes, truncated) = run_blocking("extension_syntax_themes", move || {
        manager.syntax_themes(&ExtensionPackageIdentity {
            namespace,
            name,
            version,
        })
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(map_error)?;
    Ok(Json(ListExtensionSyntaxThemesResponse {
        themes: themes
            .into_iter()
            .map(|item| ExtensionSyntaxTheme {
                label: item.theme.label,
                color_mode: item.theme.color_mode.map(str::to_owned),
                colors: item.theme.colors,
                available: item.theme.available,
                rules: item
                    .rules
                    .into_iter()
                    .map(|rule| ExtensionSyntaxRule {
                        scopes: rule.scopes,
                        foreground: rule.foreground,
                    })
                    .collect(),
            })
            .collect(),
        truncated,
    }))
}

#[utoipa::path(
    delete, path = "/v1/extensions/{namespace}/{name}/{version}",
    params(("namespace" = String, Path), ("name" = String, Path), ("version" = String, Path)),
    responses(
        (status = 204, description = "Selected version removed or not registered"),
        (status = 400, description = "Invalid extension identity", body = anyharness_contract::v1::ProblemDetails),
        (status = 401, description = "Runtime authentication required", body = anyharness_contract::v1::ProblemDetails),
        (status = 409, description = "Package validation failed; package preserved", body = anyharness_contract::v1::ProblemDetails),
        (status = 503, description = "Extension management unavailable", body = anyharness_contract::v1::ProblemDetails)
    ), tag = "extensions"
)]
pub async fn uninstall_extension(
    State(state): State<AppState>,
    Path((namespace, name, version)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    let manager = state.extension_manager.clone();
    run_blocking("extension_uninstall", move || {
        manager.uninstall(&ExtensionPackageIdentity {
            namespace,
            name,
            version,
        })
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(map_error)?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/v1/extensions/install",
    request_body = InstallRegistryExtensionRequest,
    responses(
        (status = 201, description = "Package downloaded from the registry, verified, and registered; installation does not imply activation", body = InstalledExtensionSummary),
        (status = 400, description = "Invalid extension identity", body = anyharness_contract::v1::ProblemDetails),
        (status = 401, description = "Runtime authentication required", body = anyharness_contract::v1::ProblemDetails),
        (status = 404, description = "Extension version not found in the registry", body = anyharness_contract::v1::ProblemDetails),
        (status = 409, description = "Version already registered or package validation failed", body = anyharness_contract::v1::ProblemDetails),
        (status = 502, description = "Registry returned an unusable package", body = anyharness_contract::v1::ProblemDetails),
        (status = 503, description = "Extension management unavailable", body = anyharness_contract::v1::ProblemDetails)
    ), tag = "extensions"
)]
pub async fn install_registry_extension(
    State(state): State<AppState>,
    Json(request): Json<InstallRegistryExtensionRequest>,
) -> Result<(StatusCode, Json<InstalledExtensionSummary>), ApiError> {
    let identity = ExtensionPackageIdentity {
        namespace: request.namespace,
        name: request.name,
        version: request.version,
    };
    identity.directory_name().map_err(|_| {
        ApiError::bad_request(
            "Extension identity is invalid.",
            "EXTENSION_INVALID_IDENTITY",
        )
    })?;
    let manager = state.extension_manager.clone();
    let base_url = state.extension_registry_base_url.clone();
    let row = run_blocking("extension_registry_install", move || {
        let coordinates = RegistryPackageIdentity {
            namespace: identity.namespace.clone(),
            name: identity.name.clone(),
            version: identity.version.clone(),
        };
        let fetched =
            fetch_registry_package(&base_url, &coordinates, manager.home(), MAX_PACKAGE_BYTES)
                .map_err(map_fetch_error)?;
        let result = manager
            .install(&fetched.archive, &identity, &fetched.sha256)
            .map_err(map_error);
        let _ = std::fs::remove_file(&fetched.archive);
        result
    })
    .await
    .map_err(|_| unavailable())??;
    Ok((StatusCode::CREATED, Json(to_summary(row))))
}

fn to_summary(row: RegisteredExtension) -> InstalledExtensionSummary {
    InstalledExtensionSummary {
        namespace: row.identity.namespace,
        name: row.identity.name,
        version: row.identity.version,
        sha256: row.sha256,
        active: row.active,
        state: match row.state {
            ExtensionState::Installing => ExtensionInstallationState::Installing,
            ExtensionState::Installed => ExtensionInstallationState::Installed,
            ExtensionState::Removing => ExtensionInstallationState::Removing,
            ExtensionState::Error => ExtensionInstallationState::Error,
        },
        error_code: row.error_code.map(|code| match code.as_str() {
            "interrupted_install" => ExtensionPackageErrorCode::InterruptedInstall,
            "package_missing" => ExtensionPackageErrorCode::PackageMissing,
            _ => ExtensionPackageErrorCode::PackageInvalid,
        }),
    }
}

fn map_error(error: ManagementError) -> ApiError {
    match error {
        ManagementError::NotInstalled => ApiError::not_found(
            "This extension version is not installed.",
            "EXTENSION_NOT_INSTALLED",
        ),
        ManagementError::Manifest(_) => ApiError::bad_request(
            "Extension identity is invalid.",
            "EXTENSION_INVALID_IDENTITY",
        ),
        ManagementError::InvalidPackage | ManagementError::Package(PackageError::InvalidLayout) => {
            ApiError::conflict(
                "Package validation failed. The package was preserved.",
                "EXTENSION_PACKAGE_INVALID",
            )
        }
        ManagementError::AlreadyRegistered
        | ManagementError::Package(PackageError::AlreadyInstalled) => ApiError::conflict(
            "This extension version is already registered.",
            "EXTENSION_ALREADY_REGISTERED",
        ),
        ManagementError::Package(
            PackageError::InvalidChecksum | PackageError::ChecksumMismatch | PackageError::TooLarge,
        ) => registry_unusable(),
        _ => unavailable(),
    }
}

fn map_fetch_error(error: RegistryFetchError) -> ApiError {
    match error {
        RegistryFetchError::NotFound => ApiError::not_found(
            "This extension version was not found in the registry.",
            "EXTENSION_NOT_FOUND",
        ),
        RegistryFetchError::Io(_) => unavailable(),
        _ => {
            tracing::warn!(
                error_classification = "extension_registry_failed",
                "extension registry returned an unusable response"
            );
            registry_unusable()
        }
    }
}

fn registry_unusable() -> ApiError {
    ApiError::new(
        StatusCode::BAD_GATEWAY,
        "Extension registry unavailable",
        Some("The registry did not return a usable package. Retry later.".into()),
        Some("EXTENSION_REGISTRY_FAILED"),
    )
}

fn unavailable() -> ApiError {
    tracing::warn!(
        error_classification = "extension_management_failed",
        "extension management unavailable"
    );
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "Extension management unavailable",
        Some("The runtime could not complete extension management. Retry later.".into()),
        Some("EXTENSION_MANAGEMENT_FAILED"),
    )
}
