use tokio::runtime::Runtime;
use mylib::Color;

// BAD-1: Wrong constructor
let rt = Runtime::new_async().unwrap();

// BAD-2: Invented method
handle.wait_for_completion().await;

// BAD-3: Wrong associated fn
let set = JoinSet::create();

// BAD-4: Too many args to abort
handle.abort(true);

// BAD-5: Too few args to spawn
tokio::spawn();

// BAD-6: Wrong enum variant
let color = Color::Rojo;

// BAD-7: Invented module path
use tokio::tasks::spawn;

// BAD-8: Wrong method from wrong type
let s: String = "hello".parse_json();

// BAD-9: spawn_async (previously missed at 0.6 threshold)
let h = rt.spawn_async(async { 42 });

// BAD-10: get_size (previously missed)
let n = vec.get_size();

// BAD-11: Wrong struct field name
let t = config.time_out;

// BAD-12: Hallucinated crate-root function
let h = tokio::wait_all(handles);

// BAD-13: Builder method that doesn't exist
let rt = Builder::new_async().build().unwrap();
