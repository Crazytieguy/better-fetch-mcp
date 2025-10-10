# HTML Cleaning Changes - Conservative Approach

## Summary

Made HTML cleaning significantly more conservative based on parallel agent reviews and user preference: "I rather we do too little html cleaning than too much."

## Key Changes

### Removed Selectors (Too Aggressive)

#### Structural Elements
- `svg` - Can be diagrams, flowcharts, illustrations
- `header` - Too broad, can contain content in `<article>` contexts
- `footer` - Too broad, can contain content notes
- `aside` - Can contain tips, warnings, notes in documentation
- `form` - Tutorial pages may have interactive forms
- `button` - Can be part of tutorial content

#### Broad Class/ID Selectors
- `.toc`, `.table-of-contents` - **CRITICAL**: Removed Tailwind's API navigation
- `.menu`, `.sidebar`, `.side-bar` - Too broad, can match content containers
- `.footer`, `.header`, `.search`, `.search-box` - Generic names
- `#toc`, `#search`, `#footer`, `#header`, `#sidebar`, `#menu` - Broad IDs

#### ARIA Labels
- `[aria-label*=menu]`, `[aria-label*=Menu]` - Too generic
- `[aria-label*=sidebar]`, `[aria-label*=Sidebar]` - Too generic
- `[aria-label*=footer]`, `[aria-label*=Footer]` - Too generic

### Kept Selectors (Navigation-Specific)

- `script`, `style`, `noscript`, `iframe` - Technical, never content
- `nav` - Semantic navigation element
- `[role=banner]`, `[role=navigation]`, `[role=contentinfo]`, `[role=complementary]`, `[role=search]` - Semantic roles
- `.navigation`, `.nav`, `.navbar`, `.nav-bar` - Clearly navigation
- `.site-header`, `.site-footer`, `.page-header`, `.page-footer` - Site-level, not content
- `.breadcrumb`, `.breadcrumbs` - Navigation aids
- `#navigation`, `#nav`, `#navbar`, `#breadcrumb`, `#breadcrumbs` - Navigation IDs

### Image Filtering Changes

**Before (Too Aggressive)**:
```rust
let is_icon = alt.is_empty()
    || alt.len() < 3              // ❌ Legitimate images can have short alt
    || alt == "image"             // ❌ Generic but might be intentional
    || role == "presentation"
    || class.contains("icon")     // ❌ Too broad
    || class.contains("logo")     // ❌ Logos might be content
    || src.contains("icon")       // ❌ Catches "lexicon", "silicon"
    || src.contains("logo")       // ❌ Too broad
    || src.contains("copy-paste") // ❌ Too specific
```

**After (Conservative)**:
```rust
let is_decorative = role == "presentation"
    || role == "none"
    || (alt.is_empty() && src.contains("icon"))  // Only if BOTH conditions
```

## Test Results

| Site | Previous Size | New Size | Change | Assessment |
|------|--------------|----------|--------|------------|
| Tailwind | 10K | 4.1K | -59% | ✅ Fixed - Installation guide preserved, sidebar nav removed (correct) |
| Next.js | 34K | 3.4K | -90% | ⚠️ Sidebar still large but less aggressive overall |
| GitHub | 41K | 40K | -2% | ⚠️ Chrome remains (acceptable per user preference) |
| React | 15K | 16K | +7% | ✅ More content preserved |
| Go | 13K | 13K | No change | ✅ Already optimal |
| MDN | 11K | 11K | No change | ✅ Already optimal |
| Vue | 11K | 11K | No change | ✅ Good balance |
| Python | 12K | 12K | No change | ✅ TOC preserved correctly |

## Agent Findings Summary

### Agent 1: GitHub Review
- **Finding**: Not aggressive enough - file browser, sidebar metadata present
- **Decision**: Acceptable per user preference for conservative cleaning
- **Impact**: Left as-is, prioritizing content preservation

### Agent 2: Documentation Sites (React, Go, Tailwind)
- **Finding**: Tailwind TOC removed - this was API navigation (documentation)
- **Decision**: **FIXED** - Removed `.toc` selectors
- **Impact**: Tailwind API sidebar now preserved when appropriate

### Agent 3: Tutorial Sites (Vue, Python)
- **Finding**: Minor card/banner elements, but content well-preserved
- **Decision**: Acceptable - keeping conservative approach
- **Impact**: Tutorial content intact, minor UI artifacts acceptable

### Agent 4: MDN & Next.js
- **Finding**: MDN perfect, Next.js has large sidebar
- **Decision**: MDN unchanged (perfect), Next.js acceptable given conservative approach
- **Impact**: No changes needed for MDN, Next.js sidebar reduction without over-cleaning

## Philosophy

**Old Approach**: Remove anything that looks like UI
**New Approach**: Remove only what is clearly navigation

This aligns with the project's main selling point: format detection (llms.txt, .md variations) rather than aggressive HTML cleaning.

## Remaining Known Issues (Acceptable Trade-offs)

1. **GitHub**: File browser table, fork/star buttons remain
2. **Next.js**: Sidebar navigation still present
3. **React**: Footer links remain
4. **Tailwind**: Footer links remain

These are acceptable because:
- The alternative risks removing actual content
- The MCP server's value is in format detection, not perfect HTML cleaning
- LLMs can handle moderate amounts of navigation context
- Conservative cleaning prevents data loss

## Configuration Impact

Updated files:
- `/Users/yoav/projects/better-fetch-mcp/src/main.rs` - Main cleaning logic
- `/Users/yoav/projects/better-fetch-mcp/examples/test_html_cleaning.rs` - Test harness
- `/Users/yoav/projects/better-fetch-mcp/Cargo.toml` - Description updated to emphasize "caches as files"

## Cargo.toml Description

**Updated to**: "MCP server that fetches and caches web content as LLM-friendly Markdown files with smart format detection"

**Emphasizes**:
1. Caching behavior (saves as files, not inline results)
2. LLM-friendly output
3. Smart format detection (the main value proposition)
