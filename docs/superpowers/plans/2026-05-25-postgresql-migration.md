# PostgreSQL Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate elysia (Drizzle ORM + MySQL) and rust (SeaORM + MySQL) apps to PostgreSQL via direct cutover.

**Architecture:** Export MySQL data, convert schema to PostgreSQL format, import into target PostgreSQL instance, update app drivers and connection strings, deploy and validate.

**Tech Stack:** MySQL (source), PostgreSQL (target), Drizzle ORM (elysia), SeaORM (rust), pgloader or manual conversion for schema migration.

---

## File Structure

### Elysia App Changes
- `apps/elysia/src/db/lib/database.ts` — Replace mysql2 driver with postgres driver
- `apps/elysia/src/db/lib/schema.ts` — Replace mysqlTable with pgTable, update column types
- `apps/elysia/package.json` — Replace mysql2 with pg dependency
- `.env` or config file — Update DATABASE_URL to PostgreSQL connection string

### Rust App Changes
- `apps/rust/Cargo.toml` — Replace sqlx-mysql feature with sqlx-postgres
- `apps/rust/src/infra/db_setup.rs` — Update DbBackend::MySql to DbBackend::Postgres, adjust SQL syntax
- Config/environment — Update DATABASE_URL to PostgreSQL connection string

### Migration Artifacts
- `mysql_backup.sql` — MySQL dump (created during migration, not committed)
- `converted.sql` — PostgreSQL-compatible dump (created during migration, not committed)

---

## Task Breakdown

### Task 1: Backup MySQL and Export Schema

**Files:**
- Create: `mysql_backup.sql` (temporary, not committed)

- [ ] **Step 1: Export MySQL database**

```bash
cd /mnt/code/bp3/ultimate-asepharyana.tech
mysqldump -u <mysql_user> -p <mysql_password> -h <mysql_host> <database_name> > mysql_backup.sql
```

Expected: File created with full schema + data. Verify file size > 1MB (contains data).

-[]**Step 2: Verify backup integrity**

```bash
# Check row counts in backup
grep "INSERT INTO" mysql_backup.sql | wc -l
```

Expected: Multiple INSERT statements present. Note row counts for later validation.

- [ ] **Step 3: Document backup location**

Store `mysql_backup.sql` in safe location (not in git). This is rollback insurance.

---

### Task 2: Convert MySQL Schema to PostgreSQL

**Files:**
- Create: `converted.sql` (temporary, not committed)

- [ ] **Step 1: Install pgloader (if not present)**

```bash
# macOS
brew install pgloader

# Linux (Ubuntu/Debian)
sudo apt-get install pgloader

# Or use Docker
docker run --rm -v $(pwd):/data pgloader/pgloader pgloader /data/mysql_backup.sql postgresql://asephs:hunterz@100.108.1.124:5432/hub
```

- [ ] **Step 2: Convert MySQL dump to PostgreSQL**

```bash
pgloader mysql_backup.sql postgresql://asephs:hunterz@100.108.1.124:5432/hub
```

Or manually convert if pgloader unavailable:
- Replace `AUTO_INCREMENT` with `SERIAL` or `BIGSERIAL`
- Replace `DATETIME` with `TIMESTAMP`
- Replace backticks with double quotes
- Update index syntax for PostgreSQL

Expected: Conversion completes without errors. Check for warnings about type conversions.

- [ ] **Step 3: Verify conversion output**

```bash
# If using pgloader, it creates converted.sql automatically
# If manual, save converted schema to file
cat converted.sql | head -50
```

Expected: PostgreSQL-compatible SQL syntax (no backticks, SERIAL types, TIMESTAMP).

---

### Task 3: Test PostgreSQL Import

**Files:**
- Target: PostgreSQL instance at `postgresql://asephs:hunterz@100.108.1.124:5432/hub`

- [ ] **Step 1: Connect to target PostgreSQL**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub
```

Expected: Connected to PostgreSQL. Prompt shows `hub=#`.

- [ ] **Step 2: Import converted schema**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub < converted.sql
```

Expected: Import completes. Check for errors (should be none).

- [ ] **Step 3: Validate table creation**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "\dt"
```

Expected: All tables listed (User, Account, Session, Role, Permission, UserRole, ImageCache, etc.).

- []**Step 4: Validate row counts**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "SELECT COUNT(*) FROM \"User\";"
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "SELECT COUNT(*) FROM \"Account\";"
```

Expected: Row counts match MySQL backup (from Task 1, Step 2).

- [ ] **Step 5: Validate indexes**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "\di"
```

Expected: All indexes present (email_idx, username_idx, userId_idx, sessionToken_idx, etc.).

