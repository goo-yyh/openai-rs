# OpenAPI Contract

`openai-rs` keeps a checked-in snapshot of the official documented OpenAPI spec so endpoint drift is caught by tests, not by users.

Snapshot assets:

- `codegen/openapi/openapi.documented.yml`
- `codegen/openapi/openapi.documented.json`
- `codegen/openapi/metadata.json`
- `codegen/openapi/contracts.json`

What is validated:

- generated endpoint catalog entries against the documented OpenAPI snapshot
- handwritten core endpoints such as `responses.create`, `chat.completions.create`, `embeddings.create`, `files.*`, `uploads.*`, `models.*`, and `images.*`
- `method + normalized path template` alignment
- success status presence
- high-value request / response body-shape assertions for core paths

What is explicitly tracked as an exception:

- endpoints that are intentionally exposed by the SDK but are not currently present in the documented upstream spec
- the current exception set is intentionally tiny and is kept in `codegen/openapi/contracts.json`

## Updating the snapshot

Run:

```bash
bash ./scripts/update_openapi_snapshot.sh
```

Then rerun:

```bash
cargo test --test openapi_contract --all-features
bash ./scripts/check-downstream-canaries.sh
```

If the upstream documented spec changed in a way that affects the SDK:

1. update endpoint implementations or exception tracking
2. refresh `codegen/openapi/contracts.json` if the body-shape assertions need to move
3. rerun the full CI-quality validation set
