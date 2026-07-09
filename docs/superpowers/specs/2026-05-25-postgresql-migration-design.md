# PostgreSQL Migration Design
**Date:** 2026-05-25  
**Scope:** Migrate elysia (Drizzle ORM + MySQL) and rust (SeaORM + MySQL) apps to PostgreSQL  
**Approach:** Direct cutover (Approach B)  
**Connection String:** `postgresql://asephs:hunterz@100.108.1.124:5432/hub`

---

## Current State

### Elysia App
- **ORM:** Drizzle ORM v0.45.2
- **Driver:** mysql2 v3.20.0
- **Schema Location:** `apps/elysia/src/db/lib/schema.ts`
- **Tables:** users, accounts, sessions, roles, permissions, userRoles, and related junction tables
- **Database File:** `apps/elysia/src/db/lib/database.ts`

### Rust App
- **ORM:** SeaORM v1.1.19
- **Feature:** sqlx-mysql
- **DB Setup:** `apps/rust/src/infra/db_setup.rs`
- **Tables:** ImageCache (primary table managed by app)
- **Config:** Environment-based connection string

### Apps NOT Migrating
- 9router (uses SQLite runtime, excluded per requirements)
- nextjs (frontend, minimal DB usage)
- leptos, solidjs, rust-auth (frontends, no DB)

---

## Migration Steps

### Phase 1: Pre-Migration (Preparation)
1. **Backup MySQL**
   ```bash
   mysqldump -u <user> -p <db> > mysql_backup.sql
   ```
2. **Convert MySQL dump to PostgreSQL**
   - Use `pgloader` or manual conversion for schema compatibility
   - Handle type conversions:
     - `INT` → `INTEGER`
     - `VARCHAR(n)` → `VARCHAR(n)` (PostgreSQL compatible)
     - `DATETIME` → `TIMESTAMP`
     - `AUTO_INCREMENT` → `SERIAL` or `BIGSERIAL`
   - Verify indexes and foreign keys convert correctly
3. **Test import into target PostgreSQL**
   ```bash
   psql postgresql://asephs:hunterz@100.108.1.124:5432/hub < converted.sql
   ```
4. **Validate data integrity**
   - Row counts match MySQL
   - Indexes exist
   - Foreign key constraints enforced

### Phase 2: Code Updates

#### Elysia App
1. **Update `apps/elysia/src/db/lib/database.ts`**
   - Replace `mysql2` import with `postgres` driver
   - Change Drizzle dialect from `drizzle-orm/mysql2` to `drizzle-orm/postgres`
   - Update connection string format

2. **Update `apps/elysia/src/db/lib/schema.ts`**
   - Replace `drizzle-orm/mysql-core` imports with `drizzle-orm/postgres-core`
   - Change `mysqlTable` to `pgTable`
   - Adjust column types if needed (e.g., `int` → `integer`)

3. **Update `apps/elysia/package.json`**
   - Replace `mysql2` with `pg` (PostgreSQL driver)
   - Keep `drizzle-orm` and `drizzle-kit` versions

4. **Update environment/config**
   - Change `DATABASE_URL` to PostgreSQL connection string

#### Rust App
1. **Update `apps/rust/Cargo.toml`**
   - Replace `sqlx-mysql` feature with `sqlx-postgres` in sea-orm dependency
   - Add `postgres` feature if needed

2. **Update `apps/rust/src/infra/db_setup.rs`**
   - Change `DbBackend::MySql` check to `DbBackend::Postgres`
   - Adjust SQL syntax for PostgreSQL (e.g., index creation)
   - Update error code handling (PostgreSQL uses different error codes)

3. **Update config/environment**
   - Change `DATABASE_URL` to PostgreSQL connection string

### Phase 3: Cutover (Execution)
1. **Stop all apps**
   ```bash
   # Stop elysia
   # Stop rust app
   ```
2. **Export MySQL data**
   ```bash
   mysqldump -u <user> -p <db> > final_backup.sql
   ```
3. **Convert and import to PostgreSQL**
   ```bash
   # Convert dump
   # Import to PostgreSQL
   psql postgresql://asephs:hunterz@100.108.1.124:5432/hub < converted.sql
   ```
4. **Deploy updated apps**
   - Deploy elysia with PostgreSQL driver
   - Deploy rust with PostgreSQL feature
5. **Verify connectivity**
   - Test database queries from both apps
   - Check auth flow (users table)
   - Verify session management

### Phase 4: Validation
1. **Smoke tests**
   - User login/logout
   - Session creation and retrieval
   - Role/permission queries
   - ImageCache operations (rust app)
2. **Data integrity checks**
   - Row counts match pre-migration
   - No orphaned foreign keys
   - Indexes performing as expected
3. **Performance baseline**
   - Compare query times MySQL vs PostgreSQL
   - Monitor connection pool usage

### Phase 5: Rollback Plan (if needed)
1. **Stop apps**
2. **Restore MySQL from backup**
   ```bash
   mysql -u <user> -p <db> < mysql_backup.sql
   ```
3. **Revert connection strings in apps**
4. **Redeploy with MySQL drivers**
5. **Restart apps**

---

## Technical Details

### Elysia Driver Change
**Before:**
```typescript
import { drizzle } from 'drizzle-orm/mysql2'
import { createPool } from 'mysql2/promise'
```

**After:**
```typescript
import { drizzle } from 'drizzle-orm/postgres-js'
import postgres from 'postgres'
```

### Rust Feature Change
**Before:**
```toml
sea-orm = { version = "1.1.19", features = ["sqlx-mysql", ...] }
```

**After:**
```toml
sea-orm = { version = "1.1.19", features = ["sqlx-postgres", ...] }
```

---

## Risk Assessment

| Risk | Mitigation |
|------|-----------|
| Data loss during migration | Full MySQL backup before cutover; test import first |
| App downtime | Cutover during low-traffic window; rollback plan ready |
| Connection string misconfiguration | Test connection before deploying apps |
| Schema incompatibilities | Pre-test conversion; validate indexes/constraints |
| Performance regression | Baseline MySQL performance; monitor PostgreSQL after cutover |

---

## Success Criteria

- ✅ All data migrated to PostgreSQL (row counts match)
- ✅ Elysia app connects and queries work
- ✅ Rust app connects and queries work
- ✅ Auth flow functional (login/logout)
- ✅ No orphaned foreign keys
- ✅ Indexes present and performant
- ✅ Rollback plan tested and documented

---

## Timeline

- **Pre-migration:** 30 min (backup, convert, test)
- **Code updates:** 1-2 hours (driver changes, testing)
- **Cutover:** 15-30 min (stop apps, migrate data, restart)
- **Validation:** 30 min (smoke tests, data checks)
- **Total:** ~3-4 hours (including buffer)
