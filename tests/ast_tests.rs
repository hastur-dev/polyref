use polyref::ast::{extract_calls_from_source, CallType};

#[test]
fn test_extract_method_call() {
    let source = r#"
fn main() {
    let mut v = Vec::new();
    v.push(1);
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    let push_call = calls.iter().find(|c| c.method_name == "push").unwrap();
    assert_eq!(push_call.call_type, CallType::MethodCall);
    assert_eq!(push_call.receiver, "v");
    assert_eq!(push_call.arg_count, 1);
    assert_eq!(push_call.line_number, 4);
}

#[test]
fn test_extract_associated_call() {
    let source = r#"
fn main() {
    let v = Vec::new();
    let m = HashMap::with_capacity(10);
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    let new_call = calls.iter().find(|c| c.method_name == "new").unwrap();
    assert_eq!(new_call.call_type, CallType::AssociatedCall);
    assert_eq!(new_call.receiver, "Vec");
    assert_eq!(new_call.arg_count, 0);

    let cap_call = calls.iter().find(|c| c.method_name == "with_capacity").unwrap();
    assert_eq!(cap_call.call_type, CallType::AssociatedCall);
    assert_eq!(cap_call.receiver, "HashMap");
    assert_eq!(cap_call.arg_count, 1);
}

#[test]
fn test_extract_free_call() {
    let source = r#"
fn main() {
    drop(x);
    println!("hello");
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    let drop_call = calls.iter().find(|c| c.method_name == "drop").unwrap();
    assert_eq!(drop_call.call_type, CallType::FreeCall);
    assert_eq!(drop_call.receiver, "");
    assert_eq!(drop_call.arg_count, 1);
}

#[test]
fn test_chained_method_calls() {
    let source = r#"
fn main() {
    let result = items.iter().filter(|x| x > 0).collect::<Vec<_>>();
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    let method_names: Vec<&str> = calls.iter()
        .filter(|c| c.call_type == CallType::MethodCall)
        .map(|c| c.method_name.as_str())
        .collect();
    assert!(method_names.contains(&"iter"));
    assert!(method_names.contains(&"filter"));
    assert!(method_names.contains(&"collect"));
}

#[test]
fn test_nested_calls() {
    let source = r#"
fn main() {
    let x = foo(bar(baz(1)));
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    let free_calls: Vec<&str> = calls.iter()
        .filter(|c| c.call_type == CallType::FreeCall)
        .map(|c| c.method_name.as_str())
        .collect();
    assert!(free_calls.contains(&"foo"));
    assert!(free_calls.contains(&"bar"));
    assert!(free_calls.contains(&"baz"));
}

#[test]
fn test_multi_segment_associated_call() {
    let source = r#"
fn main() {
    let channel = tokio::sync::mpsc::channel(100);
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    let channel_call = calls.iter().find(|c| c.method_name == "channel").unwrap();
    assert_eq!(channel_call.call_type, CallType::AssociatedCall);
    assert_eq!(channel_call.receiver, "tokio::sync::mpsc");
    assert_eq!(channel_call.arg_count, 1);
}

#[test]
fn test_parse_error_returns_error() {
    let source = "fn main() { let x = ; }";
    let result = extract_calls_from_source(source);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("failed to parse"));
}

#[test]
fn test_empty_source() {
    let calls = extract_calls_from_source("").unwrap();
    assert!(calls.is_empty());
}

#[test]
fn test_no_calls() {
    let source = r#"
fn main() {
    let x = 1 + 2;
    let y: &str = "hello";
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    assert!(calls.is_empty());
}

#[test]
fn test_method_call_arg_count_multiple_args() {
    let source = r#"
fn main() {
    map.insert("key", "value");
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    let insert_call = calls.iter().find(|c| c.method_name == "insert").unwrap();
    assert_eq!(insert_call.arg_count, 2);
}

#[test]
fn test_self_receiver() {
    let source = r#"
struct Foo;
impl Foo {
    fn do_thing(&self) {
        self.bar(1, 2);
    }
    fn bar(&self, _a: i32, _b: i32) {}
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    let bar_call = calls.iter().find(|c| c.method_name == "bar").unwrap();
    assert_eq!(bar_call.call_type, CallType::MethodCall);
    assert_eq!(bar_call.receiver, "self");
    assert_eq!(bar_call.arg_count, 2);
}

#[test]
fn test_extract_chained_receiver_field_access() {
    let source = r#"
fn main() {
    self.items.push(1);
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    let push_call = calls.iter().find(|c| c.method_name == "push").unwrap();
    assert_eq!(push_call.call_type, CallType::MethodCall);
    assert_eq!(push_call.receiver, "self.items");
}

#[test]
fn test_calls_inside_closures() {
    let source = r#"
fn main() {
    let f = |x: i32| x.to_string();
}
"#;
    let calls = extract_calls_from_source(source).unwrap();
    let to_str = calls.iter().find(|c| c.method_name == "to_string").unwrap();
    assert_eq!(to_str.call_type, CallType::MethodCall);
    assert_eq!(to_str.receiver, "x");
}
