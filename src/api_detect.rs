use std::path::Path;

pub fn detect_api_surface(root: &Path) -> bool {
    const MARKERS: &[&str] = &[
        "openapi.yaml",
        "openapi.yml",
        "openapi.json",
        "swagger.yaml",
        "swagger.json",
        "api.proto",
    ];
    for m in MARKERS {
        if root.join(m).exists() {
            return true;
        }
    }
    if root.join("proto").is_dir() {
        return true;
    }
    for name in [
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pyproject.toml",
        "composer.json",
        "build.gradle.kts",
        "build.gradle",
        "pubspec.yaml",
    ] {
        if let Ok(text) = std::fs::read_to_string(root.join(name)) {
            let t = text.to_ascii_lowercase();
            if t.contains("axum")
                || t.contains("actix-web")
                || t.contains("express")
                || t.contains("fastify")
                || t.contains("fastapi")
                || t.contains("ktor")
                || t.contains("spring-boot")
                || t.contains("laravel/framework")
                || t.contains("gin-gonic")
                || t.contains("github.com/labstack/echo")
            {
                return true;
            }
        }
    }
    false
}
