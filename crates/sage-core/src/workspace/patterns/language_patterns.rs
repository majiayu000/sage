//! Language-specific pattern definitions

use super::types::{ImportantFileType, ProjectPattern};

/// Universal patterns common to all projects
pub fn universal_patterns() -> Vec<ProjectPattern> {
    vec![
        // Documentation
        ProjectPattern::new("README", ImportantFileType::Documentation)
            .with_patterns(["README.md", "README.txt", "README", "readme.md"])
            .with_priority(90)
            .with_description("Project documentation"),
        ProjectPattern::new("License", ImportantFileType::License)
            .with_patterns(["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"])
            .with_priority(70),
        ProjectPattern::new("Changelog", ImportantFileType::Documentation)
            .with_patterns(["CHANGELOG.md", "CHANGELOG", "HISTORY.md"])
            .with_priority(60),
        ProjectPattern::new("Contributing", ImportantFileType::Documentation)
            .with_patterns(["CONTRIBUTING.md", "CONTRIBUTING"])
            .with_priority(50),
        // CI/CD
        ProjectPattern::new("GitHub Actions", ImportantFileType::CiCd)
            .with_patterns([".github/workflows/*.yml", ".github/workflows/*.yaml"])
            .with_priority(70),
        ProjectPattern::new("GitLab CI", ImportantFileType::CiCd)
            .with_patterns([".gitlab-ci.yml"])
            .with_priority(70),
        ProjectPattern::new("CircleCI", ImportantFileType::CiCd)
            .with_patterns([".circleci/config.yml"])
            .with_priority(70),
        // Container
        ProjectPattern::new("Dockerfile", ImportantFileType::Container)
            .with_patterns(["Dockerfile", "Dockerfile.*", "*.dockerfile"])
            .with_priority(75),
        ProjectPattern::new("Docker Compose", ImportantFileType::Container)
            .with_patterns(["docker-compose.yml", "docker-compose.yaml", "compose.yml"])
            .with_priority(75),
        // Environment
        ProjectPattern::new("Environment", ImportantFileType::Environment)
            .with_patterns([".env", ".env.example", ".env.local", ".env.development"])
            .with_priority(80),
        // Git
        ProjectPattern::new("Gitignore", ImportantFileType::Config)
            .with_patterns([".gitignore"])
            .with_priority(50),
        // Editor config
        ProjectPattern::new("EditorConfig", ImportantFileType::Config)
            .with_patterns([".editorconfig"])
            .with_priority(30),
    ]
}

/// Rust-specific patterns
pub fn rust_patterns() -> Vec<ProjectPattern> {
    vec![
        // Entry points
        ProjectPattern::new("Main", ImportantFileType::EntryPoint)
            .with_patterns(["src/main.rs", "src/bin/*.rs"])
            .with_priority(100)
            .with_description("Application entry point"),
        ProjectPattern::new("Library", ImportantFileType::EntryPoint)
            .with_patterns(["src/lib.rs"])
            .with_priority(95)
            .with_description("Library entry point"),
        // Build
        ProjectPattern::new("Cargo.toml", ImportantFileType::Build)
            .with_patterns(["Cargo.toml"])
            .with_priority(100)
            .with_description("Cargo manifest"),
        ProjectPattern::new("Cargo.lock", ImportantFileType::LockFile)
            .with_patterns(["Cargo.lock"])
            .with_priority(60),
        // Config
        ProjectPattern::new("Rust Config", ImportantFileType::Config)
            .with_patterns([
                "rust-toolchain.toml",
                "rustfmt.toml",
                ".rustfmt.toml",
                "clippy.toml",
            ])
            .with_priority(60),
        // Tests
        ProjectPattern::new("Tests", ImportantFileType::Test)
            .with_patterns(["tests/*.rs", "tests/**/*.rs"])
            .with_priority(70),
        // Build script
        ProjectPattern::new("Build Script", ImportantFileType::Build)
            .with_patterns(["build.rs"])
            .with_priority(80),
    ]
}

