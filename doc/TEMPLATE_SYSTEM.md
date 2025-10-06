# IronDrop Template & UI System Documentation (v2.6.4)

**Status**: Production ready (v2.6.4)

**Audience**: Backend & Frontend Developers, UI/UX Engineers, Integrators

**Purpose**: Explain the native template engine, variable & conditional system, modular asset architecture, card-based UI components, light button system, customization points, security model, and performance characteristics.

---

## 1. Overview

IronDrop ships with a zero‑dependency template engine designed for:

- Consistent professional UI across directory listings, upload interface, and error pages
- Fast variable interpolation with optional conditional blocks
- Embedded assets (HTML/CSS/JS + favicons) compiled directly into the binary for portable deployment
- Secure rendering with HTML escaping & controlled static asset routing

The system emphasizes simplicity (no runtime parsing of template files from disk) and predictable performance—ideal for a single‑binary distribution model.

---

## 2. Architecture

```
 Request ─┬──────────────▶ Route Layer (http.rs)
         │                    │
         │ (HTML Page Route)  │ (Static Asset Route /_irondrop/static/...)
         ▼                    ▼
   TemplateEngine          get_static_asset()
         │                    │
         ▼                    ▼
 Load Embedded Str        Return (content, mime)
 Interpolate Vars
 Apply Conditionals
         │
         ▼
     HTML Output
```

### Key Source File
`src/templates.rs` – Implements:
- Embedded constants (`include_str!` / `include_bytes!`)
- Template registry (HashMap<String, String>)
- Variable interpolation & conditional block evaluation
- Static asset & favicon retrieval
- Page-specific render helpers

### Render Helper Methods
| Method | Purpose |
|--------|---------|
| `render_directory_listing` | Directory index page assembly (entries + state) |
| `render_error_page` | Professional error pages with extended metadata |
| `render_upload_page` | Full upload page (drag & drop UI) |
| `get_upload_form` | Inline reusable upload form snippet |
| `get_static_asset` | Returns CSS/JS asset content + mime |
| `get_favicon` | Returns embedded icon bytes + mime |

---

## 3. Template Features

### 3.1 Variable Interpolation
Syntax: `{{VARIABLE_NAME}}` replaced with string value. All values inserted via higher‑level functions are pre‑escaped for HTML where appropriate.

### 3.2 Conditional Blocks
Minimal inline logic without loops or expressions:

```
{{#if UPLOAD_ENABLED}}
  <div class="upload-form">...</div>
{{/if}}
```

Condition is true only if the variable exists and equals the string `"true"`.

### 3.3 Escaping Strategy
- File / path display: Escaped with `html_escape()` (replaces `& < > " '`).
- URL generation: Percent‑encoding of a controlled subset (space, quotes, hash, percent, angle brackets, question).

### 3.4 Embedded Assets
All templates + CSS/JS + favicons are embedded at compile time for:
- Zero runtime I/O
- Immutable integrity
- Single‑binary portability

### 3.5 No Runtime File Reads
`TemplateEngine::new()` loads all HTML templates into an in‑memory map; further disk access is unnecessary.

### 3.6 Deterministic Performance
Interpolation executes O(n) over template size, using straightforward `String::replace` calls (fast for small, fixed templates).

---

## 4. Available Templates & Variables

### 4.1 Directory Listing (`directory/index.html`)
| Variable | Description |
|----------|-------------|
| `PATH` | Normalized display path (e.g. `/`, `/docs/`) |
| `ENTRY_COUNT` | Total visible entries (including directories) |
| `UPLOAD_ENABLED` | `true` / `false` to toggle upload UI conditionals |
| `CURRENT_PATH` | Raw path used for constructing upload/query suffix |
| `QUERY_UPLOAD_SUFFIX` | Prebuilt `?upload_to=...` or empty string |
| `ENTRIES` | Injected `<tr>...</tr>` rows for file table |

Conditional Blocks: `{{#if UPLOAD_ENABLED}} ... {{/if}}`