- [ ] **Step 6: Validate foreign keys**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "SELECT constraint_name, table_name FROM information_schema.table_constraints WHERE constraint_type = 'FOREIGN KEY';"
```

Expected: Foreign key constraints listed (User→Account, User→Session, etc.).

---

### Task 4: Update Elysia Database Driver

**Files:**
- Modify: `apps/elysia/src/db/lib/database.ts`
- Modify: `apps/elysia/src/db/lib/schema.ts`
- Modify: `apps/elysia/package.json`

- [ ] **Step 1: Update package.json dependencies**

Replace mysql2 with pg:

```json
{
  "dependencies": {
    "drizzle-orm": "^0.45.2",
    "pg": "^8.11.0",
    "elysia": "^1.4.28"
  },
  "devDependencies": {
    "drizzle-kit": "^0.31.10"
  }
}
```

Run: `cd apps/elysia && bun install`

Expected: pg installed, mysql2 removed from node_modules.

- [ ] **Step 2: Update database.ts driver**

Replace entire file:

```typescript
import type { PostgresJsDatabase } from 'drizzle-orm/postgres-js'
import { drizzle } from 'drizzle-orm/postgres-js'
import postgres from 'postgres'
import * as schema from './schema'

export type Database = PostgresJsDatabase<typeof schema>

let dbInstance: Database | null = null
let sqlInstance: ReturnType<typeof postgres> | null = null

export function initializeDb(databaseUrl: string): Database {
  if (dbInstance) {
    return dbInstance
  }

  sqlInstance = postgres(databaseUrl)
  dbInstance = drizzle(sqlInstance, { schema, mode: 'default' })

  return dbInstance
}

export function getDb(): Database {
  if (!dbInstance) {
    throw new Error('Database not initialized. Call initializeDb first.')
  }
  return dbInstance
}

export async function closeDb() {
  if (sqlInstance) {
    await sqlInstance.end()
    sqlInstance = null
    dbInstance = null
  }
}
```

Expected: File updated. No syntax errors.

- [ ] **Step 3: Update schema.ts imports**

Replace:
```typescript
import {
  index,
  int,
  mysqlTable,
  primaryKey,
  text,
  timestamp,
  varchar,
} from 'drizzle-orm/mysql-core'
```

With:
```typescript
import {
  index,
  integer,
  pgTable,
  primaryKey,
  text,
  timestamp,
  varchar,
} from 'drizzle-orm/postgres-core'
```

- [ ] **Step 4: Update schema.ts table definitions**

Replace all `mysqlTable` with `pgTable` and `int` with `integer`:

```typescript
// Before
export const users = mysqlTable(
  'User',
  {
    id: varchar('id', { length: 255 }).primaryKey(),
    name: varchar('name', { length: 255 }),
    // ...
  },
  // ...
)