/// Node.js/TypeScript patterns
pub fn node_patterns() -> Vec<ProjectPattern> {
    vec![
        // Entry points
        ProjectPattern::new("Index", ImportantFileType::EntryPoint)
            .with_patterns([
                "src/index.ts",
                "src/index.tsx",
                "src/index.js",
                "src/index.jsx",
                "index.ts",
                "index.js",
            ])
            .with_priority(95),
        ProjectPattern::new("App", ImportantFileType::EntryPoint)
            .with_patterns([
                "src/App.tsx",
                "src/App.jsx",
                "src/app.ts",
                "src/app.js",
                "app/page.tsx",
                "app/page.jsx",
                "pages/_app.tsx",
                "pages/_app.jsx",
            ])
            .with_priority(90),
        ProjectPattern::new("Server", ImportantFileType::EntryPoint)
            .with_patterns([
                "server.ts",
                "server.js",
                "src/server.ts",
                "src/server.js",
                "src/main.ts",
                "src/main.js",
            ])
            .with_priority(90),
        // Build
        ProjectPattern::new("Package.json", ImportantFileType::Build)
            .with_patterns(["package.json"])
            .with_priority(100),
        // Lock files
        ProjectPattern::new("Lock File", ImportantFileType::LockFile)
            .with_patterns([
                "package-lock.json",
                "yarn.lock",
                "pnpm-lock.yaml",
                "bun.lockb",
            ])
            .with_priority(60),
        // Config
        ProjectPattern::new("TypeScript Config", ImportantFileType::Config)
            .with_patterns(["tsconfig.json", "tsconfig.*.json"])
            .with_priority(85),
        ProjectPattern::new("ESLint", ImportantFileType::Config)
            .with_patterns([
                ".eslintrc",
                ".eslintrc.js",
                ".eslintrc.json",
                ".eslintrc.cjs",
                "eslint.config.js",
                "eslint.config.mjs",
            ])
            .with_priority(60),
        ProjectPattern::new("Prettier", ImportantFileType::Config)
            .with_patterns([
                ".prettierrc",
                ".prettierrc.js",
                ".prettierrc.json",
                "prettier.config.js",
                "prettier.config.mjs",
            ])
            .with_priority(50),
        ProjectPattern::new("Vite Config", ImportantFileType::Build)
            .with_patterns(["vite.config.ts", "vite.config.js"])
            .with_priority(80),
        ProjectPattern::new("Next.js Config", ImportantFileType::Build)
            .with_patterns(["next.config.js", "next.config.mjs", "next.config.ts"])
            .with_priority(85),
        ProjectPattern::new("Webpack Config", ImportantFileType::Build)
            .with_patterns(["webpack.config.js", "webpack.config.ts"])
            .with_priority(80),
        // Tests
        ProjectPattern::new("Test Config", ImportantFileType::Config)
            .with_patterns([
                "jest.config.js",
                "jest.config.ts",
                "vitest.config.ts",
                "playwright.config.ts",
                "cypress.config.ts",
            ])
            .with_priority(70),
        // Type definitions
        ProjectPattern::new("Type Definitions", ImportantFileType::TypeDefinition)
            .with_patterns(["*.d.ts", "types/*.ts", "typings/*.ts"])
            .with_priority(65),
        // API
        ProjectPattern::new("OpenAPI", ImportantFileType::ApiDefinition)
            .with_patterns([
                "openapi.yaml",
                "openapi.json",
                "swagger.yaml",
                "swagger.json",
            ])
            .with_priority(75),
    ]
}

