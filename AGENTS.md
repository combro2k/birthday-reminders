# Project Instructions: Domain-Driven Design (DDD)

This project follows strict **Domain-Driven Design (DDD)** principles to ensure scalability and maintainability. All contributors (AI and human) must adhere to the following structural and architectural guidelines.

## 1. Directory Structure
Every file must be placed according to its specific domain. A standard domain module should follow this internal hierarchy:

* **`src/[domain_name]/domain/`**: Pure business logic, entities, and value objects (no external dependencies).
* **`src/[domain_name]/application/`**: Use cases, command handlers, and services.
* **`src/[domain_name]/infrastructure/`**: Database implementations, external API clients, and repository concrete types.
* **`src/[domain_name]/presentation/`**: Actix/Axum handlers, DTOs, and request/response logic.

## 2. Template Management
Templates must be co-located with their respective domains to maintain encapsulation while remaining accessible to the global template engine.

* **Global Path**: All templates must reside within the root `/templates` directory.
* **Domain Alignment**: Within the `/templates` folder, create subdirectories that mirror the Rust domain structure.
    * *Example*: If the Rust code is in `src/billing/`, the templates must be in `/templates/billing/`.
* **Separation of Concerns**: Templates must not contain `<script>` or `<style>` blocks, nor inline `style` or event-handler attributes (e.g., `onclick`, `onsubmit`). All logic and styling must be placed in external files within the `/static` directory.

## 3. Implementation Rules
* **Encapsulation**: Do not allow infrastructure details to leak into the `domain` layer.
* **Explicit Mapping**: Use the "Newtype" pattern or dedicated DTOs when moving data between the `infrastructure` and `domain` layers.
* **Modularity**: Each domain should be a self-contained module in `mod.rs` or defined as a workspace member if the project grows significantly.
* **Shared repository error type**: Domain repository traits **MUST** use `crate::infrastructure::error::RepositoryError` as the error type. Do not define per-domain error types for repository operations.

## 4. Static Asset Management
Static assets should follow a predictable hierarchy that mirrors the domain structure, similar to templates.

### 4.1 Directory Structure
- **Global Path**: All static assets must reside within the root `/static/` directory.
- **Domain Alignment**: Feature-specific scripts and assets **MUST** be placed in subdirectories under `/static/` (e.g., `/static/birthdays/`).
- **Shared Assets**: Global scripts (e.g., `main.js`, `sw.js`) and cross-cutting assets (e.g., global CSS, shared utilities, or PWA manifests) should be kept in the root of `/static/`.

### 4.2 Dynamic Import Mapping
The project uses a modular frontend structure where page-specific logic is dynamically imported based on the `body` element's ID. Always reference `/static/main.js` to determine the correct path for page-specific modules.

**Body ID Naming Convention**: `page-[domain]-[view]`
This ID must correspond to the module path in `/static/[domain]/[view].js`.

The ID is set via the `body_attributes` template block in `base.html`. Every page template that needs JS must override it:
```
{% block body_attributes %}id="page-[domain]-[view]"{% endblock %}
```
Without this block the dynamic import in `main.js` will silently find no match and load no page module.
Primary domains identified:
- `birthdays`: `page-birthdays-list`, `page-birthdays-edit`
- `channels`: `page-channels-list`, `page-channels-edit` (Notification Channels)
- `users`: `page-users-profile`, `page-users-settings`
- `home`: `page-home-dashboard`
- `auth`: `page-auth-login`, `page-auth-register`
- `offline`: `page-offline-index`

### 4.3 File Placement Rules
- **DO NOT** place feature-specific scripts (like `list.js`) directly in the `/static/` root.
- Always verify the file path matches the dynamic import path defined in the `switch` statement in `main.js`.

## 5. Database Migrations
Every schema change **MUST** include matching migration files for all three database backends:

```
migrations/sqlite/YYYYMMDDnnnn_description.sql
migrations/mysql/YYYYMMDDnnnn_description.sql
migrations/postgres/YYYYMMDDnnnn_description.sql
```

- The timestamp prefix and filename stem **MUST** be identical across all three backends.
- SQLite, MySQL, and PostgreSQL have different SQL dialects — each file must use syntax appropriate for its backend.
- Migrations that require no schema changes on a particular backend (e.g. when a type is already correct) **MUST** still have a corresponding file, with a comment explaining why no change is needed.

## 6. Quality Checks
After completing any editing task, run the release check script before considering the work done:

```bash
bash scripts/release-check.sh
```

The script verifies version consistency between `Cargo.toml` and `package.json`, rebuilds the Tailwind CSS, and runs `cargo fmt`, `cargo test`, and `cargo clippy` with strict flags. It must exit without errors.
The script runs `cargo clean` only when there are Git changes in `src/`, `static/`, `templates/`, `tests/`, or `migrations/`; otherwise it skips clean to keep checks faster.

## 7. Release and Compliance Requirements

- **Changelog is mandatory for every version bump**: Each version bump **MUST** be documented in `CHANGELOG.md` with a clear summary of what changed.
- **Version consistency is mandatory for every version bump**: `package.json` and `Cargo.toml` **MUST** have the exact same version.
- **Version bump workflow is mandatory**: A version bump **MUST** explicitly include all of the following steps:
    - Update the version in all required files. If the exact target version is assumed rather than provided, explicitly ask the user to confirm the version before proceeding.
    - If the user asks for a version bump without specifying a target version, calculate and suggest:
        - Next minor version (`x.(y+1).0`) as the default/recommended option.
        - Next major version (`(x+1).0.0`) as an alternative option.
            Present these choices using an input selector (interactive option picker), with next minor preselected/recommended, while still allowing explicit freeform version input.
      Always ask for explicit confirmation before applying any version change.
    - Commit the version bump changes. Commit all relevant files for the version bump; if it is unclear whether all changed files should be included, ask the user before committing.
    - Create a Git tag for the version.
    - Push only when the user explicitly asks for a push.
- **Release validation is mandatory**: `bash scripts/release-check.sh` **MUST** be run and pass without errors on every version bump.
    - `cargo clean` is conditional inside the script and only runs when relevant code/assets/test/migration paths changed.
- **No personal or private information in the codebase**:
    - The repository **MUST NOT** contain personal/private data or secrets.
    - This includes (but is not limited to): tokens, passwords, usernames, API keys, credentials, private identifiers, or similar sensitive values.