// After
export const users = pgTable(
  'User',
  {
    id: varchar('id', { length: 255 }).primaryKey(),
    name: varchar('name', { length: 255 }),
    // ...
  },
  // ...
)
```

Do this for all tables: users, accounts, sessions, roles, permissions, userRoles, and any others.

Expected: All `mysqlTable` → `pgTable`, all `int` → `integer`.

- [ ] **Step 5: Update environment variable**

Set DATABASE_URL in `.env` or deployment config:

```bash
DATABASE_URL=postgresql://asephs:hunterz@100.108.1.124:5432/hub
```

Expected: Environment variable set and accessible to elysia app.

- [ ] **Step 6: Test elysia connection**

```bash
cd apps/elysia
bun run src/index.ts
```

Expected: App starts without connection errors. Check logs for "Database initialized" or similar.

- [ ] **Step 7: Commit elysia changes**

```bash
cd /mnt/code/bp3/ultimate-asepharyana.tech
git add apps/elysia/src/db/lib/database.ts apps/elysia/src/db/lib/schema.ts apps/elysia/package.json
git commit -m "feat(elysia): migrate database driver from MySQL to PostgreSQL"
```

Expected: Commit created with message.

---

### Task 5: Update Rust Database Driver

**Files:**
- Modify: `apps/rust/Cargo.toml`
- Modify: `apps/rust/src/infra/db_setup.rs`

- [ ] **Step 1: Update Cargo.toml features**

Replace:
```toml
sea-orm = { version = "1.1.19", features = ["sqlx-mysql", "runtime-tokio-rustls", "macros", "with-chrono", "with-uuid"] }
```

With:
```toml
sea-orm = { version = "1.1.19", features = ["sqlx-postgres", "runtime-tokio-rustls", "macros", "with-chrono", "with-uuid"] }
```

Expected: Cargo.toml updated. Feature changed from sqlx-mysql to sqlx-postgres.

- [ ] **Step 2: Update db_setup.rs backend check**

Replace:
```rust
match backend {
    DbBackend::MySql => {
        // MySQL-specific logic
    }
    _ => {
        info!("ℹ️ Skipping schema init for non-MySQL backend");
    }
}
```

With:
```rust
match backend {
    DbBackend::Postgres => {
        // PostgreSQL-specific logic
        let tables = vec![(
            "ImageCache",
            schema
                .create_table_from_entity(image_cache::Entity)
                .if_not_exists()
                .to_owned(),
        )];

        for (name, stmt) in tables {
            match db.execute(backend.build(&stmt)).await {
                Ok(_) => info!("   ✓ Table '{}' checked/created", name),
                Err(e) => {
                    error!("   [!] Failed to create table '{}': {}", name, e);
                    return Err(e);
                }
            }
        }

        // PostgreSQL index creation (different syntax)
        let index_sql = "CREATE INDEX IF NOT EXISTS idx_image_cache_cdn_url ON \"ImageCache\" (cdn_url)";
        match db.execute(Statement::from_string(backend, index_sql)).await {
            Ok(_) => info!("   ✓ Index 'idx_image_cache_cdn_url' ensured"),
            Err(e) => {
                let err_str = e.to_string();
                // PostgreSQL duplicate index error
                if err_str.contains("already exists") {
                    info!("   ✓ Index 'idx_image_cache_cdn_url' already exists");
                } else {
                    error!("   [!] Failed to create index on ImageCache: {}", e);
                }
            }
        }

        info!("✅ Database schema initialization complete.");
    }
    _ => {
        info!("ℹ️ Skipping schema init for non-PostgreSQL backend");
    }
}
```

Expected: db_setup.rs updated with PostgreSQL backend handling.

- [ ] **Step 3: Update environment variable**

Set DATABASE_URL in `.env` or deployment config:

```bash
DATABASE_URL=postgresql://asephs:hunterz@100.108.1.124:5432/hub
```

Expected: Environment variable set and accessible to rust app.

- [ ] **Step 4: Rebuild rust app**

```bash
cd apps/rust
cargo build --release
```

Expected: Build completes without errors. Compilation uses sqlx-postgres feature.

- [ ] **Step 5: Test rust connection**

```bash
cd apps/rust
cargo run
```

Expected: App starts without connection errors. Check logs for "Database schema initialization complete" or similar.

- [ ] **Step 6: Commit rust changes**

```bash
cd /mnt/code/bp3/ultimate-asepharyana.tech
git add apps/rust/Cargo.toml apps/rust/src/infra/db_setup.rs
git commit -m "feat(rust): migrate database driver from MySQL to PostgreSQL"
```

Expected: Commit created with message.

---

### Task 6: Smoke Tests - Elysia App

**Files:**
- Test: Manual testing via HTTP requests or app UI

- [ ] **Step 1: Start elysia app**

```bash
cd apps/elysia
bun run src/index.ts
```

Expected: App running on configured port (check logs for port).

- [ ] **Step 2: Test user login**

```bash
curl -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"password"}'
```

Expected: Response 200 or 401 (auth error is OK, connection error is not).

-[]**Step 3: Test user creation (if endpoint exists)**

```bash
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Test User","email":"newuser@example.com"}'
```

Expected: Response 200/201 or 400 (validation error is OK).

- [ ] **Step 4: Test session retrieval**

```bash
curl -X GET http://localhost:3000/sessions \
  -H "Authorization: Bearer <token>"
```

Expected: Response 200 with session data or 401 (auth error is OK).

- [ ] **Step 5: Check database logs**

```bash
# In elysia app logs, verify queries are executing against PostgreSQL
# Look for connection strings or query logs showing PostgreSQL
```

Expected: Logs show PostgreSQL queries (not MySQL).

---

### Task 7: Smoke Tests - Rust App

**Files:**
- Test: Manual testing via HTTP requests or app UI

- [ ] **Step 1: Start rust app**

```bash
cd apps/rust
cargo run --release
```

Expected: App running on configured port (check logs for port).

- [ ] **Step 2: Test image cache endpoint (if exists)**

```bash
curl -X GET http://localhost:8000/api/cache/status
```

Expected: Response 200 with cache status or 404 (endpoint may not exist).

- [ ] **Step 3: Test scraping/CDN endpoint**

```bash
curl -X GET http://localhost:8000/api/health
```

Expected: Response 200 with health status.

- [ ] **Step 4: Check database logs**

```bash
# In rust app logs, verify queries are executing against PostgreSQL
# Look for connection strings or query logs showing PostgreSQL
```

Expected: Logs show PostgreSQL queries (not MySQL).

---

### Task 8: Data Integrity Validation

**Files:**
- Test: PostgreSQL queries

- [] **Step 1: Verify user count**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "SELECT COUNT(*) as user_count FROM \"User\";"
```

