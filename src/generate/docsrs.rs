use anyhow::{Context, Result};

/// Kind of API item scraped from docs.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Struct,
    Enum,
    Trait,
    Function,
    Macro,
    DeriveMacro,
    AttrMacro,
    TypeAlias,
    Constant,
}

impl ItemKind {
    /// Section header name for grouping
    pub fn section_name(&self) -> &'static str {
        match self {
            ItemKind::Struct => "STRUCTS",
            ItemKind::Enum => "ENUMS",
            ItemKind::Trait => "TRAITS",
            ItemKind::Function => "FUNCTIONS",
            ItemKind::Macro | ItemKind::DeriveMacro | ItemKind::AttrMacro => "MACROS",
            ItemKind::TypeAlias => "TYPE ALIASES",
            ItemKind::Constant => "CONSTANTS",
        }
    }

    /// Sort order for section grouping
    pub fn sort_order(&self) -> u8 {
        match self {
            ItemKind::Struct => 0,
            ItemKind::Enum => 1,
            ItemKind::Trait => 2,
            ItemKind::Function => 3,
            ItemKind::Macro | ItemKind::DeriveMacro | ItemKind::AttrMacro => 4,
            ItemKind::TypeAlias => 5,
            ItemKind::Constant => 6,
        }
    }
}

/// A reference to an item found in the all.html page
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRef {
    pub name: String,
    pub kind: ItemKind,
    pub path: String,
    pub module_prefix: Option<String>,
}

/// Detail about a method on a struct/enum/trait
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodDetail {
    pub name: String,
    pub signature: String,
    pub description: String,
}

/// Detailed information about a single API item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDetail {
    pub signature: String,
    pub description: String,
    pub methods: Vec<MethodDetail>,
}

/// Complete scraped crate data
#[derive(Debug, Clone)]
pub struct ScrapedCrate {
    pub name: String,
    pub items: Vec<(ItemRef, ItemDetail)>,
}

/// Build the URL for the all.html page of a crate
pub fn build_all_url(crate_name: &str) -> String {
    let underscored = crate_name.replace('-', "_");
    format!(
        "https://docs.rs/{}/latest/{}/all.html",
        crate_name, underscored
    )
}

/// Build the URL for a specific item page
pub fn build_item_url(crate_name: &str, item_path: &str) -> String {
    let underscored = crate_name.replace('-', "_");
    format!(
        "https://docs.rs/{}/latest/{}/{}",
        crate_name, underscored, item_path
    )
}