### 4.2 Error Page (`error/page.html`)
| Variable | Description |
|----------|-------------|
| `ERROR_CODE` | HTTP status code (e.g., 404) |
| `ERROR_MESSAGE` | Reason phrase (e.g., `Not Found`) |
| `ERROR_DESCRIPTION` | Human‑readable explanation |
| `REQUEST_ID` | Lightweight pseudo identifier for correlation |
| `TIMESTAMP` | System timestamp at render time |

### 4.3 Upload Page (`upload/page.html`)
| Variable | Description |
|----------|-------------|
| `PATH` | Target path / context for uploads |

### 4.4 Upload Form Snippet (`upload/form.html`)
(Currently variable‑free; intended for inclusion inside other templates.)

---

## 5. Static Asset Routing

Served through controlled paths (example mapping):

| Request Path | Engine Key | MIME |
|--------------|-----------|------|
| `/_irondrop/static/common/base.css` | `common/base.css` | `text/css` |
| `/_irondrop/static/directory/styles.css` | `directory/styles.css` | `text/css` |
| `/_irondrop/static/directory/script.js` | `directory/script.js` | `application/javascript` |
| `/_irondrop/static/error/styles.css` | `error/styles.css` | `text/css` |
| `/_irondrop/static/error/script.js` | `error/script.js` | `application/javascript` |
| `/_irondrop/static/upload/styles.css` | `upload/styles.css` | `text/css` |
| `/_irondrop/static/upload/script.js` | `upload/script.js` | `application/javascript` |

Favicon assets are similarly handled (e.g. `/favicon.ico`).

---

## 6. Security Model

| Concern | Mitigation |
|---------|------------|
| Path Traversal | Only embedded assets served; no arbitrary filesystem access in template layer |
| HTML Injection | File & path variables HTML‑escaped; controlled variable set |
| Asset Tampering | Compile‑time embedding prevents runtime modification |
| Template Injection | No user‑supplied template content; no expression evaluation |
| XSS via Conditionals | Conditional logic only checks equality to literal `true` |

Additional Safeguards:
- Restrictive asset key match in `get_static_asset()`
- No dynamic include / partial expansion (reduces injection surface)

---

## 7. Performance Characteristics

| Aspect | Notes |
|--------|-------|
| Render Time | Sub‑millisecond on typical hardware (small constant templates) |
| Memory Footprint | A few KB per template; loaded once at startup |
| Allocation Pattern | Initial load + per‑render cloned `String` (kept simple for clarity) |
| Scalability | Sufficient for expected request volumes (I/O dominated workloads) |

Potential Future Optimizations (not required presently):
- Pre‑tokenization to avoid repeated string scanning
- Reusable output buffers per thread
- Optional tiny LRU if dynamic templates introduced later

---

## 8. Testing & Validation

Relevant tests:
- `tests/template_embedding_test.rs` – Ensures embedded templates & variables render correctly
- `tests/comprehensive_test.rs` – Indirect verification through directory & error responses
- `tests/upload_integration_test.rs` – Upload page & form availability

What is tested:
- Directory listing HTML contains expected rows & structure
- Error pages contain correct status text and sanitized description
- Static assets served with accurate Content-Type

---

## 9. Customization & Theming

Primary design tokens live in `templates/common/base.css` and influence all pages:

```css
:root {
  --bg-primary: #0a0a0a;
  --bg-secondary: #1a1a1a;
  --bg-tertiary: #2a2a2a;
  --text-primary: #e5e5e5;
  --text-secondary: #b0b0b0;
  --accent: #ffffff;
  --radius-sm: 6px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --shadow-minimal: 0 1px 2px rgba(0, 0, 0, 0.02);
  --shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
  /* ... */
}
```

### UI Component System
The design system is built around reusable components:

#### Button Classes
- `.btn-light` - Primary button style with light appearance and dark shadows
- `.btn-primary` - Accent gradient buttons for primary actions
- `.btn-secondary` - Secondary action buttons with glass effect
- `.btn-ghost` - Minimal transparent buttons

