# gauge-uf-app-e2e

Leptos host that mounts Gauge permission pages for Playwright. Lab-only:
insecure session cookies, `POST /api/test/seed-data`, harness auth (no lepton sign-in).

## Run

From the repository root:

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-gauge-uf-app
cd gauge-uf-app-e2e/end2end && npm ci && npx playwright install chromium && cd ../..
cargo leptos end-to-end --project gauge-uf-app-e2e
```

Host listens on `127.0.0.1:3150`. Do not Ctrl-C; the run exits when Playwright finishes.

The lab host mounts the same page components as `PermissionRoutes`, without `Lazy`
(wasm-split Lazy under `ParentRoute` panics on hydrate in the current Leptos pin).
Production hosts keep `PermissionRoutes` + `--split` for code-splitting.

## Seed

`POST /api/test/seed-data` with JSON
`{ "auth": "admin" | "requestor" | "outsider" | "unverified" | "anonymous", "refresh_requests": true }`.

Default `refresh_requests` is false (avoids wiping allow-list mid-suite). Request specs
pass `true` so the host revokes requestor grants/membership, then mints fresh PENDING
rows as **requestor** (outsider unauthorized-viewer still works).

## Scenario catalog (implemented)

Auth: `e2e.auth.anonymous_gate`, `e2e.auth.unverified_email_gate`

Permissions index: `e2e.perm.index.load_happy`, `search_match`, `search_no_match`,
`open_row`, `create_cta`

Domain / permission create: `e2e.domain.create.happy` (navigates to domain detail),
`save_happy`, `add_owner`, `remove_owner`, `owner_picker_no_admin`, `no_admin`,
`empty_name`; `e2e.perm.create.happy` (domain Select), `domain_required_client`,
`no_domains`, `domains_error`, `no_admin`

Permission detail: `load_happy`, `allow_empty`, `save_happy`, `save_no_admin`, `grant_user`,
`grant_group`, `picker_search`, `revoke_user`, `revoke_group`, `revoke_cancel`,
`picker_no_admin`, `request_btn_visible`, `request_btn_hidden`, `request_submit_happy`
(PENDING in My Requests), `request_empty_reason`, `not_found`, `delete_happy`,
`history_entries` (timeline + description stamp), `history_relation_grant` (Added +
relation row after grant), `history_acl_deny` (outsider → Not authorized MessageBar)

Groups: index load/search/open/create CTA; create `happy` / `no_admin` / `empty_name` /
`dup_name`; detail `load_happy`, `save_happy`, `save_no_admin`, `add_member_user`,
`add_member_group`, `add_owner` (Open owner actions), `remove_owner`,
`owner_picker_no_admin`, `member_picker_no_admin`, `request_submit_happy`,
`request_btn_hidden`, `request_empty_reason`, `not_found`, `delete_happy`,
`history_entries`,
`history_relation_member` (fresh group + Added member relation row),
`history_acl_deny` (outsider → Not authorized MessageBar)

Requests: dual inbox + target kind rows; `deny_permission` / `deny_group` (DENIED + no
grant/membership); `approve_permission` / `approve_group`; `terminal_denied` /
`terminal_approved`; `no_actions_requestor`, `no_actions_outsider`, `unauthorized_viewer`,
`not_found`

Search (K12): `e2e.search.principals_initial`, `principals_query_user`,
`principals_query_group`, `principals_no_admin`

### Deferred (per-ID)

| ID | Why deferred |
|----|--------------|
| `e2e.auth.session_server_fn_deny` | Layout anonymous gate + domain `unauthenticated_*_sad`; mid-session MessageBar not seeded |
| `e2e.perm.index.empty` / `load_error` | Needs empty-catalog or fault-inject seed; UI-only seams, no domain substitute |
| `e2e.group.index.empty` / `load_error` | Same |
| `e2e.domain.create.server_error` | No injectable create fault |
| `e2e.perm.create.dup_name` | Domain integ `duplicate_*` covers name conflict |
| `e2e.perm.detail.save_non_owner` | Domain `non_owner_*` / `owners_group_member_cannot_mutate_*` |
| `e2e.perm.detail.delete_no_admin` / `grant_no_admin` | Higgs deny pattern covered by `save_no_admin` + `picker_no_admin` |
| `e2e.perm.detail.load_error` | Needs fault-inject on `get_permission`; UI MessageBar only — no domain integ stand-in |
| `e2e.perm.detail.revoke_fail` | No revoke-failure sad in domain suites; needs injectable grant-store fault |
| `e2e.perm.detail.request_overlong` / `request_already_has` | Domain request sad integ |
| `e2e.perm.detail.outsider_omit` | Domain `permission_detail_allow_list_editor_matrix` (outsider empty allow_list) |
| `e2e.group.detail.save_non_owner` / `add_owner_fail` | Domain owner privacy integ |
| `e2e.group.detail.remove_owner_cancel` | Cancel-without-mutation pattern covered by `e2e.perm.detail.revoke_cancel` |
| `e2e.perm.detail.history_empty` / `e2e.group.detail.history_empty` | History row content covered by `history_entries` + domain `history_integration` |
| `e2e.req.index.review_empty` / `mine_empty` | Needs wipe-all requests seed mode |
| `e2e.req.detail.decide_unauthorized` | Overlaps `unauthorized_viewer` + `no_actions_outsider` |
| Loading-only spinners | Smokes, not primary |

Domain service contracts stay in `gauge` `permission_domain_contract` / privacy suites.
Full lepton credential matrices: `lepton-auth-ui-e2e`. Full-stack composition: `uf-embedded-e2e`.