/// Python patterns
pub fn python_patterns() -> Vec<ProjectPattern> {
    vec![
        // Entry points
        ProjectPattern::new("Main", ImportantFileType::EntryPoint)
            .with_patterns(["main.py", "app.py", "run.py", "__main__.py"])
            .with_priority(95),
        ProjectPattern::new("Package Init", ImportantFileType::EntryPoint)
            .with_patterns(["src/*/__init__.py", "*/__init__.py"])
            .with_priority(80),
        // Build
        ProjectPattern::new("Pyproject", ImportantFileType::Build)
            .with_patterns(["pyproject.toml"])
            .with_priority(100),
        ProjectPattern::new("Setup", ImportantFileType::Build)
            .with_patterns(["setup.py", "setup.cfg"])
            .with_priority(90),
        ProjectPattern::new("Requirements", ImportantFileType::Build)
            .with_patterns(["requirements.txt", "requirements/*.txt"])
            .with_priority(85),
        // Lock files
        ProjectPattern::new("Lock File", ImportantFileType::LockFile)
            .with_patterns(["poetry.lock", "Pipfile.lock", "pdm.lock", "uv.lock"])
            .with_priority(60),
        // Config
        ProjectPattern::new("Pytest Config", ImportantFileType::Config)
            .with_patterns(["pytest.ini", "pyproject.toml", "conftest.py"])
            .with_priority(70),
        ProjectPattern::new("Linter Config", ImportantFileType::Config)
            .with_patterns([".flake8", "ruff.toml", ".ruff.toml", ".pylintrc"])
            .with_priority(50),
        // Tests
        ProjectPattern::new("Tests", ImportantFileType::Test)
            .with_patterns(["tests/*.py", "test_*.py", "*_test.py"])
            .with_priority(70),
        // Type definitions
        ProjectPattern::new("Type Stubs", ImportantFileType::TypeDefinition)
            .with_patterns(["*.pyi", "py.typed"])
            .with_priority(60),
        // Database
        ProjectPattern::new("Alembic", ImportantFileType::Database)
            .with_patterns(["alembic.ini", "alembic/*.py"])
            .with_priority(70),
    ]
}

/// Go patterns
pub fn go_patterns() -> Vec<ProjectPattern> {
    vec![
        // Entry points
        ProjectPattern::new("Main", ImportantFileType::EntryPoint)
            .with_patterns(["main.go", "cmd/*/main.go"])
            .with_priority(100),
        // Build
        ProjectPattern::new("Go Mod", ImportantFileType::Build)
            .with_patterns(["go.mod"])
            .with_priority(100),
        ProjectPattern::new("Go Sum", ImportantFileType::LockFile)
            .with_patterns(["go.sum"])
            .with_priority(60),
        ProjectPattern::new("Makefile", ImportantFileType::Build)
            .with_patterns(["Makefile"])
            .with_priority(80),
        // Tests
        ProjectPattern::new("Tests", ImportantFileType::Test)
            .with_patterns(["*_test.go", "**/*_test.go"])
            .with_priority(70),
        // Config
        ProjectPattern::new("Golangci", ImportantFileType::Config)
            .with_patterns([".golangci.yml", ".golangci.yaml"])
            .with_priority(60),
    ]
}

/// Java/Kotlin patterns
pub fn java_patterns() -> Vec<ProjectPattern> {
    vec![
        // Entry points
        ProjectPattern::new("Main", ImportantFileType::EntryPoint)
            .with_patterns([
                "**/Application.java",
                "**/Main.java",
                "**/App.java",
                "**/Application.kt",
                "**/Main.kt",
            ])
            .with_priority(95),
        // Build
        ProjectPattern::new("Maven", ImportantFileType::Build)
            .with_patterns(["pom.xml"])
            .with_priority(100),
        ProjectPattern::new("Gradle", ImportantFileType::Build)
            .with_patterns([
                "build.gradle",
                "build.gradle.kts",
                "settings.gradle",
                "settings.gradle.kts",
            ])
            .with_priority(100),
        // Config
        ProjectPattern::new("Application Config", ImportantFileType::Config)
            .with_patterns([
                "src/main/resources/application.properties",
                "src/main/resources/application.yml",
                "src/main/resources/application.yaml",
            ])
            .with_priority(90),
        // Tests
        ProjectPattern::new("Tests", ImportantFileType::Test)
            .with_patterns(["src/test/java/**/*.java", "src/test/kotlin/**/*.kt"])
            .with_priority(70),
    ]
}