/// Top-level entry point: scrape a crate from docs.rs
pub fn scrape_crate(crate_name: &str) -> Result<ScrapedCrate> {
    let items = fetch_all_items(crate_name)?;
    let mut scraped_items = Vec::new();

    for item in &items {
        let detail = match fetch_item_detail(crate_name, item) {
            Ok(d) => d,
            Err(_) => ItemDetail {
                signature: String::new(),
                description: String::new(),
                methods: Vec::new(),
            },
        };
        scraped_items.push((item.clone(), detail));

        // Rate limiting: 200ms between requests
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    Ok(ScrapedCrate {
        name: crate_name.to_string(),
        items: scraped_items,
    })
}

/// Fetch and parse the all.html page for a crate
fn fetch_all_items(crate_name: &str) -> Result<Vec<ItemRef>> {
    let url = build_all_url(crate_name);
    let body = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .with_context(|| format!("Failed to fetch {}", url))?
        .into_string()
        .with_context(|| format!("Failed to read body from {}", url))?;
    parse_all_html(&body, crate_name)
}

/// Fetch detail page for a single item
fn fetch_item_detail(crate_name: &str, item: &ItemRef) -> Result<ItemDetail> {
    let url = build_item_url(crate_name, &item.path);
    let body = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .with_context(|| format!("Failed to fetch {}", url))?
        .into_string()
        .with_context(|| format!("Failed to read body from {}", url))?;
    parse_item_html(&body, item.kind)
}

/// Parse all.html to extract item references (pure, testable)
pub fn parse_all_html(html: &str, _crate_name: &str) -> Result<Vec<ItemRef>> {
    let document = scraper::Html::parse_document(html);
    let mut items = Vec::new();

    // docs.rs all.html has sections with id like "structs", "enums", etc.
    // Each section contains a list of links to items
    let section_mappings: &[(&str, ItemKind)] = &[
        ("structs", ItemKind::Struct),
        ("enums", ItemKind::Enum),
        ("traits", ItemKind::Trait),
        ("functions", ItemKind::Function),
        ("macros", ItemKind::Macro),
        ("derive-macros", ItemKind::DeriveMacro),
        ("attribute-macros", ItemKind::AttrMacro),
        ("type-aliases", ItemKind::TypeAlias),
        ("type-definitions", ItemKind::TypeAlias),
        ("constants", ItemKind::Constant),
    ];

    for &(section_id, kind) in section_mappings {
        // Try to find the section by id
        let section_sel =
            scraper::Selector::parse(&format!("#{}", section_id)).unwrap();
        if let Some(section_el) = document.select(&section_sel).next() {
            // Find the containing section - walk up or look for sibling list
            // In docs.rs, the id is on an h3, and items are in a ul after it
            // We need to find the next sibling ul or the items within the same parent
            let parent = section_el.parent().and_then(|p| {
                scraper::ElementRef::wrap(p)
            });

            if let Some(parent_el) = parent {
                let a_sel = scraper::Selector::parse("a").unwrap();
                for link in parent_el.select(&a_sel) {
                    if let Some(href) = link.value().attr("href") {
                        let text = link.text().collect::<String>();
                        let name = text.trim().to_string();
                        if name.is_empty() {
                            continue;
                        }

                        // Detect module prefix from path segments
                        let module_prefix = extract_module_prefix(href);

                        items.push(ItemRef {
                            name,
                            kind,
                            path: href.trim_start_matches("./").to_string(),
                            module_prefix,
                        });
                    }
                }
            }
        }
    }

    // Fallback: try parsing by looking at link patterns in the HTML
    if items.is_empty() {
        items = parse_all_html_by_links(html);
    }

    Ok(items)
}

/// Fallback parser that looks for item links by URL pattern
fn parse_all_html_by_links(html: &str) -> Vec<ItemRef> {
    let document = scraper::Html::parse_document(html);
    let a_sel = scraper::Selector::parse("a[href]").unwrap();
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for link in document.select(&a_sel) {
        let href = match link.value().attr("href") {
            Some(h) => h,
            None => continue,
        };
        let text = link.text().collect::<String>();
        let name = text.trim().to_string();
        if name.is_empty() {
            continue;
        }

        let clean_href = href.trim_start_matches("./");
        // Get the filename part (after last slash, or the whole thing)
        let filename = clean_href.rsplit('/').next().unwrap_or(clean_href);
        let kind = if filename.starts_with("struct.") {
            Some(ItemKind::Struct)
        } else if filename.starts_with("enum.") {
            Some(ItemKind::Enum)
        } else if filename.starts_with("trait.") {
            Some(ItemKind::Trait)
        } else if filename.starts_with("fn.") {
            Some(ItemKind::Function)
        } else if filename.starts_with("macro.") {
            Some(ItemKind::Macro)
        } else if filename.starts_with("type.") {
            Some(ItemKind::TypeAlias)
        } else if filename.starts_with("constant.") {
            Some(ItemKind::Constant)
        } else if filename.starts_with("derive.") {
            Some(ItemKind::DeriveMacro)
        } else if filename.starts_with("attr.") {
            Some(ItemKind::AttrMacro)
        } else {
            None
        };

        if let Some(kind) = kind {
            let key = (name.clone(), clean_href.to_string());
            if seen.insert(key) {
                let module_prefix = extract_module_prefix(clean_href);
                items.push(ItemRef {
                    name,
                    kind,
                    path: clean_href.to_string(),
                    module_prefix,
                });
            }
        }
    }

    items
}

/// Extract module prefix from a path like "collections/struct.HashMap.html" -> Some("collections")
fn extract_module_prefix(href: &str) -> Option<String> {
    let clean = href.trim_start_matches("./");
    // If there's a directory component, that's the module prefix
    if let Some(slash_idx) = clean.rfind('/') {
        let prefix = &clean[..slash_idx];
        if !prefix.is_empty() {
            return Some(prefix.replace('/', "::"));
        }
    }
    None
}

/// Parse an item detail page HTML (pure, testable)
pub fn parse_item_html(html: &str, kind: ItemKind) -> Result<ItemDetail> {
    let document = scraper::Html::parse_document(html);

    // Extract signature from the item declaration
    let signature = extract_signature(&document, kind);

    // Extract first paragraph description
    let description = extract_description(&document);

    // Extract methods for structs, enums, and traits
    let methods = if matches!(kind, ItemKind::Struct | ItemKind::Enum | ItemKind::Trait) {
        extract_methods(&document)
    } else {
        Vec::new()
    };

    Ok(ItemDetail {
        signature,
        description,
        methods,
    })
}

/// Extract the main signature/declaration from an item page
fn extract_signature(document: &scraper::Html, _kind: ItemKind) -> String {
    // docs.rs puts the declaration in a <pre> inside .item-decl or .rust.item-decl
    let selectors = [
        ".item-decl pre",
        "pre.rust.item-decl",
        ".docblock.item-decl pre",
        "pre.rust",
    ];

    for sel_str in &selectors {
        if let Ok(sel) = scraper::Selector::parse(sel_str) {
            if let Some(el) = document.select(&sel).next() {
                let text = el.text().collect::<String>();
                let cleaned = clean_signature(&text);
                if !cleaned.is_empty() {
                    return cleaned;
                }
            }
        }
    }

    String::new()
}

/// Clean up a signature by collapsing whitespace and trimming
fn clean_signature(sig: &str) -> String {
    let lines: Vec<&str> = sig.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let joined = lines.join(" ");
    // Collapse multiple spaces
    let mut result = String::new();
    let mut prev_space = false;
    for ch in joined.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result.trim().to_string()
}

/// Extract the first paragraph description from the docblock
fn extract_description(document: &scraper::Html) -> String {
    let selectors = [
        ".docblock > p:first-child",
        ".docblock p",
        ".item-info + .docblock p",
    ];

    for sel_str in &selectors {
        if let Ok(sel) = scraper::Selector::parse(sel_str) {
            if let Some(el) = document.select(&sel).next() {
                let text = el.text().collect::<String>();
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    // Truncate long descriptions to first sentence or 120 chars
                    return truncate_description(&trimmed);
                }
            }
        }
    }

    String::new()
}