#### Card Components
All card-like elements use the base `.card` class for consistency:
- Error pages: `<div class="card error-container">`
- Monitor metrics: `<div class="card metric-card">`
- Upload areas: `<div class="card">`

This ensures uniform styling with:
- 20px border radius
- Consistent hover effects (`translateY(-1px)`)
- Unified shadow system
- Responsive behavior

Customization Steps:
1. Adjust global tokens in `base.css` (preferred – cascades across modules)
2. Add page‑specific overrides in each module's `styles.css`
3. Insert new conditional blocks guarded by Boolean variables as needed
4. Expose new variables via render helper functions (modify `templates.rs`)

Adding a New Template:
1. Create HTML file under `templates/<name>/`.
2. Add `const` with `include_str!` in `templates.rs`.
3. Insert into `TemplateEngine::new()` registry.
4. Add static assets (CSS/JS) & map them in `get_static_asset()`.
5. Provide a specialized render helper if passing structured data.
6. Add tests verifying presence of expected markers.

---

## 10. Example: Adding a Badge Section Conditionally

Template snippet:
```html
{{#if SHOW_BADGE}}
  <div class="badge">BETA</div>
{{/if}}
```

Rust usage:
```rust
let mut vars = HashMap::new();
vars.insert("SHOW_BADGE".into(), "true".into());
let html = template_engine.render("directory_index", &vars)?;
```

---

## 11. Roadmap / Future Enhancements

| Feature | Rationale |
|---------|-----------|
| Partials / Includes | Reuse of common fragments without duplication |
| Loop Constructs | Dynamic tables without pre‑concatenating HTML strings |
| Streaming Renderer | Avoid allocating large intermediate strings for very large templates |
| Theming Profiles | Switchable light/dark or branded themes via config |
| Asset Fingerprinting | Long‑term caching with content hashes (static CDNs) |

All are intentionally deferred to preserve current simplicity & zero‑dependency footprint.

---

## 12. Integration Points (Cross‑Reference)

| Component | Interaction |
|-----------|-------------|
| `http.rs` | Routes HTML page responses & static asset paths |
| `fs.rs` | Supplies directory entries for listing rendering |
| `upload.rs` | Provides runtime validation feeding upload UI decisions |
| `response.rs` | Wraps final HTML into HTTP responses with headers |
| `error.rs` | Supplies error context to error page renderer |

---

## 13. Troubleshooting

| Symptom | Cause | Resolution |
|---------|-------|-----------|
| Missing CSS/JS | Asset key mismatch | Verify path mapping in `get_static_asset()` |
| Conditional Block Always Hidden | Variable not set to literal `true` | Insert `variable.insert("VAR".into(), "true".into());` |
| Raw `{{VAR}}` Appears | Variable absent | Add to variables map before render |
| Incorrect Escaping | Manually inserted raw HTML | Pre-escape or extend engine with safe variant |

---

## 14. Reference Snippets

### 14.1 Rendering Directory Listing
```rust
let html = engine.render_directory_listing(
    "/",            // display path
    &entries,        // Vec<(name, size, date)>
    entries.len(),   // count
    uploads_enabled, // bool
    current_path,    // raw path
)?;
```

### 14.2 Serving a Static Asset
```rust
if let Some((content, mime)) = engine.get_static_asset("directory/styles.css") {
    // write response with mime
}
```

### 14.3 Error Page
```rust
let err_html = engine.render_error_page(404, "Not Found", get_error_description(404))?;
```

---

## 15. See Also

- [Architecture Documentation](./ARCHITECTURE.md#template--ui-system)  
- [Upload Integration Guide](./UPLOAD_INTEGRATION.md)  
- [Multipart Parser Documentation](./MULTIPART_README.md)  
- [Security Fixes Documentation](./SECURITY_FIXES.md)  
- [Documentation Index](./README.md)

---

*This document is part of the IronDrop v2.6.4 documentation suite and will evolve with future template system enhancements.*

Return to documentation index: [./README.md](./README.md)
