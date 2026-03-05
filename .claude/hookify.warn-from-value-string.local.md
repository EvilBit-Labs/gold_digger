---
name: warn-from-value-string
enabled: true
event: file
pattern: from_value\s*::\s*<\s*String\s*>
action: warn
---

**NEVER use `from_value::<String>()`** -- this causes panics on NULL and non-string types.

**Use TypeTransformer instead:**

- CSV/TSV: `TypeTransformer::value_to_string(&value)?`
- JSON: `TypeTransformer::value_to_json(&value)?`
- Full row to strings: `TypeTransformer::row_to_strings(row)?`
- Full row to JSON map: `TypeTransformer::row_to_json(row)?`

See `src/type_transformer.rs` for the safe conversion API.