/// Truncate a description to a reasonable length
fn truncate_description(desc: &str) -> String {
    // Try to cut at first sentence
    if let Some(dot_idx) = desc.find(". ") {
        if dot_idx < 150 {
            return desc[..=dot_idx].to_string();
        }
    }
    if desc.len() <= 120 {
        return desc.to_string();
    }
    // Truncate at word boundary
    let truncated = &desc[..120];
    if let Some(space_idx) = truncated.rfind(' ') {
        format!("{}...", &truncated[..space_idx])
    } else {
        format!("{}...", truncated)
    }
}

/// Extract methods from a struct/enum/trait page
fn extract_methods(document: &scraper::Html) -> Vec<MethodDetail> {
    let mut methods = Vec::new();

    // docs.rs shows methods in .impl-items with each method in a section
    // Method signatures are in h4.code-header or similar
    let method_sel = scraper::Selector::parse(
        "details.toggle.method-toggle summary h4.code-header, \
         section.method h4.code-header, \
         .impl-items h4.code-header, \
         h4.code-header"
    );

    let method_sel = match method_sel {
        Ok(s) => s,
        Err(_) => return methods,
    };

    for el in document.select(&method_sel) {
        let text = el.text().collect::<String>();
        let sig = clean_signature(&text);

        // Extract method name from signature
        let name = extract_fn_name_from_sig(&sig);
        if name.is_empty() {
            continue;
        }

        // Try to get the description from the next sibling docblock
        let description = extract_method_description(el);

        methods.push(MethodDetail {
            name,
            signature: sig,
            description,
        });
    }

    // Deduplicate by name
    let mut seen = std::collections::HashSet::new();
    methods.retain(|m| seen.insert(m.name.clone()));

    methods
}