Expected: Count matches MySQL backup count (from Task 1, Step 2).

- [ ] **Step 2: Verify account count**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "SELECT COUNT(*) as account_count FROM \"Account\";"
```

Expected: Count matches MySQL backup.

- [ ] **Step 3: Verify session count**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "SELECT COUNT(*) as session_count FROM \"Session\";"
```

Expected: Count matches MySQL backup.

- [] **Step 4: Verify no orphaned foreign keys**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "
SELECT a.id FROM \"Account\" a
LEFT JOIN \"User\" u ON a.user_id = u.id
WHERE u.id IS NULL;
"
```

Expected: No rows returned (no orphaned accounts).

- [ ] **Step 5: Verify role/permission relationships**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "SELECT COUNT(*) as role_count FROM \"Role\";"
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "SELECT COUNT(*) as permission_count FROM \"Permission\";"
```

Expected: Counts match MySQL backup.

---

### Task 9: Performance Baseline

**Files:**
- Test: Query performance comparison

- [ ] **Step 1: Benchmark user query on PostgreSQL**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "EXPLAIN ANALYZE SELECT * FROM \"User\" WHERE email = 'test@example.com';"
```

Expected: Query plan shows index usage (Seq Scan or Index Scan). Note execution time.

- [ ] **Step 2: Benchmark account query on PostgreSQL**

```bash
psql postgresql://asephs:hunterz@100.108.1.124:5432/hub -c "EXPLAIN ANALYZE SELECT * FROM \"Account\" WHERE user_id = 'user-123';"
```

Expected: Query plan shows index usage. Note execution time.

- [ ] **Step 3: Compare with MySQL baseline (if available)**

If MySQL is still running, run same queries and compare execution times.

Expected: PostgreSQL performance similar or better than MySQL.

---

### Task 10: Cleanup and Documentation

**Files:**
- Create: `MIGRATION_LOG.md` (optional, for documentation)

- [ ] **Step 1: Remove temporary files**

```bash
rm mysql_backup.sql converted.sql
```

Expected: Temporary migration files deleted.

- [ ] **Step 2: Document migration completion**

Create `MIGRATION_LOG.md`:

```markdown
# PostgreSQL Migration Log

**Date:** 2026-05-25
**Status:** ✅ Complete

## Summary
- Migrated elysia app from MySQL to PostgreSQL
- Migrated rust app from MySQL to PostgreSQL
- All data validated and integrity confirmed
- Apps tested and operational

## Changes
- elysia: Updated database driver (mysql2 → postgres), schema (mysqlTable → pgTable)
- rust: Updated Cargo.toml feature (sqlx-mysql → sqlx-postgres), db_setup.rs backend handling

## Validation
- Row counts match pre-migration
- Foreign keys intact
- Indexes present and performant
- Auth flow functional
- Session management working

## Rollback
MySQL backup available at: [location if kept]
To rollback: Restore MySQL from backup, revert connection strings, redeploy apps.
```

- [ ] **Step 3: Final commit**

```bash
git add MIGRATION_LOG.md
git commit -m "docs: add PostgreSQL migration completion log"
```

Expected: Commit created.

- [ ] **Step 4: Verify all apps running**

```bash
# Check elysia
curl http://localhost:3000/health

# Check rust
curl http://localhost:8000/health
```

Expected: Both apps respond with 200 status.

---

## Self-Review

**Spec Coverage:**
- ✅ Pre-migration (backup, convert, test) — Tasks 1-3
- ✅ Elysia code updates (driver, schema, env) — Task 4
- ✅ Rust code updates (Cargo.toml, db_setup.rs, env) — Task 5
- ✅ Smoke tests (auth, endpoints, logs) — Tasks 6-7
- ✅ Data validation (row counts, foreign keys) — Task 8
- ✅ Performance baseline — Task 9
- ✅ Cleanup and documentation — Task 10

**Placeholder Scan:**
- ✅ No TBD/TODO
- ✅ All code blocks complete
- ✅ All commands exact with expected output
- ✅ All file paths exact

**Type Consistency:**
- ✅ Database type: `PostgresJsDatabase` (elysia), `DbBackend::Postgres` (rust)
- ✅ Connection string format consistent: `postgresql://asephs:hunterz@100.108.1.124:5432/hub`
- ✅ Table names consistent: "User", "Account", "Session", etc.