/// Extract function name from a signature like "pub fn foo(args) -> Ret"
fn extract_fn_name_from_sig(sig: &str) -> String {
    // Look for "fn name" pattern
    if let Some(fn_idx) = sig.find("fn ") {
        let rest = &sig[fn_idx + 3..];
        let name_end = rest
            .find(|c: char| c == '(' || c == '<' || c.is_whitespace())
            .unwrap_or(rest.len());
        let name = rest[..name_end].trim();
        if !name.is_empty() {
            return name.to_string();
        }
    }
    String::new()
}

/// Try to extract a method's description from sibling elements
fn extract_method_description(el: scraper::ElementRef) -> String {
    // Walk up to the parent details/section, then find .docblock p
    let mut current = el.parent();
    for _ in 0..5 {
        if let Some(node) = current {
            if let Some(el_ref) = scraper::ElementRef::wrap(node) {
                if let Ok(sel) = scraper::Selector::parse(".docblock p") {
                    if let Some(p) = el_ref.select(&sel).next() {
                        let text = p.text().collect::<String>();
                        let trimmed = text.trim().to_string();
                        if !trimmed.is_empty() {
                            return truncate_description(&trimmed);
                        }
                    }
                }
            }
            current = node.parent();
        } else {
            break;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_all_html_structs() {
        let html = r#"<html><body>
            <h3 id="structs">Structs</h3>
            <ul>
                <li><a href="struct.HashMap.html">HashMap</a></li>
                <li><a href="struct.BTreeMap.html">BTreeMap</a></li>
            </ul>
        </body></html>"#;

        // Use the fallback link-based parser since this minimal HTML
        // won't have the exact docs.rs DOM structure
        let items = parse_all_html(html, "std").unwrap();
        let struct_items: Vec<&ItemRef> = items.iter().filter(|i| i.kind == ItemKind::Struct).collect();
        assert!(struct_items.len() >= 2, "Expected at least 2 structs, got {}", struct_items.len());
        assert!(struct_items.iter().any(|i| i.name == "HashMap"));
        assert!(struct_items.iter().any(|i| i.name == "BTreeMap"));
    }

    #[test]
    fn test_parse_all_html_multiple_kinds() {
        let html = r#"<html><body>
            <a href="struct.Config.html">Config</a>
            <a href="trait.Builder.html">Builder</a>
            <a href="fn.run.html">run</a>
        </body></html>"#;

        let items = parse_all_html(html, "mycrate").unwrap();
        assert!(items.iter().any(|i| i.kind == ItemKind::Struct && i.name == "Config"));
        assert!(items.iter().any(|i| i.kind == ItemKind::Trait && i.name == "Builder"));
        assert!(items.iter().any(|i| i.kind == ItemKind::Function && i.name == "run"));
    }

    #[test]
    fn test_parse_all_html_submodule_item() {
        let html = r#"<html><body>
            <a href="collections/struct.Entry.html">Entry</a>
        </body></html>"#;

        let items = parse_all_html(html, "mycrate").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Entry");
        assert_eq!(items[0].module_prefix, Some("collections".to_string()));
    }

    #[test]
    fn test_parse_all_html_empty_page() {
        let html = r#"<html><body><p>No items here</p></body></html>"#;
        let items = parse_all_html(html, "empty").unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_parse_item_html_function() {
        let html = r#"<html><body>
            <div class="item-decl"><pre class="rust">pub fn run(config: Config) -&gt; Result&lt;()&gt;</pre></div>
            <div class="docblock"><p>Run the application with the given configuration.</p></div>
        </body></html>"#;

        let detail = parse_item_html(html, ItemKind::Function).unwrap();
        assert!(detail.signature.contains("pub fn run"), "sig was: {}", detail.signature);
        assert!(detail.description.contains("Run the application"));
        assert!(detail.methods.is_empty());
    }

    #[test]
    fn test_parse_item_html_struct_with_methods() {
        let html = r#"<html><body>
            <div class="item-decl"><pre class="rust">pub struct Builder { }</pre></div>
            <div class="docblock"><p>A builder for configuration.</p></div>
            <div class="impl-items">
                <h4 class="code-header">pub fn new() -&gt; Builder</h4>
                <div class="docblock"><p>Create a new builder.</p></div>
                <h4 class="code-header">pub fn build(self) -&gt; Config</h4>
                <div class="docblock"><p>Build the config.</p></div>
            </div>
        </body></html>"#;

        let detail = parse_item_html(html, ItemKind::Struct).unwrap();
        assert!(detail.signature.contains("pub struct Builder"));
        assert!(detail.methods.len() >= 2, "Expected at least 2 methods, got {}", detail.methods.len());
        assert!(detail.methods.iter().any(|m| m.name == "new"));
        assert!(detail.methods.iter().any(|m| m.name == "build"));
    }

    #[test]
    fn test_parse_item_html_missing_signature() {
        let html = r#"<html><body><p>Just some text</p></body></html>"#;
        let detail = parse_item_html(html, ItemKind::Function).unwrap();
        assert!(detail.signature.is_empty());
    }

    #[test]
    fn test_build_item_url() {
        assert_eq!(
            build_item_url("serde-json", "struct.Value.html"),
            "https://docs.rs/serde-json/latest/serde_json/struct.Value.html"
        );
        assert_eq!(
            build_item_url("tokio", "fn.spawn.html"),
            "https://docs.rs/tokio/latest/tokio/fn.spawn.html"
        );
    }

    #[test]
    fn test_build_all_url() {
        assert_eq!(
            build_all_url("serde-json"),
            "https://docs.rs/serde-json/latest/serde_json/all.html"
        );
    }

    #[test]
    fn test_extract_module_prefix() {
        assert_eq!(extract_module_prefix("struct.Foo.html"), None);
        assert_eq!(
            extract_module_prefix("collections/struct.Entry.html"),
            Some("collections".to_string())
        );
        assert_eq!(
            extract_module_prefix("io/net/struct.TcpStream.html"),
            Some("io::net".to_string())
        );
    }

    #[test]
    fn test_extract_fn_name_from_sig() {
        assert_eq!(extract_fn_name_from_sig("pub fn new() -> Self"), "new");
        assert_eq!(extract_fn_name_from_sig("pub fn build(self) -> Config"), "build");
        assert_eq!(extract_fn_name_from_sig("fn helper<T>(x: T)"), "helper");
        assert_eq!(extract_fn_name_from_sig("no function here"), "");
    }

    #[test]
    fn test_truncate_description() {
        assert_eq!(truncate_description("Short desc."), "Short desc.");
        assert_eq!(
            truncate_description("First sentence. Second sentence."),
            "First sentence."
        );
        let long = "A ".repeat(100);
        let result = truncate_description(&long);
        assert!(result.len() <= 130);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_clean_signature() {
        assert_eq!(
            clean_signature("  pub fn   new()  -> Self  "),
            "pub fn new() -> Self"
        );
        assert_eq!(
            clean_signature("pub struct\n    Foo {\n    bar: i32\n}"),
            "pub struct Foo { bar: i32 }"
        );
    }
}
